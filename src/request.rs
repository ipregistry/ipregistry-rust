//! Request builders returned by [`Client`] lookup methods.
//!
//! Each request implements [`IntoFuture`], so it can be awaited directly or
//! refined first:
//!
//! ```no_run
//! # async fn example(client: &ipregistry::Client) -> Result<(), ipregistry::Error> {
//! let info = client
//!     .lookup("8.8.8.8".parse::<std::net::IpAddr>().unwrap())
//!     .hostname(true)
//!     .fields("location.country.name,security")
//!     .await?;
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::future::{Future, IntoFuture};
use std::net::IpAddr;
use std::pin::Pin;

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::Method;
use serde::Deserialize;

use crate::client::{Client, cache_key};
use crate::error::{ApiError, ApiErrorPayload, Result};
use crate::model::{IpInfo, RequesterIpInfo};

/// The boxed future produced when awaiting a request builder.
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

macro_rules! impl_lookup_params {
    ($ty:ty) => {
        impl $ty {
            /// Enables or disables reverse-DNS hostname resolution for the
            /// looked-up IP addresses. It is disabled by default.
            pub fn hostname(mut self, enabled: bool) -> Self {
                self.params
                    .insert("hostname".to_owned(), enabled.to_string());
                self
            }

            /// Restricts the response to the given fields, using Ipregistry's
            /// field selector syntax (for example
            /// `"location.country.name,security"`). This reduces payload size
            /// and, in some cases, credit usage. See
            /// <https://ipregistry.co/docs/filtering-selecting-fields> for the
            /// syntax. Excluded fields hold their default value (see the
            /// [model documentation](crate::model)).
            pub fn fields(mut self, expression: impl Into<String>) -> Self {
                self.params.insert("fields".to_owned(), expression.into());
                self
            }

            /// Sets an arbitrary query parameter. Use it for options not
            /// covered by a dedicated method.
            pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
                self.params.insert(name.into(), value.into());
                self
            }
        }
    };
}

/// A single-IP lookup request. Created with [`Client::lookup`]; await it (or
/// call [`send`](LookupRequest::send)) to execute it.
#[derive(Debug, Clone)]
#[must_use = "requests do nothing unless awaited or sent"]
pub struct LookupRequest {
    client: Client,
    ip: IpAddr,
    params: BTreeMap<String, String>,
}

impl_lookup_params!(LookupRequest);

impl LookupRequest {
    pub(crate) fn new(client: Client, ip: IpAddr) -> Self {
        Self {
            client,
            ip,
            params: BTreeMap::new(),
        }
    }

    /// Executes the request. Awaiting the request directly is equivalent.
    pub async fn send(self) -> Result<IpInfo> {
        let key = cache_key(&self.ip, &self.params);
        if let Some(hit) = self.client.cache_get(&key) {
            return Ok(hit);
        }

        let url = self.client.endpoint(&self.ip.to_string(), &self.params);
        let data = self.client.execute(Method::GET, url, None).await?;
        let info: IpInfo = serde_json::from_slice(&data)?;
        self.client.cache_set(&key, info.clone());
        Ok(info)
    }
}

impl IntoFuture for LookupRequest {
    type Output = Result<IpInfo>;
    type IntoFuture = BoxFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// An origin (requester) IP lookup request. Created with
/// [`Client::lookup_origin`]; await it (or call
/// [`send`](LookupOriginRequest::send)) to execute it.
#[derive(Debug, Clone)]
#[must_use = "requests do nothing unless awaited or sent"]
pub struct LookupOriginRequest {
    client: Client,
    params: BTreeMap<String, String>,
}

impl_lookup_params!(LookupOriginRequest);

impl LookupOriginRequest {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            params: BTreeMap::new(),
        }
    }

    /// Executes the request. Awaiting the request directly is equivalent.
    pub async fn send(self) -> Result<RequesterIpInfo> {
        let url = self.client.endpoint("", &self.params);
        let data = self.client.execute(Method::GET, url, None).await?;
        Ok(serde_json::from_slice(&data)?)
    }
}

impl IntoFuture for LookupOriginRequest {
    type Output = Result<RequesterIpInfo>;
    type IntoFuture = BoxFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A batch IP lookup request. Created with [`Client::lookup_batch`]; await it
/// (or call [`send`](LookupBatchRequest::send)) to execute it.
#[derive(Debug, Clone)]
#[must_use = "requests do nothing unless awaited or sent"]
pub struct LookupBatchRequest {
    client: Client,
    ips: Vec<IpAddr>,
    params: BTreeMap<String, String>,
}

impl_lookup_params!(LookupBatchRequest);

impl LookupBatchRequest {
    pub(crate) fn new(client: Client, ips: Vec<IpAddr>) -> Self {
        Self {
            client,
            ips,
            params: BTreeMap::new(),
        }
    }

    /// Executes the request. Awaiting the request directly is equivalent.
    ///
    /// Results preserve the order of the input; each entry independently
    /// succeeds or fails. An `Err` from this method indicates the whole
    /// request failed.
    pub async fn send(self) -> Result<Vec<Result<IpInfo, ApiError>>> {
        let Self {
            client,
            ips,
            params,
        } = self;

        // Serve cache hits locally and only request the misses.
        let mut cached: Vec<Option<IpInfo>> = Vec::with_capacity(ips.len());
        let mut misses: Vec<IpAddr> = Vec::new();
        for ip in &ips {
            match client.cache_get(&cache_key(ip, &params)) {
                Some(info) => cached.push(Some(info)),
                None => {
                    cached.push(None);
                    misses.push(*ip);
                }
            }
        }

        let fresh = resolve_misses(&client, misses, &params).await?;
        let mut fresh = fresh.into_iter();

        let mut results = Vec::with_capacity(ips.len());
        for (ip, hit) in ips.iter().zip(cached) {
            if let Some(info) = hit {
                results.push(Ok(info));
                continue;
            }
            match fresh.next() {
                Some(Ok(info)) => {
                    client.cache_set(&cache_key(ip, &params), info.clone());
                    results.push(Ok(info));
                }
                Some(Err(err)) => results.push(Err(err)),
                // Defensive: the API returned fewer results than requested.
                None => results.push(Err(ApiError {
                    code: None,
                    message: "missing result for requested IP address".to_owned(),
                    resolution: String::new(),
                    status: None,
                })),
            }
        }
        Ok(results)
    }
}

impl IntoFuture for LookupBatchRequest {
    type Output = Result<Vec<Result<IpInfo, ApiError>>>;
    type IntoFuture = BoxFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// Fetches fresh data for the cache-missed IP addresses. Sends a single
/// request when the addresses fit within the API's per-request limit, and
/// otherwise splits them into chunks dispatched with bounded concurrency.
/// The returned results preserve the order of `misses`. If any chunk fails,
/// the first error is returned and the remaining in-flight requests are
/// cancelled.
async fn resolve_misses(
    client: &Client,
    misses: Vec<IpAddr>,
    params: &BTreeMap<String, String>,
) -> Result<Vec<Result<IpInfo, ApiError>>> {
    if misses.is_empty() {
        return Ok(Vec::new());
    }
    if misses.len() <= client.inner.max_batch_size {
        return batch_request(client, &misses, params).await;
    }

    let chunk_results: Vec<Vec<Result<IpInfo, ApiError>>> = stream::iter(
        misses
            .chunks(client.inner.max_batch_size)
            .map(|chunk| {
                let client = client.clone();
                let chunk = chunk.to_vec();
                let params = params.clone();
                async move { batch_request(&client, &chunk, &params).await }
            })
            // Collect the futures eagerly so the borrow of `misses` ends here.
            .collect::<Vec<_>>(),
    )
    .buffered(client.inner.batch_concurrency)
    .try_collect()
    .await?;

    Ok(chunk_results.into_iter().flatten().collect())
}

/// Performs a single POST batch request for the given addresses.
async fn batch_request(
    client: &Client,
    ips: &[IpAddr],
    params: &BTreeMap<String, String>,
) -> Result<Vec<Result<IpInfo, ApiError>>> {
    let body = serde_json::to_vec(&ips.iter().map(ToString::to_string).collect::<Vec<_>>())?;
    let url = client.endpoint("", params);
    let data = client.execute(Method::POST, url, Some(body)).await?;
    let envelope: BatchEnvelope<IpInfo> = serde_json::from_slice(&data)?;
    Ok(envelope.into_results())
}

/// The `{"results": [...]}` envelope wrapping batch responses, where each
/// element is either a payload or an error object.
#[derive(Debug, Deserialize)]
pub(crate) struct BatchEnvelope<T> {
    #[serde(default = "Vec::new")]
    results: Vec<BatchEntry<T>>,
}

impl<T> BatchEnvelope<T> {
    pub(crate) fn into_results(self) -> Vec<Result<T, ApiError>> {
        self.results
            .into_iter()
            .map(BatchEntry::into_result)
            .collect()
    }
}

/// One element of a batch response: an error object (detected by the presence
/// of a `code` field) or a successfully resolved payload.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BatchEntry<T> {
    // Tried first: only matches objects carrying a string `code` field, which
    // payload objects never have at the top level.
    Err(ApiErrorPayload),
    Ok(T),
}

impl<T> BatchEntry<T> {
    fn into_result(self) -> Result<T, ApiError> {
        match self {
            Self::Err(payload) => Err(payload.into_api_error(None)),
            Self::Ok(value) => Ok(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn batch_envelope_maps_entries_to_results() {
        let json = r#"{
            "results": [
                {"ip": "8.8.8.8", "type": "IPv4"},
                {"code": "RESERVED_IP_ADDRESS", "message": "reserved", "resolution": "use a public IP"},
                {"ip": "2001:4860:4860::8888", "type": "IPv6"}
            ]
        }"#;
        let envelope: BatchEnvelope<IpInfo> = serde_json::from_str(json).unwrap();
        let results = envelope.into_results();
        assert_eq!(results.len(), 3);

        let first = results[0].as_ref().unwrap();
        assert_eq!(first.ip, Some("8.8.8.8".parse().unwrap()));

        let err = results[1].as_ref().unwrap_err();
        assert_eq!(err.code, Some(ErrorCode::ReservedIpAddress));
        assert_eq!(err.message, "reserved");
        assert_eq!(err.status, None);

        let third = results[2].as_ref().unwrap();
        assert_eq!(third.ip, Some("2001:4860:4860::8888".parse().unwrap()));
    }

    #[test]
    fn batch_envelope_tolerates_missing_results() {
        let envelope: BatchEnvelope<IpInfo> = serde_json::from_str("{}").unwrap();
        assert!(envelope.into_results().is_empty());
    }
}

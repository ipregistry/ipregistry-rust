//! The Ipregistry API client.

use std::collections::BTreeMap;
use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Method;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, RETRY_AFTER, USER_AGENT};
use url::Url;

use crate::cache::Cache;
use crate::error::{ApiError, Error, Result, parse_api_error};
use crate::model::{IpInfo, UserAgent};
use crate::request::{BatchEnvelope, LookupBatchRequest, LookupOriginRequest, LookupRequest};

/// The base URL of the Ipregistry API used unless overridden with
/// [`ClientBuilder::base_url`].
pub const DEFAULT_BASE_URL: &str = "https://api.ipregistry.co";

/// The maximum number of IP addresses Ipregistry accepts in a single batch
/// request. [`Client::lookup_batch`] transparently splits larger inputs into
/// several requests so callers never have to.
pub const MAX_BATCH_SIZE: usize = 1024;

/// The default per-request timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// The default maximum number of automatic retries performed in addition to
/// the initial attempt.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// The default base backoff between retries.
pub const DEFAULT_RETRY_INTERVAL: Duration = Duration::from_secs(1);

/// The default number of batch sub-requests dispatched concurrently when a
/// batch lookup is split into chunks.
pub const DEFAULT_BATCH_CONCURRENCY: usize = 4;

/// The default value of the `User-Agent` header sent with requests.
const CLIENT_USER_AGENT: &str = concat!("IpregistryClient/Rust/", env!("CARGO_PKG_VERSION"));

/// A client for the Ipregistry API. Create one with [`Client::new`] or
/// [`Client::builder`].
///
/// A `Client` is cheap to clone (clones share the same connection pool and
/// cache) and safe to share across threads and tasks — there is no need to
/// wrap it in [`Arc`] or a mutex.
///
/// ```no_run
/// # async fn example() -> Result<(), ipregistry::Error> {
/// use ipregistry::Client;
///
/// let client = Client::new("YOUR_API_KEY");
/// let info = client.lookup("54.85.132.205".parse::<std::net::IpAddr>().unwrap()).await?;
/// println!("{}", info.location.country.name);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    api_key: String,
    base_url: Url,
    http: reqwest::Client,
    cache: Option<Arc<dyn Cache>>,
    max_retries: u32,
    retry_interval: Duration,
    retry_on_server_error: bool,
    retry_on_too_many_requests: bool,
    pub(crate) max_batch_size: usize,
    pub(crate) batch_concurrency: usize,
    user_agent: String,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("api_key", &"<redacted>")
            .field("base_url", &self.inner.base_url.as_str())
            .field("cache", &self.inner.cache.is_some())
            .field("max_retries", &self.inner.max_retries)
            .field("retry_interval", &self.inner.retry_interval)
            .field("retry_on_server_error", &self.inner.retry_on_server_error)
            .field(
                "retry_on_too_many_requests",
                &self.inner.retry_on_too_many_requests,
            )
            .field("max_batch_size", &self.inner.max_batch_size)
            .field("batch_concurrency", &self.inner.batch_concurrency)
            .field("user_agent", &self.inner.user_agent)
            .finish()
    }
}

impl Client {
    /// Creates a client authenticating with the given API key and default
    /// settings. You can obtain a key, along with a generous free tier, at
    /// <https://ipregistry.co>.
    ///
    /// By default the client uses a 15-second timeout, retries transient
    /// failures up to three times, and performs no caching. Use
    /// [`Client::builder`] to customize the behavior.
    ///
    /// # Panics
    ///
    /// Panics if the TLS backend cannot be initialized. Use
    /// [`Client::builder`] to handle that failure instead of panicking.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder(api_key)
            .build()
            .expect("failed to build ipregistry client")
    }

    /// Returns a builder to customize the client configuration.
    pub fn builder(api_key: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    /// Looks up the data associated with the given IP address. To look up the
    /// requester's own IP, use [`Client::lookup_origin`] instead.
    ///
    /// The returned request can be awaited directly, or refined first:
    ///
    /// ```no_run
    /// # async fn example(client: &ipregistry::Client) -> Result<(), ipregistry::Error> {
    /// use std::net::IpAddr;
    ///
    /// let ip: IpAddr = "8.8.8.8".parse().unwrap();
    /// let info = client.lookup(ip).await?;
    /// let info = client.lookup(ip).hostname(true).fields("location,security").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// When a cache is configured, a hit is returned without contacting the
    /// API.
    pub fn lookup(&self, ip: impl Into<IpAddr>) -> LookupRequest {
        LookupRequest::new(self.clone(), ip.into())
    }

    /// Looks up the data associated with the IP address the request
    /// originates from. The response is enriched with parsed User-Agent data.
    /// Origin lookups are never cached, because the requester IP is only
    /// known from the response.
    pub fn lookup_origin(&self) -> LookupOriginRequest {
        LookupOriginRequest::new(self.clone())
    }

    /// Looks up several IP addresses in a single request. Results preserve
    /// the order of the input, and each entry independently succeeds or fails
    /// (for example on a reserved address) as a `Result<IpInfo, ApiError>`.
    /// An `Err` from awaiting the request itself indicates the whole request
    /// failed (for example authentication or a network error).
    ///
    /// Entries already present in the cache are served locally; only the
    /// remainder are requested from the API. Inputs larger than the API's
    /// per-request limit ([`MAX_BATCH_SIZE`]) are transparently split into
    /// several requests dispatched with bounded concurrency (see
    /// [`ClientBuilder::batch_concurrency`]).
    ///
    /// ```no_run
    /// # async fn example(client: &ipregistry::Client) -> Result<(), ipregistry::Error> {
    /// use std::net::IpAddr;
    ///
    /// let ips: Vec<IpAddr> = ["8.8.8.8", "1.1.1.1"].iter().map(|s| s.parse().unwrap()).collect();
    /// for entry in client.lookup_batch(ips).await? {
    ///     match entry {
    ///         Ok(info) => println!("{}", info.location.country.name),
    ///         Err(err) => eprintln!("entry failed: {err}"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn lookup_batch<I>(&self, ips: I) -> LookupBatchRequest
    where
        I: IntoIterator,
        I::Item: Into<IpAddr>,
    {
        LookupBatchRequest::new(self.clone(), ips.into_iter().map(Into::into).collect())
    }

    /// Parses one or more raw User-Agent strings (such as the `User-Agent`
    /// header of an incoming HTTP request) into structured data. Results
    /// preserve the order of the input, and each entry independently succeeds
    /// or fails.
    pub async fn parse_user_agents<I>(
        &self,
        user_agents: I,
    ) -> Result<Vec<Result<UserAgent, ApiError>>>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let agents: Vec<String> = user_agents.into_iter().map(Into::into).collect();
        let body = serde_json::to_vec(&agents)?;
        let url = self.endpoint("user_agent", &BTreeMap::new());
        let data = self.execute(Method::POST, url, Some(body)).await?;
        let envelope: BatchEnvelope<UserAgent> = serde_json::from_slice(&data)?;
        Ok(envelope.into_results())
    }

    /// Returns the cache used by the client, when one is configured with
    /// [`ClientBuilder::cache`].
    pub fn cache(&self) -> Option<&dyn Cache> {
        self.inner.cache.as_deref()
    }

    /// Returns the cached value for `key`, when a cache is configured.
    pub(crate) fn cache_get(&self, key: &str) -> Option<IpInfo> {
        self.inner.cache.as_ref()?.get(key)
    }

    /// Stores `value` in the cache, when one is configured.
    pub(crate) fn cache_set(&self, key: &str, value: IpInfo) {
        if let Some(cache) = &self.inner.cache {
            cache.set(key, value);
        }
    }

    /// Builds the request URL for the given endpoint segment. An empty
    /// segment targets the origin (requester) endpoint.
    pub(crate) fn endpoint(&self, segment: &str, params: &BTreeMap<String, String>) -> Url {
        let mut url = self.inner.base_url.clone();
        {
            let mut path = url
                .path_segments_mut()
                .expect("base URL validated at build time");
            path.pop_if_empty().push(segment);
        }
        if !params.is_empty() {
            url.query_pairs_mut().extend_pairs(params.iter());
        }
        url
    }

    /// Performs an HTTP request with automatic retries and returns the raw
    /// successful response body. Non-2xx responses are converted to
    /// [`Error::Api`]; transport failures to [`Error::Transport`].
    pub(crate) async fn execute(
        &self,
        method: Method,
        url: Url,
        body: Option<Vec<u8>>,
    ) -> Result<Vec<u8>> {
        let mut attempt: u32 = 0;
        loop {
            let mut request = self
                .inner
                .http
                .request(method.clone(), url.clone())
                .header(AUTHORIZATION, format!("ApiKey {}", self.inner.api_key))
                .header(USER_AGENT, &self.inner.user_agent)
                .header(ACCEPT, "application/json");
            if let Some(body) = &body {
                request = request
                    .header(CONTENT_TYPE, "application/json")
                    .body(body.clone());
            }

            let response = match request.send().await {
                Ok(response) => response,
                Err(err) => {
                    // Transport errors are retried up to max_retries regardless
                    // of the retry-on-status settings.
                    if attempt < self.inner.max_retries {
                        self.backoff(attempt, None).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::Transport(err));
                }
            };

            let status = response.status();
            let retry_after = parse_retry_after(response.headers());
            let data = response.bytes().await;

            if status.is_success() {
                match data {
                    Ok(data) => return Ok(data.into()),
                    Err(err) => {
                        if attempt < self.inner.max_retries {
                            self.backoff(attempt, None).await;
                            attempt += 1;
                            continue;
                        }
                        return Err(Error::Transport(err));
                    }
                }
            }

            if self.should_retry_status(status.as_u16()) && attempt < self.inner.max_retries {
                self.backoff(attempt, retry_after).await;
                attempt += 1;
                continue;
            }

            let data = data.unwrap_or_default();
            return Err(Error::Api(parse_api_error(&data, status.as_u16())));
        }
    }

    /// Reports whether a non-2xx status is eligible for retry given the
    /// client's configuration.
    fn should_retry_status(&self, status: u16) -> bool {
        if status == 429 {
            return self.inner.retry_on_too_many_requests;
        }
        if (500..600).contains(&status) {
            return self.inner.retry_on_server_error;
        }
        false
    }

    /// Waits before the next retry attempt, honoring an explicit `Retry-After`
    /// duration when positive and otherwise using exponential backoff.
    async fn backoff(&self, attempt: u32, retry_after: Option<Duration>) {
        let delay = match retry_after {
            Some(delay) if !delay.is_zero() => delay,
            _ => self
                .inner
                .retry_interval
                .saturating_mul(1u32 << attempt.min(30)),
        };
        tokio::time::sleep(delay).await;
    }
}

/// Parses a `Retry-After` header expressed as an integer number of seconds.
/// Returns `None` when the header is absent or not a valid non-negative
/// integer (the HTTP-date form is not supported).
fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?;
    let seconds: u64 = value.trim().parse().ok()?;
    Some(Duration::from_secs(seconds))
}

/// Derives a deterministic cache key from an IP address and its query
/// parameters. Parameters are iterated in sorted order, so the key is stable
/// regardless of the order options were applied in.
pub(crate) fn cache_key(ip: &IpAddr, params: &BTreeMap<String, String>) -> String {
    if params.is_empty() {
        return ip.to_string();
    }
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params.iter())
        .finish();
    format!("{ip};{query}")
}

/// Builds a [`Client`]. Create one with [`Client::builder`].
///
/// ```no_run
/// use std::time::Duration;
/// use ipregistry::{Client, InMemoryCache};
///
/// let client = Client::builder("YOUR_API_KEY")
///     .timeout(Duration::from_secs(5))
///     .cache(InMemoryCache::new())
///     .retry_on_too_many_requests(true)
///     .build()
///     .unwrap();
/// ```
#[must_use = "call `build()` to create the client"]
pub struct ClientBuilder {
    api_key: String,
    base_url: String,
    http: Option<reqwest::Client>,
    cache: Option<Arc<dyn Cache>>,
    timeout: Duration,
    max_retries: u32,
    retry_interval: Duration,
    retry_on_server_error: bool,
    retry_on_too_many_requests: bool,
    max_batch_size: usize,
    batch_concurrency: usize,
    user_agent: String,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &"<redacted>")
            .field("base_url", &self.base_url)
            .field("cache", &self.cache.is_some())
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("retry_interval", &self.retry_interval)
            .field("retry_on_server_error", &self.retry_on_server_error)
            .field(
                "retry_on_too_many_requests",
                &self.retry_on_too_many_requests,
            )
            .field("max_batch_size", &self.max_batch_size)
            .field("batch_concurrency", &self.batch_concurrency)
            .field("user_agent", &self.user_agent)
            .finish()
    }
}

impl ClientBuilder {
    fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_owned(),
            http: None,
            cache: None,
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_interval: DEFAULT_RETRY_INTERVAL,
            retry_on_server_error: true,
            retry_on_too_many_requests: false,
            max_batch_size: MAX_BATCH_SIZE,
            batch_concurrency: DEFAULT_BATCH_CONCURRENCY,
            user_agent: CLIENT_USER_AGENT.to_owned(),
        }
    }

    /// Overrides the API base URL. This is mainly useful for testing or
    /// pointing at a private deployment.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Supplies a custom [`reqwest::Client`], giving full control over
    /// connection pooling, proxying, TLS, and instrumentation. When set, the
    /// supplied client's own timeout applies and [`ClientBuilder::timeout`]
    /// is ignored.
    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.http = Some(http_client);
        self
    }

    /// Enables response caching using the supplied [`Cache`]. By default no
    /// cache is used, so that data is never stale.
    ///
    /// Pass an `Arc<C>` to keep a handle to the cache after handing it to the
    /// client (see the [`Cache`] documentation for an example).
    pub fn cache(mut self, cache: impl Cache + 'static) -> Self {
        self.cache = Some(Arc::new(cache));
        self
    }

    /// Sets the per-request timeout applied to the default HTTP client.
    /// Defaults to 15 seconds. It is ignored when a custom client is provided
    /// with [`ClientBuilder::http_client`].
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the maximum number of automatic retries performed in addition to
    /// the initial attempt. Set to `0` to disable retries. Defaults to 3.
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// Sets the base backoff between retries. Successive retries use an
    /// exponentially increasing delay (`interval * 2^attempt`). When a 429
    /// response carries a `Retry-After` header, that value takes precedence.
    /// Defaults to 1 second. A zero duration leaves the default.
    pub fn retry_interval(mut self, interval: Duration) -> Self {
        if !interval.is_zero() {
            self.retry_interval = interval;
        }
        self
    }

    /// Controls whether 5xx responses are retried. Defaults to `true`.
    pub fn retry_on_server_error(mut self, enabled: bool) -> Self {
        self.retry_on_server_error = enabled;
        self
    }

    /// Controls whether *429 Too Many Requests* responses are retried,
    /// honoring the `Retry-After` header when present. Ipregistry does not
    /// rate limit by default (it is opt-in per API key), so this defaults to
    /// `false`.
    pub fn retry_on_too_many_requests(mut self, enabled: bool) -> Self {
        self.retry_on_too_many_requests = enabled;
        self
    }

    /// Sets the maximum number of IP addresses sent in a single batch
    /// request. [`Client::lookup_batch`] splits larger inputs into this many
    /// addresses per request. Values are clamped to `1..=`[`MAX_BATCH_SIZE`]
    /// (the API limit); `0` leaves the default.
    pub fn max_batch_size(mut self, n: usize) -> Self {
        if n > 0 {
            self.max_batch_size = n.min(MAX_BATCH_SIZE);
        }
        self
    }

    /// Sets how many batch sub-requests [`Client::lookup_batch`] dispatches
    /// concurrently when an input is large enough to be split into chunks.
    /// Defaults to 4; `0` leaves the default. Set it to `1` for strictly
    /// sequential dispatch, which is gentler on a rate-limited API key.
    pub fn batch_concurrency(mut self, n: usize) -> Self {
        if n > 0 {
            self.batch_concurrency = n;
        }
        self
    }

    /// Overrides the `User-Agent` header sent with requests. An empty value
    /// leaves the default.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        let user_agent = user_agent.into();
        if !user_agent.is_empty() {
            self.user_agent = user_agent;
        }
        self
    }

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] when the base URL cannot be parsed, and
    /// [`Error::Transport`] when the underlying HTTP client cannot be
    /// initialized (for example when the TLS backend fails to load).
    pub fn build(self) -> Result<Client> {
        let base_url = Url::parse(self.base_url.trim_end_matches('/'))
            .map_err(|err| Error::Config(format!("invalid base URL: {err}")))?;
        if base_url.cannot_be_a_base() {
            return Err(Error::Config(format!(
                "invalid base URL: {} cannot be a base",
                self.base_url
            )));
        }

        let http = match self.http {
            Some(http) => http,
            None => reqwest::Client::builder()
                .timeout(self.timeout)
                .build()
                .map_err(Error::Transport)?,
        };

        Ok(Client {
            inner: Arc::new(ClientInner {
                api_key: self.api_key,
                base_url,
                http,
                cache: self.cache,
                max_retries: self.max_retries,
                retry_interval: self.retry_interval,
                retry_on_server_error: self.retry_on_server_error,
                retry_on_too_many_requests: self.retry_on_too_many_requests,
                max_batch_size: self.max_batch_size,
                batch_concurrency: self.batch_concurrency,
                user_agent: self.user_agent,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn cache_key_without_params_is_the_ip() {
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        assert_eq!(cache_key(&ip, &BTreeMap::new()), "8.8.8.8");
    }

    #[test]
    fn cache_key_is_deterministic_and_order_independent() {
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        let a = params(&[("hostname", "true"), ("fields", "location")]);
        let b = params(&[("fields", "location"), ("hostname", "true")]);
        assert_eq!(cache_key(&ip, &a), cache_key(&ip, &b));
        assert_eq!(cache_key(&ip, &a), "8.8.8.8;fields=location&hostname=true");
    }

    #[test]
    fn endpoint_builds_expected_urls() {
        let client = Client::builder("k")
            .base_url("https://example.test/sub/")
            .build()
            .unwrap();
        assert_eq!(
            client.endpoint("8.8.8.8", &BTreeMap::new()).as_str(),
            "https://example.test/sub/8.8.8.8"
        );
        assert_eq!(
            client.endpoint("", &BTreeMap::new()).as_str(),
            "https://example.test/sub/"
        );
        assert_eq!(
            client
                .endpoint("8.8.8.8", &params(&[("hostname", "true")]))
                .as_str(),
            "https://example.test/sub/8.8.8.8?hostname=true"
        );
    }

    #[test]
    fn build_rejects_invalid_base_url() {
        assert!(matches!(
            Client::builder("k").base_url("not a url").build(),
            Err(Error::Config(_))
        ));
    }

    #[test]
    fn batch_settings_are_clamped() {
        let client = Client::builder("k")
            .max_batch_size(50_000)
            .batch_concurrency(0)
            .build()
            .unwrap();
        assert_eq!(client.inner.max_batch_size, MAX_BATCH_SIZE);
        assert_eq!(client.inner.batch_concurrency, DEFAULT_BATCH_CONCURRENCY);

        let client = Client::builder("k").max_batch_size(0).build().unwrap();
        assert_eq!(client.inner.max_batch_size, MAX_BATCH_SIZE);
    }

    #[test]
    fn parse_retry_after_header() {
        let mut headers = HeaderMap::new();
        assert_eq!(parse_retry_after(&headers), None);

        headers.insert(RETRY_AFTER, "7".parse().unwrap());
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(7)));

        headers.insert(RETRY_AFTER, "-1".parse().unwrap());
        assert_eq!(parse_retry_after(&headers), None);

        headers.insert(
            RETRY_AFTER,
            "Wed, 21 Oct 2015 07:28:00 GMT".parse().unwrap(),
        );
        assert_eq!(parse_retry_after(&headers), None);
    }

    #[test]
    fn debug_redacts_api_key() {
        let client = Client::new("super-secret");
        let debug = format!("{client:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("<redacted>"));
    }
}

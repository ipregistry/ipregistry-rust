//! Error types returned by the client.

use std::fmt;

/// A convenient alias for results produced by this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The error type returned by all client operations.
///
/// Match on it to distinguish failures reported by the Ipregistry API from
/// failures that happened on the client side:
///
/// ```no_run
/// # async fn example() {
/// use ipregistry::{Client, Error, ErrorCode};
///
/// let client = Client::new("YOUR_API_KEY");
/// match client.lookup("8.8.8.8".parse::<std::net::IpAddr>().unwrap()).await {
///     Ok(info) => println!("{:?}", info.location.country.name),
///     Err(Error::Api(err)) if err.code == Some(ErrorCode::InsufficientCredits) => {
///         // handle exhausted credits
///     }
///     Err(Error::Transport(err)) => {
///         // handle network error, timeout, ...
///     }
///     Err(err) => eprintln!("{err}"),
/// }
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The Ipregistry API reported a failure, such as an invalid IP address,
    /// an exhausted credit balance, or throttling.
    #[error(transparent)]
    Api(#[from] ApiError),

    /// The request could not be sent or the response could not be read: a
    /// network error, a timeout, or a TLS failure. The underlying cause is
    /// available through [`std::error::Error::source`].
    #[error("ipregistry: transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// A successful response could not be decoded as the expected JSON shape.
    #[error("ipregistry: failed to decode response: {0}")]
    Decode(#[from] serde_json::Error),

    /// The client was misconfigured, for example with a base URL that cannot
    /// be parsed.
    #[error("ipregistry: invalid configuration: {0}")]
    Config(String),
}

impl Error {
    /// Returns the underlying [`ApiError`] when the API reported the failure.
    pub fn as_api(&self) -> Option<&ApiError> {
        match self {
            Error::Api(err) => Some(err),
            _ => None,
        }
    }

    /// Returns the typed [`ErrorCode`] when the API reported the failure with
    /// a recognizable code.
    pub fn code(&self) -> Option<&ErrorCode> {
        self.as_api().and_then(|err| err.code.as_ref())
    }
}

/// A failure reported by the Ipregistry API, such as an invalid IP address, an
/// exhausted credit balance, or throttling.
///
/// In batch lookups, an `ApiError` may also describe the failure of a single
/// entry rather than the whole request; per-entry errors carry no
/// [`status`](ApiError::status).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub struct ApiError {
    /// The error code returned by the API, or `None` when the response did not
    /// carry a recognizable error payload. Unrecognized codes are preserved in
    /// [`ErrorCode::Other`].
    pub code: Option<ErrorCode>,
    /// A human-readable description of the error.
    pub message: String,
    /// A suggestion on how to resolve the error, when available.
    pub resolution: String,
    /// The HTTP status of the failed request, or `None` for per-entry errors
    /// in batch responses.
    pub status: Option<u16>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ipregistry: ")?;
        if self.message.is_empty() {
            f.write_str("API error")?;
        } else {
            f.write_str(&self.message)?;
        }
        if let Some(code) = &self.code {
            write!(f, " ({code})")?;
        }
        if !self.resolution.is_empty() {
            write!(f, ": {}", self.resolution)?;
        }
        Ok(())
    }
}

/// A strongly typed Ipregistry API error code. It lets callers branch on error
/// conditions without matching on raw strings. See
/// <https://ipregistry.co/docs/errors> for the authoritative list.
///
/// Codes not known to this version of the crate are preserved verbatim in
/// [`ErrorCode::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// The request is malformed.
    BadRequest,
    /// The API key exists but is disabled.
    DisabledApiKey,
    /// Lookups for this IP address are not allowed for the API key.
    ForbiddenIp,
    /// Requests from this origin are not allowed for the API key.
    ForbiddenOrigin,
    /// Requests from this IP address or origin are not allowed.
    ForbiddenIpOrigin,
    /// The API encountered an internal error.
    Internal,
    /// The account has no credits left.
    InsufficientCredits,
    /// The API key is not valid.
    InvalidApiKey,
    /// The requested Autonomous System Number is malformed.
    InvalidAsn,
    /// The `fields` selection expression is malformed.
    InvalidFilterSyntax,
    /// The requested IP address is malformed.
    InvalidIpAddress,
    /// No API key was supplied.
    MissingApiKey,
    /// The requested Autonomous System Number is reserved.
    ReservedAsn,
    /// The requested IP address is reserved (private, loopback, ...).
    ReservedIpAddress,
    /// The batch request contains more ASNs than allowed.
    TooManyAsns,
    /// The batch request contains more IP addresses than allowed.
    TooManyIps,
    /// The API key is rate limited and the limit was exceeded.
    TooManyRequests,
    /// The request contains more User-Agent strings than allowed.
    TooManyUserAgents,
    /// The requested Autonomous System Number is unknown.
    UnknownAsn,
    /// An error code this version of the crate does not know about, preserved
    /// verbatim.
    Other(String),
}

impl ErrorCode {
    /// Parses a raw API error code, mapping unrecognized codes to
    /// [`ErrorCode::Other`].
    pub fn from_raw(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "BAD_REQUEST" => Self::BadRequest,
            "DISABLED_API_KEY" => Self::DisabledApiKey,
            "FORBIDDEN_IP" => Self::ForbiddenIp,
            "FORBIDDEN_ORIGIN" => Self::ForbiddenOrigin,
            "FORBIDDEN_IP_ORIGIN" => Self::ForbiddenIpOrigin,
            "INTERNAL" => Self::Internal,
            "INSUFFICIENT_CREDITS" => Self::InsufficientCredits,
            "INVALID_API_KEY" => Self::InvalidApiKey,
            "INVALID_ASN" => Self::InvalidAsn,
            "INVALID_FILTER_SYNTAX" => Self::InvalidFilterSyntax,
            "INVALID_IP_ADDRESS" => Self::InvalidIpAddress,
            "MISSING_API_KEY" => Self::MissingApiKey,
            "RESERVED_ASN" => Self::ReservedAsn,
            "RESERVED_IP_ADDRESS" => Self::ReservedIpAddress,
            "TOO_MANY_ASNS" => Self::TooManyAsns,
            "TOO_MANY_IPS" => Self::TooManyIps,
            "TOO_MANY_REQUESTS" => Self::TooManyRequests,
            "TOO_MANY_USER_AGENTS" => Self::TooManyUserAgents,
            "UNKNOWN_ASN" => Self::UnknownAsn,
            _ => Self::Other(raw.to_owned()),
        }
    }

    /// Returns the raw form of the code, as returned by the API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::BadRequest => "BAD_REQUEST",
            Self::DisabledApiKey => "DISABLED_API_KEY",
            Self::ForbiddenIp => "FORBIDDEN_IP",
            Self::ForbiddenOrigin => "FORBIDDEN_ORIGIN",
            Self::ForbiddenIpOrigin => "FORBIDDEN_IP_ORIGIN",
            Self::Internal => "INTERNAL",
            Self::InsufficientCredits => "INSUFFICIENT_CREDITS",
            Self::InvalidApiKey => "INVALID_API_KEY",
            Self::InvalidAsn => "INVALID_ASN",
            Self::InvalidFilterSyntax => "INVALID_FILTER_SYNTAX",
            Self::InvalidIpAddress => "INVALID_IP_ADDRESS",
            Self::MissingApiKey => "MISSING_API_KEY",
            Self::ReservedAsn => "RESERVED_ASN",
            Self::ReservedIpAddress => "RESERVED_IP_ADDRESS",
            Self::TooManyAsns => "TOO_MANY_ASNS",
            Self::TooManyIps => "TOO_MANY_IPS",
            Self::TooManyRequests => "TOO_MANY_REQUESTS",
            Self::TooManyUserAgents => "TOO_MANY_USER_AGENTS",
            Self::UnknownAsn => "UNKNOWN_ASN",
            Self::Other(raw) => raw,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Mirrors the JSON error body returned by the API.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct ApiErrorPayload {
    pub(crate) code: String,
    #[serde(default)]
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) resolution: String,
}

impl ApiErrorPayload {
    /// Converts a decoded payload into a typed [`ApiError`].
    pub(crate) fn into_api_error(self, status: Option<u16>) -> ApiError {
        ApiError {
            code: (!self.code.is_empty()).then(|| ErrorCode::from_raw(&self.code)),
            message: self.message,
            resolution: self.resolution,
            status,
        }
    }
}

/// Converts a non-2xx response body into an [`ApiError`], falling back to a
/// generic message when the body is not a recognizable error payload.
pub(crate) fn parse_api_error(data: &[u8], status: u16) -> ApiError {
    match serde_json::from_slice::<ApiErrorPayload>(data) {
        Ok(payload) if !payload.code.is_empty() => payload.into_api_error(Some(status)),
        _ => ApiError {
            code: None,
            message: format!("unexpected HTTP status {status}"),
            resolution: String::new(),
            status: Some(status),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_roundtrip() {
        let codes = [
            "BAD_REQUEST",
            "DISABLED_API_KEY",
            "FORBIDDEN_IP",
            "FORBIDDEN_ORIGIN",
            "FORBIDDEN_IP_ORIGIN",
            "INTERNAL",
            "INSUFFICIENT_CREDITS",
            "INVALID_API_KEY",
            "INVALID_ASN",
            "INVALID_FILTER_SYNTAX",
            "INVALID_IP_ADDRESS",
            "MISSING_API_KEY",
            "RESERVED_ASN",
            "RESERVED_IP_ADDRESS",
            "TOO_MANY_ASNS",
            "TOO_MANY_IPS",
            "TOO_MANY_REQUESTS",
            "TOO_MANY_USER_AGENTS",
            "UNKNOWN_ASN",
        ];
        for raw in codes {
            let code = ErrorCode::from_raw(raw);
            assert!(!matches!(code, ErrorCode::Other(_)), "{raw} not recognized");
            assert_eq!(code.as_str(), raw);
        }
    }

    #[test]
    fn error_code_is_case_and_space_insensitive() {
        assert_eq!(
            ErrorCode::from_raw(" invalid_api_key "),
            ErrorCode::InvalidApiKey
        );
    }

    #[test]
    fn unknown_error_code_is_preserved() {
        let code = ErrorCode::from_raw("SOMETHING_NEW");
        assert_eq!(code, ErrorCode::Other("SOMETHING_NEW".to_owned()));
        assert_eq!(code.as_str(), "SOMETHING_NEW");
    }

    #[test]
    fn api_error_display() {
        let err = ApiError {
            code: Some(ErrorCode::InvalidIpAddress),
            message: "the IP address is invalid".to_owned(),
            resolution: "provide a valid IPv4 or IPv6 address".to_owned(),
            status: Some(400),
        };
        assert_eq!(
            err.to_string(),
            "ipregistry: the IP address is invalid (INVALID_IP_ADDRESS): \
             provide a valid IPv4 or IPv6 address"
        );
    }

    #[test]
    fn api_error_display_without_details() {
        let err = ApiError {
            code: None,
            message: String::new(),
            resolution: String::new(),
            status: Some(502),
        };
        assert_eq!(err.to_string(), "ipregistry: API error");
    }

    #[test]
    fn parse_api_error_with_payload() {
        let body =
            br#"{"code":"INVALID_API_KEY","message":"key is invalid","resolution":"fix it"}"#;
        let err = parse_api_error(body, 403);
        assert_eq!(err.code, Some(ErrorCode::InvalidApiKey));
        assert_eq!(err.message, "key is invalid");
        assert_eq!(err.resolution, "fix it");
        assert_eq!(err.status, Some(403));
    }

    #[test]
    fn parse_api_error_with_unrecognizable_body() {
        let err = parse_api_error(b"<html>Bad Gateway</html>", 502);
        assert_eq!(err.code, None);
        assert_eq!(err.message, "unexpected HTTP status 502");
        assert_eq!(err.status, Some(502));
    }
}

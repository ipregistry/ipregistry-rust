use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use super::de::null_default;
use super::location::Location;
use super::user_agent::UserAgent;

/// The comprehensive set of information associated with an IP address, as
/// returned by the Ipregistry API.
///
/// Nested objects are always present as values, so accessing their fields
/// never requires unwrapping; data points the API does not know hold their
/// default value. See the [module documentation](crate::model) for details.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct IpInfo {
    /// The IP address the data refers to, or `None` when excluded by a
    /// `fields` selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<IpAddr>,
    /// The IP version.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub ip_type: Option<IpType>,
    /// The reverse-DNS hostname, when hostname resolution is requested (see
    /// [`LookupRequest::hostname`](crate::LookupRequest::hostname)) and
    /// available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// Mobile carrier information.
    #[serde(default, deserialize_with = "null_default")]
    pub carrier: Carrier,
    /// Ownership information for the IP address.
    #[serde(default, deserialize_with = "null_default")]
    pub company: Company,
    /// Network connection information.
    #[serde(default, deserialize_with = "null_default")]
    pub connection: Connection,
    /// Currency information for the IP address location.
    #[serde(default, deserialize_with = "null_default")]
    pub currency: Currency,
    /// Geographical location.
    #[serde(default, deserialize_with = "null_default")]
    pub location: Location,
    /// Threat-intelligence flags.
    #[serde(default, deserialize_with = "null_default")]
    pub security: Security,
    /// Time zone information for the IP address location.
    #[serde(default, deserialize_with = "null_default")]
    pub time_zone: TimeZone,
}

/// [`IpInfo`] enriched with parsed User-Agent data. It is returned by
/// [`Client::lookup_origin`](crate::Client::lookup_origin), where the
/// User-Agent of the calling client is known.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct RequesterIpInfo {
    /// The IP data for the requester address.
    #[serde(flatten)]
    pub info: IpInfo,
    /// The parsed User-Agent of the requester, or `None` when the API did not
    /// return any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<UserAgent>,
}

/// The version of an IP address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub enum IpType {
    /// An IPv4 address.
    #[serde(rename = "IPv4")]
    V4,
    /// An IPv6 address.
    #[serde(rename = "IPv6")]
    V6,
    /// The IP version could not be determined.
    Unknown,
    /// A value this version of the crate does not know about.
    #[serde(untagged)]
    Other(String),
}

/// Mobile carrier information associated with an IP address.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Carrier {
    /// The carrier name.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The Mobile Country Code.
    #[serde(default, deserialize_with = "null_default")]
    pub mcc: String,
    /// The Mobile Network Code.
    #[serde(default, deserialize_with = "null_default")]
    pub mnc: String,
}

/// The kind of company that owns an IP address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum CompanyType {
    /// A regular business.
    Business,
    /// An educational institution.
    Education,
    /// A government agency.
    Government,
    /// A hosting or cloud provider.
    Hosting,
    /// An Internet service provider.
    Isp,
    /// A value this version of the crate does not know about.
    #[serde(untagged)]
    Other(String),
}

/// Ownership information for an IP address.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Company {
    /// The company name.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The company domain.
    #[serde(default, deserialize_with = "null_default")]
    pub domain: String,
    /// The kind of company, or `None` when unknown.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub company_type: Option<CompanyType>,
}

/// The kind of network an IP address belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ConnectionType {
    /// A business network.
    Business,
    /// An educational institution network.
    Education,
    /// A government network.
    Government,
    /// A hosting or cloud provider network.
    Hosting,
    /// An inactive network.
    Inactive,
    /// An Internet service provider network.
    Isp,
    /// A value this version of the crate does not know about.
    #[serde(untagged)]
    Other(String),
}

/// Network connection information for an IP address.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Connection {
    /// The Autonomous System Number, or `None` when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asn: Option<u32>,
    /// The domain of the Autonomous System.
    #[serde(default, deserialize_with = "null_default")]
    pub domain: String,
    /// The organization operating the Autonomous System.
    #[serde(default, deserialize_with = "null_default")]
    pub organization: String,
    /// The BGP route (CIDR prefix) announcing the IP address.
    #[serde(default, deserialize_with = "null_default")]
    pub route: String,
    /// The kind of network, or `None` when unknown.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
}

/// Currency information for an IP address location.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Currency {
    /// The ISO 4217 currency code (for example `USD`).
    #[serde(default, deserialize_with = "null_default")]
    pub code: String,
    /// The currency name in English.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The currency name in the local language.
    #[serde(default, deserialize_with = "null_default")]
    pub name_native: String,
    /// The plural form of the currency name in English.
    #[serde(default, deserialize_with = "null_default")]
    pub plural: String,
    /// The plural form of the currency name in the local language.
    #[serde(default, deserialize_with = "null_default")]
    pub plural_native: String,
    /// The currency symbol (for example `$`).
    #[serde(default, deserialize_with = "null_default")]
    pub symbol: String,
    /// The currency symbol in the local script.
    #[serde(default, deserialize_with = "null_default")]
    pub symbol_native: String,
    /// How monetary values are formatted for the currency.
    #[serde(default, deserialize_with = "null_default")]
    pub format: CurrencyFormat,
}

/// How monetary values are formatted for a currency.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct CurrencyFormat {
    /// The character separating the integer part from the fraction.
    #[serde(default, deserialize_with = "null_default")]
    pub decimal_separator: String,
    /// The character grouping thousands.
    #[serde(default, deserialize_with = "null_default")]
    pub group_separator: String,
    /// The affixes applied around negative amounts.
    #[serde(default, deserialize_with = "null_default")]
    pub negative: CurrencyFormatAffix,
    /// The affixes applied around positive amounts.
    #[serde(default, deserialize_with = "null_default")]
    pub positive: CurrencyFormatAffix,
}

/// The prefix and suffix applied around a formatted monetary value (for
/// example the currency symbol and a sign).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct CurrencyFormatAffix {
    /// Text placed before the amount.
    #[serde(default, deserialize_with = "null_default")]
    pub prefix: String,
    /// Text placed after the amount.
    #[serde(default, deserialize_with = "null_default")]
    pub suffix: String,
}

/// Threat-intelligence flags for an IP address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Security {
    /// The IP address is a known source of abuse (spam, scraping, ...).
    #[serde(default, deserialize_with = "null_default")]
    pub is_abuser: bool,
    /// The IP address is a known source of attacks.
    #[serde(default, deserialize_with = "null_default")]
    pub is_attacker: bool,
    /// The IP address is a bogon (not in any range allocated for public use).
    #[serde(default, deserialize_with = "null_default")]
    pub is_bogon: bool,
    /// The IP address belongs to a cloud provider.
    #[serde(default, deserialize_with = "null_default")]
    pub is_cloud_provider: bool,
    /// The IP address is an open proxy.
    #[serde(default, deserialize_with = "null_default")]
    pub is_proxy: bool,
    /// The IP address is a relay (for example iCloud Private Relay).
    #[serde(default, deserialize_with = "null_default")]
    pub is_relay: bool,
    /// The IP address is a Tor node.
    #[serde(default, deserialize_with = "null_default")]
    pub is_tor: bool,
    /// The IP address is a Tor exit node.
    #[serde(default, deserialize_with = "null_default")]
    pub is_tor_exit: bool,
    /// The IP address anonymizes its user (VPN, proxy, relay, or Tor).
    #[serde(default, deserialize_with = "null_default")]
    pub is_anonymous: bool,
    /// The IP address is considered a threat.
    #[serde(default, deserialize_with = "null_default")]
    pub is_threat: bool,
    /// The IP address belongs to a VPN provider.
    #[serde(default, deserialize_with = "null_default")]
    pub is_vpn: bool,
}

/// Time zone information for an IP address location.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct TimeZone {
    /// The IANA time zone identifier (for example `America/Los_Angeles`).
    #[serde(default, deserialize_with = "null_default")]
    pub id: String,
    /// The time zone abbreviation (for example `PDT`).
    #[serde(default, deserialize_with = "null_default")]
    pub abbreviation: String,
    /// The current local time in RFC 3339 format.
    #[serde(default, deserialize_with = "null_default")]
    pub current_time: String,
    /// The time zone name (for example `Pacific Daylight Time`).
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The current offset from UTC in seconds.
    #[serde(default, deserialize_with = "null_default")]
    pub offset: i32,
    /// Whether daylight saving time is currently in effect.
    #[serde(default, deserialize_with = "null_default")]
    pub in_daylight_saving: bool,
}

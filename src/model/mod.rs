//! Data types returned by the Ipregistry API.
//!
//! # Absent fields
//!
//! The API returns `null` for data points it does not know, and omits fields
//! that were excluded with a `fields` selection (see
//! [`LookupRequest::fields`](crate::LookupRequest::fields)). To keep access
//! ergonomic, nested objects are always present as values and plain fields
//! fall back to their [`Default`] (an empty string, `0`, `false`, ...) —
//! accessing them never requires unwrapping. [`Option`] is reserved for fields
//! whose default value would be ambiguous, such as
//! [`Connection::asn`] (where `0` is a real ASN) or
//! [`Location::latitude`] (where `0.0` is a real coordinate).

mod ip_info;
mod location;
mod user_agent;

pub use ip_info::{
    Carrier, Company, CompanyType, Connection, ConnectionType, Currency, CurrencyFormat,
    CurrencyFormatAffix, IpInfo, IpType, RequesterIpInfo, Security, TimeZone,
};
pub use location::{Continent, Country, Flag, Language, Location, Region};
pub use user_agent::{UserAgent, UserAgentDevice, UserAgentEngine, UserAgentOs, is_bot};

pub(crate) mod de {
    //! Deserialization helpers shared by the model types.

    use serde::{Deserialize, Deserializer};

    /// Deserializes a value, mapping JSON `null` to the type's default. The
    /// Ipregistry API returns explicit `null` for unknown data points; combined
    /// with `#[serde(default)]` this makes fields tolerant to both `null` and
    /// omission.
    pub(crate) fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de> + Default,
    {
        Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
    }
}

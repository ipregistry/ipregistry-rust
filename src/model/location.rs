use serde::{Deserialize, Serialize};

use super::de::null_default;

/// The geographical location associated with an IP address.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Location {
    /// Continent-level information.
    #[serde(default, deserialize_with = "null_default")]
    pub continent: Continent,
    /// Country-level information.
    #[serde(default, deserialize_with = "null_default")]
    pub country: Country,
    /// Administrative region (state/province) information.
    #[serde(default, deserialize_with = "null_default")]
    pub region: Region,
    /// The city name.
    #[serde(default, deserialize_with = "null_default")]
    pub city: String,
    /// The postal code.
    #[serde(default, deserialize_with = "null_default")]
    pub postal: String,
    /// The decimal-degree latitude, or `None` when unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// The decimal-degree longitude, or `None` when unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// The primary language spoken at the location.
    #[serde(default, deserialize_with = "null_default")]
    pub language: Language,
    /// Whether the location is within a European Union member state.
    #[serde(default, deserialize_with = "null_default")]
    pub in_eu: bool,
}

/// Continent-level information for a location.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Continent {
    /// The two-letter continent code (for example `NA`).
    #[serde(default, deserialize_with = "null_default")]
    pub code: String,
    /// The continent name.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
}

/// Country-level information for a location.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Country {
    /// The total land area in square kilometers.
    #[serde(default, deserialize_with = "null_default")]
    pub area: f64,
    /// The ISO 3166-1 alpha-2 codes of bordering countries.
    #[serde(default, deserialize_with = "null_default")]
    pub borders: Vec<String>,
    /// The international calling code (for example `1`).
    #[serde(default, deserialize_with = "null_default")]
    pub calling_code: String,
    /// The capital city.
    #[serde(default, deserialize_with = "null_default")]
    pub capital: String,
    /// The ISO 3166-1 alpha-2 country code (for example `US`).
    #[serde(default, deserialize_with = "null_default")]
    pub code: String,
    /// The country name.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The estimated number of inhabitants.
    #[serde(default, deserialize_with = "null_default")]
    pub population: u64,
    /// The number of inhabitants per square kilometer.
    #[serde(default, deserialize_with = "null_default")]
    pub population_density: f64,
    /// Representations of the country flag.
    #[serde(default, deserialize_with = "null_default")]
    pub flag: Flag,
    /// The languages spoken in the country.
    #[serde(default, deserialize_with = "null_default")]
    pub languages: Vec<Language>,
    /// The country-code top-level domain (for example `.us`).
    #[serde(default, deserialize_with = "null_default")]
    pub tld: String,
}

/// Administrative region (state/province) information.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Region {
    /// The region code, typically the ISO 3166-2 subdivision code.
    #[serde(default, deserialize_with = "null_default")]
    pub code: String,
    /// The region name.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
}

/// Language information.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Language {
    /// The ISO 639-1 language code (for example `en`).
    #[serde(default, deserialize_with = "null_default")]
    pub code: String,
    /// The language name in English.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The language's name in the language itself.
    #[serde(default, rename = "native", deserialize_with = "null_default")]
    pub native_name: String,
}

/// Representations of a country flag across several icon sets.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Flag {
    /// The flag as an emoji character.
    #[serde(default, deserialize_with = "null_default")]
    pub emoji: String,
    /// The Unicode code points of the emoji flag.
    #[serde(default, deserialize_with = "null_default")]
    pub emoji_unicode: String,
    /// A URL to the EmojiTwo flag icon.
    #[serde(default, deserialize_with = "null_default")]
    pub emojitwo: String,
    /// A URL to the Noto flag icon.
    #[serde(default, deserialize_with = "null_default")]
    pub noto: String,
    /// A URL to the Twemoji flag icon.
    #[serde(default, deserialize_with = "null_default")]
    pub twemoji: String,
    /// A URL to the Wikimedia flag image.
    #[serde(default, deserialize_with = "null_default")]
    pub wikimedia: String,
}

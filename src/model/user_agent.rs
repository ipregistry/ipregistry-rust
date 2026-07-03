use serde::{Deserialize, Serialize};

use super::de::null_default;

/// Structured data parsed from a raw User-Agent string.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct UserAgent {
    /// The raw User-Agent string that was parsed.
    #[serde(default, deserialize_with = "null_default")]
    pub header: String,
    /// The agent name (for example `Chrome`).
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The agent type (for example `browser`).
    #[serde(default, rename = "type", deserialize_with = "null_default")]
    pub agent_type: String,
    /// The agent version.
    #[serde(default, deserialize_with = "null_default")]
    pub version: String,
    /// The agent major version.
    #[serde(default, deserialize_with = "null_default")]
    pub version_major: String,
    /// The device data parsed from the User-Agent string.
    #[serde(default, deserialize_with = "null_default")]
    pub device: UserAgentDevice,
    /// The layout-engine data parsed from the User-Agent string.
    #[serde(default, deserialize_with = "null_default")]
    pub engine: UserAgentEngine,
    /// The operating-system data parsed from the User-Agent string.
    #[serde(default, deserialize_with = "null_default")]
    pub os: UserAgentOs,
}

/// Device data parsed from a User-Agent string.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct UserAgentDevice {
    /// The device brand (for example `Apple`).
    #[serde(default, deserialize_with = "null_default")]
    pub brand: String,
    /// The device name.
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The device type (for example `desktop`).
    #[serde(default, rename = "type", deserialize_with = "null_default")]
    pub device_type: String,
}

/// Layout-engine data parsed from a User-Agent string.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct UserAgentEngine {
    /// The engine name (for example `Blink`).
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The engine type.
    #[serde(default, rename = "type", deserialize_with = "null_default")]
    pub engine_type: String,
    /// The engine version.
    #[serde(default, deserialize_with = "null_default")]
    pub version: String,
    /// The engine major version.
    #[serde(default, deserialize_with = "null_default")]
    pub version_major: String,
}

/// Operating-system data parsed from a User-Agent string.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct UserAgentOs {
    /// The operating-system name (for example `macOS`).
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    /// The operating-system type (for example `desktop`).
    #[serde(default, rename = "type", deserialize_with = "null_default")]
    pub os_type: String,
    /// The operating-system version.
    #[serde(default, deserialize_with = "null_default")]
    pub version: String,
}

/// Reports whether the given raw User-Agent string looks like a crawler or
/// bot. It is a lightweight heuristic — useful for skipping IP lookups on
/// automated traffic — that matches the substrings `bot`, `spider`, and
/// `slurp` case-insensitively.
///
/// ```
/// assert!(ipregistry::is_bot("Mozilla/5.0 (compatible; Googlebot/2.1)"));
/// assert!(!ipregistry::is_bot("Mozilla/5.0 (Macintosh) Safari/605.1.15"));
/// ```
pub fn is_bot(user_agent: &str) -> bool {
    let ua = user_agent.to_ascii_lowercase();
    ua.contains("bot") || ua.contains("spider") || ua.contains("slurp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bots() {
        for ua in [
            "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
            "Mozilla/5.0 (compatible; Baiduspider/2.0)",
            "Mozilla/5.0 (compatible; Yahoo! Slurp)",
            "SomeBOT/1.0",
        ] {
            assert!(is_bot(ua), "{ua} should be detected as a bot");
        }
    }

    #[test]
    fn does_not_flag_regular_browsers() {
        for ua in [
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/120.0 Safari/537.36",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) Version/17.0 Mobile Safari",
            "",
        ] {
            assert!(!is_bot(ua), "{ua} should not be detected as a bot");
        }
    }
}

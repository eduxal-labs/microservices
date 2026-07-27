use chrono::Duration;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Trait implemented by token payloads (`Access`, `Refresh`, `Setup`)
/// defining their type kind string and default TTL.
pub trait TokenType: Serialize + DeserializeOwned + Send + Sync {
    const KIND: &'static str;
    const TTL: Duration;

    /// Calculates expiration timestamp based on current UTC time and `TTL`.
    fn expiry() -> crate::types::DateTime {
        let now = crate::types::DateTime::now();
        let expires_ts = now.timestamp() + Self::TTL.num_seconds();
        crate::types::DateTime::from_timestamp(expires_ts, 0).unwrap_or(now)
    }
}

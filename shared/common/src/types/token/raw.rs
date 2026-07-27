use crate::types::DateTime;
use serde::{Deserialize, Serialize};

/// Internal helper representation used for Serde serialization / deserialization and PASETO payload formatting.
#[derive(Debug, Serialize)]
pub struct RawTokenRef<'a, T> {
    #[serde(rename = "type")]
    pub token_type: &'static str,
    pub expires: &'a DateTime,
    #[serde(flatten)]
    pub claims: &'a T,
}

/// Internal helper representation used for Serde deserialization.
#[derive(Debug, Deserialize)]
pub struct RawTokenDe<T> {
    #[serde(rename = "type")]
    pub token_type: String,
    pub expires: DateTime,
    #[serde(flatten)]
    pub claims: T,
}

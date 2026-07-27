use crate::types::{token::traits::TokenType, Phone};
use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Setup token payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setup {
    pub phone: Phone,
}

impl TokenType for Setup {
    const KIND: &'static str = "Setup";
    const TTL: Duration = Duration::minutes(60);
}

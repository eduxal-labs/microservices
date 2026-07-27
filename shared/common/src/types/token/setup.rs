use crate::types::{token::traits::TokenType, Id, Phone};
use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Setup token payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setup {
    pub id: Id,
    pub phone: Phone,
}

impl TokenType for Setup {
    const KIND: &'static str = "Setup";
    const TTL: Duration = Duration::minutes(60);
}

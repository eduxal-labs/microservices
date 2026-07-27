use crate::types::{token::traits::TokenType, Id};
use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Refresh token payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refresh {
    pub session: Id,
    pub user: Id,
}

impl TokenType for Refresh {
    const KIND: &'static str = "Refresh";
    const TTL: Duration = Duration::days(30);
}

use crate::types::{token::traits::TokenType, Id};
use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Access token payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Access {
    pub session: Id,
    pub user: Id,
}

impl TokenType for Access {
    const KIND: &'static str = "Access";
    const TTL: Duration = Duration::minutes(15);
}

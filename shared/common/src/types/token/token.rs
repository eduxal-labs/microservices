use crate::types::{
    token::{
        access::Access,
        raw::{RawTokenDe, RawTokenRef},
        refresh::Refresh,
        setup::Setup,
        traits::TokenType,
    },
    DateTime, Error, Id, Phone,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Deref;

/// Generic Token wrapper where the state is directly tied to the type parameter `T`.
/// `expires` is stored on the Token struct, while `T` defines the payload claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<T> {
    pub claims: T,
    pub expires: DateTime,
}

impl<T: TokenType> Token<T> {
    /// Creates a new `Token<T>` with default TTL for type `T`.
    pub fn new(claims: T) -> Self {
        Self {
            claims,
            expires: T::expiry(),
        }
    }

    /// Creates a new `Token<T>` with an explicit expiration timestamp.
    pub fn with_expiry(claims: T, expires: DateTime) -> Self {
        Self { claims, expires }
    }

    pub fn claims(&self) -> &T {
        &self.claims
    }

    pub fn into_claims(self) -> T {
        self.claims
    }

    /// Returns true if the token's expiration timestamp is before or equal to the current time.
    pub fn is_expired(&self) -> bool {
        self.expires.timestamp() <= DateTime::now().timestamp()
    }

    /// Encrypts this token into a PASETO v4-local string using a 32-byte secret key.
    pub fn encode_paseto(&self, secret_key: &[u8; 32]) -> Result<String, Error> {
        use rusty_paseto::core::{Key, PasetoSymmetricKey};
        use rusty_paseto::generic::*;
        use rusty_paseto::prelude::*;

        let raw = RawTokenRef {
            token_type: T::KIND,
            expires: &self.expires,
            claims: &self.claims,
        };

        let key = PasetoSymmetricKey::<V4, Local>::from(Key::<32>::from(*secret_key));
        let token_json = serde_json::to_string(&raw).map_err(|_| Error::InvalidToken)?;
        let claim =
            CustomClaim::try_from(("data", token_json.as_str())).map_err(|_| Error::InvalidToken)?;

        let token_string = PasetoBuilder::<V4, Local>::default()
            .set_claim(claim)
            .build(&key)
            .map_err(|_| Error::InvalidToken)?;

        Ok(token_string)
    }

    /// Decrypts a PASETO v4-local string back into `Token<T>`.
    /// Performs type validation and expiry validation during parsing.
    pub fn decode_paseto(token_str: &str, secret_key: &[u8; 32]) -> Result<Self, Error> {
        use rusty_paseto::core::{Key, PasetoSymmetricKey};
        use rusty_paseto::generic::*;
        use rusty_paseto::prelude::*;

        let key = PasetoSymmetricKey::<V4, Local>::from(Key::<32>::from(*secret_key));
        let json_value = PasetoParser::<V4, Local>::default()
            .parse(token_str, &key)
            .map_err(|_| Error::InvalidToken)?;

        let data_str = json_value
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or(Error::InvalidToken)?;

        let token: Self = serde_json::from_str(data_str).map_err(|_| Error::InvalidToken)?;
        Ok(token)
    }
}

impl<T> Deref for Token<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.claims
    }
}

impl Token<Access> {
    pub fn access(session: Id, user: Id) -> Self {
        Self::new(Access { session, user })
    }
}

impl Token<Refresh> {
    pub fn refresh(session: Id, user: Id) -> Self {
        Self::new(Refresh { session, user })
    }
}

impl Token<Setup> {
    pub fn setup(phone: Phone) -> Self {
        Self::new(Setup { phone })
    }
}

impl<T: TokenType> Serialize for Token<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw = RawTokenRef {
            token_type: T::KIND,
            expires: &self.expires,
            claims: &self.claims,
        };
        raw.serialize(serializer)
    }
}

impl<'de, T: TokenType> Deserialize<'de> for Token<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawTokenDe::<T>::deserialize(deserializer)?;

        if raw.token_type != T::KIND {
            return Err(serde::de::Error::custom(format!(
                "invalid token type: expected '{}', got '{}'",
                T::KIND,
                raw.token_type
            )));
        }

        let token = Token {
            claims: raw.claims,
            expires: raw.expires,
        };

        if token.is_expired() {
            return Err(serde::de::Error::custom("token has expired"));
        }

        Ok(token)
    }
}

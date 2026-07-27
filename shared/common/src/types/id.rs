use bson::oid::ObjectId;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub ObjectId);

impl Id {
    pub fn new() -> Self {
        Self(ObjectId::new())
    }

    pub fn from_bytes(bytes: [u8; 12]) -> Self {
        Self(ObjectId::from_bytes(bytes))
    }

    pub fn bytes(&self) -> [u8; 12] {
        self.0.bytes()
    }

    pub fn as_bytes_ref(&self) -> &[u8; 12] {
        unsafe { &*(self as *const Id as *const [u8; 12]) }
    }

    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.to_hex())
    }
}

impl FromStr for Id {
    type Err = bson::oid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ObjectId::parse_str(s).map(Id)
    }
}

impl From<ObjectId> for Id {
    fn from(oid: ObjectId) -> Self {
        Self(oid)
    }
}

impl From<Id> for ObjectId {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IdVisitor;

        impl<'de> de::Visitor<'de> for IdVisitor {
            type Value = Id;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a 24-character hex string representing an ObjectId")
            }

            fn visit_str<E>(self, value: &str) -> Result<Id, E>
            where
                E: de::Error,
            {
                Id::from_str(value).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(IdVisitor)
    }
}

#[cfg(feature = "diesel")]
mod diesel_impl {
    use super::*;
    use diesel::backend::Backend;
    use diesel::deserialize::{self, FromSql};
    use diesel::serialize::{self, ToSql};
    use diesel::sql_types::Binary;

    impl<DB> ToSql<Binary, DB> for Id
    where
        DB: Backend,
        [u8]: ToSql<Binary, DB>,
    {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, DB>) -> serialize::Result {
            <[u8] as ToSql<Binary, DB>>::to_sql(self.as_bytes_ref(), out)
        }
    }

    impl<DB> FromSql<Binary, DB> for Id
    where
        DB: Backend,
        Vec<u8>: FromSql<Binary, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
            let raw_bytes = Vec::<u8>::from_sql(bytes)?;
            if raw_bytes.len() != 12 {
                return Err("Invalid byte length for ObjectId, expected 12 bytes".into());
            }
            let mut arr = [0u8; 12];
            arr.copy_from_slice(&raw_bytes);
            Ok(Id::from_bytes(arr))
        }
    }
}

#[cfg(feature = "dynamodb")]
pub mod dynamodb_impl {
    use super::*;
    use aws_sdk_dynamodb::types::AttributeValue;

    #[derive(Debug, thiserror::Error)]
    pub enum IdParseError {
        #[error("Invalid AttributeValue type, expected String AttributeValue::S")]
        InvalidAttributeType,
        #[error("Invalid ObjectId string format: {0}")]
        InvalidFormat(#[from] bson::oid::Error),
    }

    impl From<Id> for AttributeValue {
        fn from(id: Id) -> Self {
            AttributeValue::S(id.to_hex())
        }
    }

    impl From<&Id> for AttributeValue {
        fn from(id: &Id) -> Self {
            AttributeValue::S(id.to_hex())
        }
    }

    impl TryFrom<&AttributeValue> for Id {
        type Error = IdParseError;

        fn try_from(value: &AttributeValue) -> Result<Self, Self::Error> {
            match value {
                AttributeValue::S(s) => Id::from_str(s).map_err(IdParseError::InvalidFormat),
                _ => Err(IdParseError::InvalidAttributeType),
            }
        }
    }

    impl TryFrom<AttributeValue> for Id {
        type Error = IdParseError;

        fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
            Self::try_from(&value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation_and_hex_conversion() {
        let id = Id::new();
        let hex = id.to_hex();
        assert_eq!(hex.len(), 24);

        let parsed = Id::from_str(&hex).expect("Should parse valid hex string");
        assert_eq!(id, parsed);
        assert_eq!(id.bytes(), parsed.bytes());
    }

    #[test]
    fn test_serde_serialization() {
        let id = Id::new();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{}\"", id.to_hex()));

        let deserialized: Id = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[cfg(feature = "dynamodb")]
    #[test]
    fn test_dynamodb_attribute_value_conversion() {
        use aws_sdk_dynamodb::types::AttributeValue;

        let id = Id::new();
        let av: AttributeValue = (&id).into();

        assert_eq!(av, AttributeValue::S(id.to_hex()));

        let converted_id = Id::try_from(&av).expect("Should convert AttributeValue::S to Id");
        assert_eq!(id, converted_id);
    }
}

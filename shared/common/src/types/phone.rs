use super::Error;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Phone(String);

impl Phone {
    /// Validates if a string is in international E.164 country code format.
    /// E.164 requires a leading '+' followed by 7 to 15 digits.
    pub fn validate(s: &str) -> bool {
        if !s.starts_with('+') {
            return false;
        }
        let digits = &s[1..];
        digits.len() >= 7 && digits.len() <= 15 && digits.chars().all(|c| c.is_ascii_digit())
    }

    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if Self::validate(&s) {
            Ok(Self(s))
        } else {
            Err(Error::InvalidPhone)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for Phone {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for Phone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Phone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Phone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Phone::new(s).map_err(de::Error::custom)
    }
}

#[cfg(feature = "diesel")]
mod diesel_impl {
    use super::*;
    use ::diesel::deserialize::{self, FromSql};
    use ::diesel::serialize::{self, ToSql};
    use ::diesel::sql_types::Text;
    use ::diesel::sqlite::Sqlite;

    impl ToSql<Text, Sqlite> for Phone {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
            <str as ToSql<Text, Sqlite>>::to_sql(self.as_str(), out)
        }
    }

    impl FromSql<Text, Sqlite> for Phone {
        fn from_sql(bytes: <Sqlite as ::diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            let s = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
            Phone::new(s).map_err(|e| e.into())
        }
    }
}

#[cfg(feature = "dynamodb")]
mod dynamodb_impl {
    use super::*;
    use aws_sdk_dynamodb::types::AttributeValue;

    impl From<Phone> for AttributeValue {
        fn from(phone: Phone) -> Self {
            AttributeValue::S(phone.into_inner())
        }
    }

    impl From<&Phone> for AttributeValue {
        fn from(phone: &Phone) -> Self {
            AttributeValue::S(phone.to_string())
        }
    }

    impl TryFrom<&AttributeValue> for Phone {
        type Error = Error;

        fn try_from(value: &AttributeValue) -> Result<Self, Self::Error> {
            match value {
                AttributeValue::S(s) => Phone::new(s),
                _ => Err(Error::InvalidAttributeValue),
            }
        }
    }

    impl TryFrom<AttributeValue> for Phone {
        type Error = Error;

        fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
            Self::try_from(&value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_validation() {
        assert!(Phone::new("+254712345678").is_ok());
        assert!(Phone::new("+14155552671").is_ok());

        assert_eq!(Phone::new("0712345678"), Err(Error::InvalidPhone));
        assert_eq!(Phone::new("254712345678"), Err(Error::InvalidPhone));
        assert_eq!(Phone::new("+123"), Err(Error::InvalidPhone));
        assert_eq!(Phone::new("+abcdefghijk"), Err(Error::InvalidPhone));
    }

    #[test]
    fn test_phone_serde() {
        let valid_phone = Phone::new("+254712345678").unwrap();
        let json = serde_json::to_string(&valid_phone).unwrap();
        assert_eq!(json, "\"+254712345678\"");

        let deserialized: Phone = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, valid_phone);

        let invalid_json = "\"0712345678\"";
        assert!(serde_json::from_str::<Phone>(invalid_json).is_err());
    }

    #[cfg(feature = "dynamodb")]
    #[test]
    fn test_dynamodb_conversion() {
        use aws_sdk_dynamodb::types::AttributeValue;

        let phone = Phone::new("+254712345678").unwrap();
        let attr: AttributeValue = (&phone).into();
        assert_eq!(attr.as_s().unwrap(), "+254712345678");

        let parsed = Phone::try_from(&attr).unwrap();
        assert_eq!(parsed, phone);
    }
}

use super::{DateTime, Error, Phone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    pub phone: Phone,
    pub code: String,
    pub created: DateTime,
    pub ttl: DateTime,
}

impl Verification {
    pub fn new(phone: Phone, code: impl Into<String>, created: DateTime, ttl: DateTime) -> Self {
        Self {
            phone,
            code: code.into(),
            created,
            ttl,
        }
    }
}

#[cfg(feature = "dynamodb")]
mod dynamodb_impl {
    use super::*;
    use aws_sdk_dynamodb::types::AttributeValue;
    use std::collections::HashMap;

    impl From<&Verification> for HashMap<String, AttributeValue> {
        fn from(v: &Verification) -> Self {
            let mut map = HashMap::new();
            map.insert("phone".to_string(), AttributeValue::from(&v.phone));
            map.insert("code".to_string(), AttributeValue::S(v.code.clone()));
            map.insert("created".to_string(), AttributeValue::from(&v.created));
            map.insert("ttl".to_string(), AttributeValue::N(v.ttl.timestamp().to_string()));
            map
        }
    }

    impl TryFrom<&HashMap<String, AttributeValue>> for Verification {
        type Error = Error;

        fn try_from(map: &HashMap<String, AttributeValue>) -> Result<Self, Self::Error> {
            let phone_attr = map.get("phone").ok_or(Error::InvalidAttributeValue)?;
            let code_attr = map.get("code").ok_or(Error::InvalidAttributeValue)?;
            let created_attr = map.get("created").ok_or(Error::InvalidAttributeValue)?;
            let ttl_attr = map.get("ttl").ok_or(Error::InvalidAttributeValue)?;

            let phone = Phone::try_from(phone_attr)?;
            let code = code_attr.as_s().map_err(|_| Error::InvalidAttributeValue)?.clone();
            let created = DateTime::try_from(created_attr)?;
            let ttl = DateTime::try_from(ttl_attr)?;

            Ok(Verification {
                phone,
                code,
                created,
                ttl,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_serde() {
        let now = DateTime::now();
        let ttl = DateTime::from_timestamp(now.timestamp() + 300, 0).unwrap();
        let verification = Verification::new(
            Phone::new("+254712345678").unwrap(),
            "123456",
            now,
            ttl,
        );

        let json = serde_json::to_string(&verification).unwrap();
        let deserialized: Verification = serde_json::from_str(&json).unwrap();
        assert_eq!(verification, deserialized);
    }

    #[cfg(feature = "dynamodb")]
    #[test]
    fn test_dynamodb_verification_conversion() {
        use aws_sdk_dynamodb::types::AttributeValue;
        use std::collections::HashMap;

        let now = DateTime::now();
        let ttl = DateTime::from_timestamp(now.timestamp() + 300, 0).unwrap();
        let verification = Verification::new(
            Phone::new("+254712345678").unwrap(),
            "123456",
            now,
            ttl,
        );

        let item: HashMap<String, AttributeValue> = (&verification).into();
        let parsed = Verification::try_from(&item).unwrap();

        assert_eq!(verification.phone, parsed.phone);
        assert_eq!(verification.code, parsed.code);
        assert_eq!(verification.created.timestamp(), parsed.created.timestamp());
        assert_eq!(verification.ttl.timestamp(), parsed.ttl.timestamp());
    }
}

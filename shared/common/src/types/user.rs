use super::{DateTime, Error, Id, Phone};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Id,
    pub phone: Phone,
    pub name: String,
    pub status: Status,
    pub created: DateTime,
}

impl User {
    pub fn new(id: Id, phone: Phone, name: impl Into<String>, status: Status, created: DateTime) -> Self {
        Self {
            id,
            phone,
            name: name.into(),
            status,
            created,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
pub enum Status {
    Active,
    Invited,
    Suspended,
    Deleted,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Active => "Active",
            Status::Invited => "Invited",
            Status::Suspended => "Suspended",
            Status::Deleted => "Deleted",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Status {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(Status::Active),
            "Invited" => Ok(Status::Invited),
            "Suspended" => Ok(Status::Suspended),
            "Deleted" => Ok(Status::Deleted),
            _ => Err(Error::InvalidAttributeValue),
        }
    }
}

#[cfg(feature = "diesel")]
mod diesel_impl {
    use super::*;
    use ::diesel::deserialize::{self, FromSql};
    use ::diesel::serialize::{self, ToSql};
    use ::diesel::sql_types::Text;
    use ::diesel::sqlite::Sqlite;

    impl ToSql<Text, Sqlite> for Status {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
            <str as ToSql<Text, Sqlite>>::to_sql(self.as_str(), out)
        }
    }

    impl FromSql<Text, Sqlite> for Status {
        fn from_sql(bytes: <Sqlite as ::diesel::backend::Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            let s = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
            Status::from_str(&s).map_err(|e| e.into())
        }
    }
}

#[cfg(feature = "dynamodb")]
mod dynamodb_impl {
    use super::*;
    use aws_sdk_dynamodb::types::AttributeValue;

    impl From<Status> for AttributeValue {
        fn from(status: Status) -> Self {
            AttributeValue::S(status.as_str().to_string())
        }
    }

    impl From<&Status> for AttributeValue {
        fn from(status: &Status) -> Self {
            AttributeValue::S(status.as_str().to_string())
        }
    }

    impl TryFrom<&AttributeValue> for Status {
        type Error = Error;

        fn try_from(value: &AttributeValue) -> Result<Self, Self::Error> {
            match value {
                AttributeValue::S(s) => Status::from_str(s),
                _ => Err(Error::InvalidAttributeValue),
            }
        }
    }

    impl TryFrom<AttributeValue> for Status {
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
    fn test_status_str_conversion() {
        assert_eq!(Status::from_str("Active").unwrap(), Status::Active);
        assert_eq!(Status::Active.as_str(), "Active");
        assert!(Status::from_str("Unknown").is_err());
    }

    #[test]
    fn test_user_serde() {
        let user = User::new(
            Id::new(),
            Phone::new("+254712345678").unwrap(),
            "John Doe",
            Status::Active,
            DateTime::now(),
        );

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(user, deserialized);
    }
}

use super::Error;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Deref};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DateTime(chrono::DateTime<Utc>);

impl DateTime {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn parse_from_rfc3339(s: &str) -> Result<Self, Error> {
        let dt = chrono::DateTime::parse_from_rfc3339(s).map_err(|_| Error::InvalidDateTime)?;
        Ok(Self(dt.with_timezone(&Utc)))
    }

    pub fn from_timestamp(secs: i64, nsecs: u32) -> Result<Self, Error> {
        let dt = chrono::DateTime::from_timestamp(secs, nsecs).ok_or(Error::InvalidDateTime)?;
        Ok(Self(dt))
    }

    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }

    pub fn timestamp(&self) -> i64 {
        self.0.timestamp()
    }

    pub fn inner(&self) -> chrono::DateTime<Utc> {
        self.0
    }
}

impl<T> Add<T> for DateTime
where
    chrono::DateTime<Utc>: Add<T, Output = chrono::DateTime<Utc>>,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Deref for DateTime {
    type Target = chrono::DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<chrono::DateTime<Utc>> for DateTime {
    fn from(dt: chrono::DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl FromStr for DateTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_from_rfc3339(s)
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_rfc3339())
    }
}

#[cfg(feature = "diesel")]
mod diesel_impl {
    use super::*;
    use ::diesel::deserialize::{self, FromSql};
    use ::diesel::serialize::{self, ToSql};
    use ::diesel::sql_types::Text;
    use ::diesel::sqlite::Sqlite;

    impl ToSql<Text, Sqlite> for DateTime {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
            out.set_value(self.to_rfc3339());
            Ok(serialize::IsNull::No)
        }
    }

    impl FromSql<Text, Sqlite> for DateTime {
        fn from_sql(
            bytes: <Sqlite as ::diesel::backend::Backend>::RawValue<'_>,
        ) -> deserialize::Result<Self> {
            let s = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
            DateTime::parse_from_rfc3339(&s).map_err(|e| e.into())
        }
    }
}

#[cfg(feature = "dynamodb")]
mod dynamodb_impl {
    use super::*;
    use aws_sdk_dynamodb::types::AttributeValue;

    impl From<DateTime> for AttributeValue {
        fn from(dt: DateTime) -> Self {
            AttributeValue::S(dt.to_rfc3339())
        }
    }

    impl From<&DateTime> for AttributeValue {
        fn from(dt: &DateTime) -> Self {
            AttributeValue::S(dt.to_rfc3339())
        }
    }

    impl TryFrom<&AttributeValue> for DateTime {
        type Error = Error;

        fn try_from(value: &AttributeValue) -> Result<Self, Self::Error> {
            match value {
                AttributeValue::S(s) => DateTime::parse_from_rfc3339(s),
                AttributeValue::N(n) => {
                    let secs = n.parse::<i64>().map_err(|_| Error::InvalidDateTime)?;
                    DateTime::from_timestamp(secs, 0)
                }
                _ => Err(Error::InvalidAttributeValue),
            }
        }
    }

    impl TryFrom<AttributeValue> for DateTime {
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
    fn test_datetime_now_and_rfc3339() {
        let now = DateTime::now();
        let rfc = now.to_rfc3339();
        let parsed = DateTime::parse_from_rfc3339(&rfc).unwrap();
        assert_eq!(now.timestamp(), parsed.timestamp());
    }

    #[test]
    fn test_datetime_serde() {
        let now = DateTime::now();
        let json = serde_json::to_string(&now).unwrap();
        let deserialized: DateTime = serde_json::from_str(&json).unwrap();
        assert_eq!(now.timestamp(), deserialized.timestamp());
    }

    #[cfg(feature = "diesel")]
    #[test]
    fn test_sqlite_datetime_conversion() {
        use ::diesel::prelude::*;
        use ::diesel::sql_types::Text;
        use ::diesel::sqlite::SqliteConnection;
        use ::diesel::QueryableByName;

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        let now = DateTime::now();

        #[derive(QueryableByName, Debug, PartialEq, Eq)]
        struct Row {
            #[diesel(sql_type = Text)]
            val: DateTime,
        }

        let result = ::diesel::sql_query("SELECT ? as val")
            .bind::<Text, _>(&now)
            .get_result::<Row>(&mut conn)
            .unwrap();

        assert_eq!(now.timestamp(), result.val.timestamp());
    }

    #[cfg(feature = "dynamodb")]
    #[test]
    fn test_dynamodb_conversion() {
        use aws_sdk_dynamodb::types::AttributeValue;

        let dt = DateTime::now();
        let attr_s: AttributeValue = (&dt).into();
        let parsed_s = DateTime::try_from(&attr_s).unwrap();
        assert_eq!(dt.timestamp(), parsed_s.timestamp());

        let attr_n = AttributeValue::N(dt.timestamp().to_string());
        let parsed_n = DateTime::try_from(&attr_n).unwrap();
        assert_eq!(dt.timestamp(), parsed_n.timestamp());

        let invalid_attr = AttributeValue::S("not-a-date".to_string());
        assert!(DateTime::try_from(&invalid_attr).is_err());
    }
}

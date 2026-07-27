use super::{DateTime, Error, Id};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(diesel::QueryableByName))]
pub struct Session {
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Binary))]
    pub user: Id,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Binary))]
    pub id: Id,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
    pub device: String,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
    pub status: SessionStatus,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
    pub created: DateTime,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
    pub ttl: DateTime,
}

impl Session {
    pub fn new(
        user: Id,
        id: Id,
        device: impl Into<String>,
        status: SessionStatus,
        created: DateTime,
        ttl: DateTime,
    ) -> Self {
        Self {
            user,
            id,
            device: device.into(),
            status,
            created,
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.status == SessionStatus::Expired || self.ttl.timestamp() <= DateTime::now().timestamp()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
pub enum SessionStatus {
    Active,
    Revoked,
    Expired,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "Active",
            SessionStatus::Revoked => "Revoked",
            SessionStatus::Expired => "Expired",
        }
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SessionStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(SessionStatus::Active),
            "Revoked" => Ok(SessionStatus::Revoked),
            "Expired" => Ok(SessionStatus::Expired),
            _ => Err(Error::InvalidSessionStatus),
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

    impl ToSql<Text, Sqlite> for SessionStatus {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
            <str as ToSql<Text, Sqlite>>::to_sql(self.as_str(), out)
        }
    }

    impl FromSql<Text, Sqlite> for SessionStatus {
        fn from_sql(
            bytes: <Sqlite as ::diesel::backend::Backend>::RawValue<'_>,
        ) -> deserialize::Result<Self> {
            let s = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
            SessionStatus::from_str(&s).map_err(|e| e.into())
        }
    }
}

#[cfg(feature = "dynamodb")]
mod dynamodb_impl {
    use super::*;
    use aws_sdk_dynamodb::types::AttributeValue;
    use std::collections::HashMap;

    impl From<SessionStatus> for AttributeValue {
        fn from(status: SessionStatus) -> Self {
            AttributeValue::S(status.as_str().to_string())
        }
    }

    impl From<&SessionStatus> for AttributeValue {
        fn from(status: &SessionStatus) -> Self {
            AttributeValue::S(status.as_str().to_string())
        }
    }

    impl TryFrom<&AttributeValue> for SessionStatus {
        type Error = Error;

        fn try_from(value: &AttributeValue) -> Result<Self, Self::Error> {
            match value {
                AttributeValue::S(s) => SessionStatus::from_str(s),
                _ => Err(Error::InvalidAttributeValue),
            }
        }
    }

    impl TryFrom<AttributeValue> for SessionStatus {
        type Error = Error;

        fn try_from(value: AttributeValue) -> Result<Self, Self::Error> {
            Self::try_from(&value)
        }
    }

    impl From<&Session> for HashMap<String, AttributeValue> {
        fn from(s: &Session) -> Self {
            let mut map = HashMap::new();
            map.insert("user".to_string(), AttributeValue::from(&s.user));
            map.insert("id".to_string(), AttributeValue::from(&s.id));
            map.insert("device".to_string(), AttributeValue::S(s.device.clone()));
            map.insert("status".to_string(), AttributeValue::from(&s.status));
            map.insert("created".to_string(), AttributeValue::from(&s.created));
            map.insert(
                "ttl".to_string(),
                AttributeValue::N(s.ttl.timestamp().to_string()),
            );
            map
        }
    }

    impl TryFrom<&HashMap<String, AttributeValue>> for Session {
        type Error = Error;

        fn try_from(map: &HashMap<String, AttributeValue>) -> Result<Self, Self::Error> {
            let user_attr = map.get("user").ok_or(Error::InvalidAttributeValue)?;
            let id_attr = map.get("id").ok_or(Error::InvalidAttributeValue)?;
            let device_attr = map.get("device").ok_or(Error::InvalidAttributeValue)?;
            let status_attr = map.get("status").ok_or(Error::InvalidAttributeValue)?;
            let created_attr = map.get("created").ok_or(Error::InvalidAttributeValue)?;
            let ttl_attr = map.get("ttl").ok_or(Error::InvalidAttributeValue)?;

            let user = Id::try_from(user_attr)?;
            let id = Id::try_from(id_attr)?;
            let device = device_attr
                .as_s()
                .map_err(|_| Error::InvalidAttributeValue)?
                .clone();
            let status = SessionStatus::try_from(status_attr)?;
            let created = DateTime::try_from(created_attr)?;
            let ttl = DateTime::try_from(ttl_attr)?;

            Ok(Session {
                user,
                id,
                device,
                status,
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
    fn test_session_status_str_conversion() {
        assert_eq!(SessionStatus::from_str("Active").unwrap(), SessionStatus::Active);
        assert_eq!(SessionStatus::from_str("Revoked").unwrap(), SessionStatus::Revoked);
        assert_eq!(SessionStatus::from_str("Expired").unwrap(), SessionStatus::Expired);
        assert_eq!(SessionStatus::Active.as_str(), "Active");
        assert!(SessionStatus::from_str("Unknown").is_err());
    }

    #[test]
    fn test_session_serde() {
        let now = DateTime::now();
        let ttl = DateTime::from_timestamp(now.timestamp() + 3600, 0).unwrap();
        let session = Session::new(
            Id::new(),
            Id::new(),
            "Mozilla/5.0",
            SessionStatus::Active,
            now,
            ttl,
        );

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session, deserialized);
    }

    #[cfg(feature = "diesel")]
    #[test]
    fn test_sqlite_session_conversion() {
        use ::diesel::prelude::*;
        use ::diesel::sql_types::{Binary, Text};
        use ::diesel::sqlite::SqliteConnection;

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        let now = DateTime::now();
        let ttl = DateTime::from_timestamp(now.timestamp() + 3600, 0).unwrap();
        let session = Session::new(
            Id::new(),
            Id::new(),
            "EduxalClient/1.0",
            SessionStatus::Active,
            now,
            ttl,
        );

        let result = ::diesel::sql_query(
            "SELECT ? as user, ? as id, ? as device, ? as status, ? as created, ? as ttl",
        )
        .bind::<Binary, _>(&session.user)
        .bind::<Binary, _>(&session.id)
        .bind::<Text, _>(&session.device)
        .bind::<Text, _>(&session.status)
        .bind::<Text, _>(&session.created)
        .bind::<Text, _>(&session.ttl)
        .get_result::<Session>(&mut conn)
        .unwrap();

        assert_eq!(session, result);
    }

    #[cfg(feature = "dynamodb")]
    #[test]
    fn test_dynamodb_session_conversion() {
        use aws_sdk_dynamodb::types::AttributeValue;
        use std::collections::HashMap;

        let now = DateTime::now();
        let ttl = DateTime::from_timestamp(now.timestamp() + 3600, 0).unwrap();
        let session = Session::new(
            Id::new(),
            Id::new(),
            "PostmanRuntime/7.29.2",
            SessionStatus::Active,
            now,
            ttl,
        );

        let item: HashMap<String, AttributeValue> = (&session).into();
        assert_eq!(
            item.get("user").unwrap().as_s().unwrap(),
            &session.user.to_hex()
        );
        assert_eq!(
            item.get("id").unwrap().as_s().unwrap(),
            &session.id.to_hex()
        );
        assert_eq!(item.get("device").unwrap().as_s().unwrap(), "PostmanRuntime/7.29.2");
        assert_eq!(item.get("status").unwrap().as_s().unwrap(), "Active");

        let parsed = Session::try_from(&item).unwrap();
        assert_eq!(session.user, parsed.user);
        assert_eq!(session.id, parsed.id);
        assert_eq!(session.device, parsed.device);
        assert_eq!(session.status, parsed.status);
        assert_eq!(session.created.timestamp(), parsed.created.timestamp());
        assert_eq!(session.ttl.timestamp(), parsed.ttl.timestamp());
    }
}

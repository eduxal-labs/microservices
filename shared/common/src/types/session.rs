use super::{DateTime, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(diesel::QueryableByName))]
pub struct Session {
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Binary))]
    pub user: Id,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Binary))]
    pub id: Id,
    #[cfg_attr(
        feature = "diesel",
        diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)
    )]
    pub ip: Option<String>,
    #[cfg_attr(
        feature = "diesel",
        diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)
    )]
    pub user_agent: Option<String>,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
    pub created: DateTime,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
    pub expires: DateTime,
}

impl Session {
    pub fn new(
        user: Id,
        id: Id,
        ip: Option<impl Into<String>>,
        user_agent: Option<impl Into<String>>,
        created: DateTime,
        expires: DateTime,
    ) -> Self {
        Self {
            user,
            id,
            ip: ip.map(Into::into),
            user_agent: user_agent.map(Into::into),
            created,
            expires,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires.timestamp() <= DateTime::now().timestamp()
    }
}

#[cfg(feature = "dynamodb")]
mod dynamodb_impl {
    use super::*;
    use crate::types::Error;
    use aws_sdk_dynamodb::types::AttributeValue;
    use std::collections::HashMap;

    impl From<&Session> for HashMap<String, AttributeValue> {
        fn from(s: &Session) -> Self {
            let mut map = HashMap::new();
            map.insert("user".to_string(), AttributeValue::from(&s.user));
            map.insert("id".to_string(), AttributeValue::from(&s.id));

            if let Some(ref ip) = s.ip {
                map.insert("ip".to_string(), AttributeValue::S(ip.clone()));
            }

            if let Some(ref ua) = s.user_agent {
                map.insert("user_agent".to_string(), AttributeValue::S(ua.clone()));
            }

            map.insert("created".to_string(), AttributeValue::from(&s.created));
            map.insert(
                "expires".to_string(),
                AttributeValue::N(s.expires.timestamp().to_string()),
            );
            map
        }
    }

    impl TryFrom<&HashMap<String, AttributeValue>> for Session {
        type Error = Error;

        fn try_from(map: &HashMap<String, AttributeValue>) -> Result<Self, Self::Error> {
            let user_attr = map.get("user").ok_or(Error::InvalidAttributeValue)?;
            let id_attr = map.get("id").ok_or(Error::InvalidAttributeValue)?;
            let created_attr = map.get("created").ok_or(Error::InvalidAttributeValue)?;
            let expires_attr = map.get("expires").ok_or(Error::InvalidAttributeValue)?;

            let user = Id::try_from(user_attr)?;
            let id = Id::try_from(id_attr)?;
            let created = DateTime::try_from(created_attr)?;
            let expires = DateTime::try_from(expires_attr)?;

            let ip = match map.get("ip") {
                Some(AttributeValue::S(s)) => Some(s.clone()),
                _ => None,
            };

            let user_agent = match map.get("user_agent") {
                Some(AttributeValue::S(s)) => Some(s.clone()),
                _ => None,
            };

            Ok(Session {
                user,
                id,
                ip,
                user_agent,
                created,
                expires,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_serde() {
        let now = DateTime::now();
        let expires = DateTime::from_timestamp(now.timestamp() + 3600, 0).unwrap();
        let session = Session::new(
            Id::new(),
            Id::new(),
            Some("127.0.0.1"),
            Some("Mozilla/5.0"),
            now,
            expires,
        );

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session, deserialized);
    }

    #[cfg(feature = "diesel")]
    #[test]
    fn test_sqlite_session_conversion() {
        use ::diesel::prelude::*;
        use ::diesel::sql_types::{Binary, Nullable, Text};
        use ::diesel::sqlite::SqliteConnection;

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        let now = DateTime::now();
        let expires = DateTime::from_timestamp(now.timestamp() + 3600, 0).unwrap();
        let session = Session::new(
            Id::new(),
            Id::new(),
            Some("192.168.1.1"),
            Some("EduxalClient/1.0"),
            now,
            expires,
        );

        let result = ::diesel::sql_query(
            "SELECT ? as user, ? as id, ? as ip, ? as user_agent, ? as created, ? as expires",
        )
        .bind::<Binary, _>(&session.user)
        .bind::<Binary, _>(&session.id)
        .bind::<Nullable<Text>, _>(&session.ip)
        .bind::<Nullable<Text>, _>(&session.user_agent)
        .bind::<Text, _>(&session.created)
        .bind::<Text, _>(&session.expires)
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
        let expires = DateTime::from_timestamp(now.timestamp() + 3600, 0).unwrap();
        let session = Session::new(
            Id::new(),
            Id::new(),
            Some("10.0.0.1"),
            Some("PostmanRuntime/7.29.2"),
            now,
            expires,
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
        assert_eq!(item.get("ip").unwrap().as_s().unwrap(), "10.0.0.1");

        let parsed = Session::try_from(&item).unwrap();
        assert_eq!(session, parsed);
    }
}

use super::{DateTime, Id};
use serde::{Deserialize, Serialize};

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
    pub created: DateTime,
    #[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
    pub ttl: DateTime,
}

impl Session {
    pub fn new(
        user: Id,
        id: Id,
        device: impl Into<String>,
        created: DateTime,
        ttl: DateTime,
    ) -> Self {
        Self {
            user,
            id,
            device: device.into(),
            created,
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.ttl.timestamp() <= DateTime::now().timestamp()
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
            map.insert("device".to_string(), AttributeValue::S(s.device.clone()));
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
            let created_attr = map.get("created").ok_or(Error::InvalidAttributeValue)?;
            let ttl_attr = map.get("ttl").ok_or(Error::InvalidAttributeValue)?;

            let user = Id::try_from(user_attr)?;
            let id = Id::try_from(id_attr)?;
            let device = device_attr
                .as_s()
                .map_err(|_| Error::InvalidAttributeValue)?
                .clone();
            let created = DateTime::try_from(created_attr)?;
            let ttl = DateTime::try_from(ttl_attr)?;

            Ok(Session {
                user,
                id,
                device,
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
    fn test_session_serde() {
        let now = DateTime::now();
        let ttl = DateTime::from_timestamp(now.timestamp() + 3600, 0).unwrap();
        let session = Session::new(Id::new(), Id::new(), "Mozilla/5.0", now, ttl);

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
        let session = Session::new(Id::new(), Id::new(), "EduxalClient/1.0", now, ttl);

        let result = ::diesel::sql_query(
            "SELECT ? as user, ? as id, ? as device, ? as created, ? as ttl",
        )
        .bind::<Binary, _>(&session.user)
        .bind::<Binary, _>(&session.id)
        .bind::<Text, _>(&session.device)
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
        let session = Session::new(Id::new(), Id::new(), "PostmanRuntime/7.29.2", now, ttl);

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

        let parsed = Session::try_from(&item).unwrap();
        assert_eq!(session.user, parsed.user);
        assert_eq!(session.id, parsed.id);
        assert_eq!(session.device, parsed.device);
        assert_eq!(session.created.timestamp(), parsed.created.timestamp());
        assert_eq!(session.ttl.timestamp(), parsed.ttl.timestamp());
    }
}

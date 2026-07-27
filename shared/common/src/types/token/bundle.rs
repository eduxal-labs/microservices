use super::{Access, Refresh, Token};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub access: Token<Access>,
    pub refresh: Token<Refresh>,
}

impl Bundle {
    pub fn new(access: Token<Access>, refresh: Token<Refresh>) -> Self {
        Self { access, refresh }
    }
}

impl Serialize for Bundle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let access_str = self
            .access
            .tokenize()
            .map_err(|_| serde::ser::Error::custom("failed to tokenize access token"))?;
        let refresh_str = self
            .refresh
            .tokenize()
            .map_err(|_| serde::ser::Error::custom("failed to tokenize refresh token"))?;

        let mut state = serializer.serialize_struct("Bundle", 2)?;
        state.serialize_field("access", &access_str)?;
        state.serialize_field("refresh", &refresh_str)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Bundle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Access,
            Refresh,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`access` or `refresh`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "access" => Ok(Field::Access),
                            "refresh" => Ok(Field::Refresh),
                            _ => Err(de::Error::unknown_field(value, &["access", "refresh"])),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct BundleVisitor;

        impl<'de> Visitor<'de> for BundleVisitor {
            type Value = Bundle;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Bundle")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Bundle, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut access: Option<Token<Access>> = None;
                let mut refresh: Option<Token<Refresh>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Access => {
                            if access.is_some() {
                                return Err(de::Error::duplicate_field("access"));
                            }
                            access = Some(map.next_value()?);
                        }
                        Field::Refresh => {
                            if refresh.is_some() {
                                return Err(de::Error::duplicate_field("refresh"));
                            }
                            refresh = Some(map.next_value()?);
                        }
                    }
                }

                let access = access.ok_or_else(|| de::Error::missing_field("access"))?;
                let refresh = refresh.ok_or_else(|| de::Error::missing_field("refresh"))?;

                Ok(Bundle { access, refresh })
            }
        }

        const FIELDS: &[&str] = &["access", "refresh"];
        deserializer.deserialize_struct("Bundle", FIELDS, BundleVisitor)
    }
}

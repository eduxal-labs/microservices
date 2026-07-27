#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    #[error("invalid id")]
    InvalidId,

    #[error("invalid phone number: must be in E.164 country code format (e.g., +254712345678)")]
    InvalidPhone,

    #[error("invalid datetime format")]
    InvalidDateTime,

    #[cfg(feature = "dynamodb")]
    #[error("invalid dynamodb attribute type")]
    InvalidAttributeValue,
}

impl From<bson::oid::Error> for Error {
    fn from(_: bson::oid::Error) -> Self {
        Error::InvalidId
    }
}

impl From<chrono::ParseError> for Error {
    fn from(_: chrono::ParseError) -> Self {
        Error::InvalidDateTime
    }
}

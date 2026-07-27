#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid id")]
    InvalidId,
    #[cfg(feature = "dynamodb")]
    #[error("invalid dynamodb attribute type")]
    InvalidAttributeValue,
}

impl From<bson::oid::Error> for Error {
    fn from(_: bson::oid::Error) -> Self {
        Error::InvalidId
    }
}

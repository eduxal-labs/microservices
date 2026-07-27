#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid id")]
    InvalidId,
}

impl From<bson::oid::Error> for Error {
    fn from(_: bson::oid::Error) -> Self {
        Error::InvalidId
    }
}

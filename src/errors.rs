use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while deserializing the document: {0}")]
    Deserialize(#[from] serde_yaml::Error),
    #[error("{0}")]
    Format(#[from] std::fmt::Error),
    #[error("{0}")]
    Custom(String),
}

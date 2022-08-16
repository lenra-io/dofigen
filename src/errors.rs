use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while deserializing the JSON document: {0}")]
    DeserializeJson(#[from] serde_json::Error),
    #[error("Error while deserializing the YAML document: {0}")]
    DeserializeYaml(#[from] serde_yaml::Error),
    #[error("{0}")]
    Custom(String),
}

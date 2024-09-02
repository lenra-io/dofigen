use std::fmt::Display;

use serde_yaml::Location;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while deserializing the document{}: {0}", location_into(.0.location()))]
    Deserialize(#[from] serde_yaml::Error),
    #[error("Error while parsing: {0}")]
    ParseFromStr(#[from] serde::de::value::Error),
    #[error("{0}")]
    Format(#[from] std::fmt::Error),
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub fn display<S: Display>(error: S) -> Self {
        Self::Custom(format!("{}", error))
    }
}

fn location_into(location: Option<Location>) -> String {
    location
        .map(|location| format!(" at line {}, column {}", location.line(), location.column()))
        .unwrap_or_else(|| "".into())
}

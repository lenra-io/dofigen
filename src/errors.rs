use std::fmt::Display;

use serde_yaml::Location;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while deserializing the document{loc}: {0}", loc = location_into(.0.location()))]
    Deserialize(#[from] serde_yaml::Error),
    #[cfg(feature = "serde_json")]
    #[error("Error while deserializing JSON: {0}")]
    DeserializeJSON(#[from] serde_json::Error),
    #[error("Error while parsing: {0}")]
    ParseFromStr(#[from] serde::de::value::Error),
    #[error("Error while parsing bool value: {0}")]
    ParseBool(#[from] std::str::ParseBoolError),
    #[error("{0}")]
    Format(#[from] std::fmt::Error),
    #[error("{e}", e = report(.0))]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    Regex(#[from] regex::Error),
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

fn report(mut err: &dyn std::error::Error) -> String {
    let mut s = format!("{}", err);
    while let Some(src) = err.source() {
        s.push_str(format!("\n\tCaused by: {}", src).as_str());
        err = src;
    }
    s
}

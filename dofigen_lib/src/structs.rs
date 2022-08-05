use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/** Represents the Dockerfile main stage */
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Image {
    // Common part
    pub image: String,
    pub user: Option<String>,
    pub workdir: Option<String>,
    pub envs: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    pub adds: Option<Vec<String>>,
    pub root: Option<Root>,
    pub script: Option<Vec<String>>,
    pub caches: Option<Vec<String>>,
    // Specific part
    pub builders: Option<Vec<Builder>>,
    pub ignores: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub ports: Option<Vec<u16>>,
    pub healthcheck: Option<Healthcheck>,
}

/** Represents a Dockerfile builder stage */
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Builder {
    // Common part
    pub image: String,
    pub user: Option<String>,
    pub workdir: Option<String>,
    pub envs: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    pub adds: Option<Vec<String>>,
    pub root: Option<Root>,
    pub script: Option<Vec<String>>,
    pub caches: Option<Vec<String>>,
    // Specific part
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Artifact {
    pub builder: String,
    pub source: String,
    pub destination: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Root {
    pub script: Option<Vec<String>>,
    pub caches: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Healthcheck {
    pub cmd: Option<String>,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub start: Option<String>,
    pub retries: Option<u16>,
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/** Represents the Dockerfile main stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Image {
    // Common part
    #[serde(alias = "from")]
    pub image: String,
    pub user: Option<String>,
    pub workdir: Option<String>,
    #[serde(alias = "env")]
    pub envs: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add")]
    pub adds: Option<Vec<String>>,
    pub root: Option<Root>,
    #[serde(alias = "run")]
    pub script: Option<Vec<String>>,
    #[serde(alias = "cache")]
    pub caches: Option<Vec<String>>,
    // Specific part
    pub builders: Option<Vec<Builder>>,
    #[serde(alias = "ignore")]
    pub ignores: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub ports: Option<Vec<u16>>,
    pub healthcheck: Option<Healthcheck>,
}

/** Represents a Dockerfile builder stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Builder {
    // Common part
    #[serde(alias = "from")]
    pub image: String,
    pub user: Option<String>,
    pub workdir: Option<String>,
    #[serde(alias = "env")]
    pub envs: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add")]
    pub adds: Option<Vec<String>>,
    pub root: Option<Root>,
    #[serde(alias = "run")]
    pub script: Option<Vec<String>>,
    #[serde(alias = "cache")]
    pub caches: Option<Vec<String>>,
    // Specific part
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Artifact {
    pub builder: String,
    pub source: String,
    #[serde(alias = "target")]
    pub destination: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Root {
    #[serde(alias = "run")]
    pub script: Option<Vec<String>>,
    #[serde(alias = "cache")]
    pub caches: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Healthcheck {
    pub cmd: String,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub start: Option<String>,
    pub retries: Option<u16>,
}

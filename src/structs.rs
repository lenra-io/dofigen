#[cfg(feature = "json_schema")]
use schemars::JsonSchema;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Vec(Vec<T>),
}

impl<T> OneOrMany<T> {
    pub fn to_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(t) => vec![t],
            OneOrMany::Vec(v) => v,
        }
    }
}

impl From<Vec<String>> for OneOrMany<String> {
    fn from(v: Vec<String>) -> Self {
        OneOrMany::Vec(v)
    }
}

impl<T> Into<Vec<T>> for OneOrMany<T> {
    fn into(self) -> Vec<T> {
        match self {
            OneOrMany::One(t) => vec![t],
            OneOrMany::Vec(v) => v,
        }
    }
}

impl<T> Default for OneOrMany<T> {
    fn default() -> Self {
        OneOrMany::Vec(Vec::new())
    }
}

/** Represents the Dockerfile main stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Image {
    // Common part
    #[serde(alias = "image")]
    pub from: Option<ImageName>,
    pub user: Option<String>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: Option<OneOrMany<CopyResources>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Option<OneOrMany<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<OneOrMany<String>>,
    // Specific part
    pub builders: Option<Vec<Builder>>,
    pub context: Option<OneOrMany<String>>,
    #[serde(alias = "ignores")]
    pub ignore: Option<OneOrMany<String>>,
    pub entrypoint: Option<OneOrMany<String>>,
    pub cmd: Option<OneOrMany<String>>,
    #[serde(alias = "ports")]
    pub expose: Option<OneOrMany<u16>>,
    pub healthcheck: Option<Healthcheck>,
}

/** Represents a Dockerfile builder stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Builder {
    // Common part
    #[serde(alias = "image")]
    pub from: ImageName,
    pub user: Option<String>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: Option<OneOrMany<CopyResources>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Option<OneOrMany<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<OneOrMany<String>>,
    // Specific part
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Artifact {
    pub builder: String,
    pub source: String,
    #[serde(alias = "destination")]
    pub target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Root {
    #[serde(alias = "script")]
    pub run: Option<OneOrMany<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<OneOrMany<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Healthcheck {
    pub cmd: String,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub start: Option<String>,
    pub retries: Option<u16>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct ImageName {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub path: String,
    pub version: Option<ImageVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum ImageVersion {
    Tag(String),
    Digest(String),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum CopyResources {
    Copy(Copy),
    AddGitRepo(AddGitRepo),
    AddSimple(AddSimple),
}

/// Represents the COPY instruction in a Dockerfile.
/// See https://docs.docker.com/reference/dockerfile/#copy
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Copy {
    pub paths: OneOrMany<String>,
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<Chown>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: Option<OneOrMany<String>>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: Option<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---parents
    pub parents: Option<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    pub from: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct AddGitRepo {
    pub repo: String,
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<Chown>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: Option<OneOrMany<String>>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: Option<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir
    pub keep_git_dir: Option<bool>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct AddSimple {
    pub urls: OneOrMany<String>,
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<Chown>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Chown {
    pub user: String,
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum GitRepo {
    Http(HttpGitRepo),
    Ssh(SshGitRepo),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct HttpGitRepo {
    pub url: String,
    /// The branch or tag to checkout
    pub reference: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct SshGitRepo {
    pub url: String,
    #[serde(default = "default_sshgitrepo_user")]
    pub user: String,
}

fn default_sshgitrepo_user() -> String {
    "git".to_string()
}

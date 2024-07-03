#[cfg(feature = "json_schema")]
use schemars::JsonSchema;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::serde_permissive::{StringOrStruct, deserialize_optional_one_or_many};

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
    #[serde(alias = "add", alias = "adds", deserialize_with = "deserialize_optional_one_or_many", default)]
    pub copy: Option<Vec<CopyResources>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Option<Vec<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<Vec<String>>,
    // Specific part
    pub builders: Option<Vec<Builder>>,
    pub context: Option<Vec<String>>,
    #[serde(alias = "ignores")]
    pub ignore: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    #[serde(alias = "ports")]
    pub expose: Option<Vec<u16>>,
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
    pub copy: Option<Vec<CopyResources>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Option<Vec<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<Vec<String>>,
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
    pub run: Option<Vec<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<Vec<String>>,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(from = "StringOrStruct<ImageName>")]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(from = "StringOrStruct<CopyResources>")]
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
    pub paths: Vec<String>,
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<Chown>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: Option<Vec<String>>,
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
    pub exclude: Option<Vec<String>>,
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
    pub urls: Vec<String>,
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

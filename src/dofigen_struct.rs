#[cfg(feature = "permissive")]
use crate::serde_permissive::{
    deserialize_one_or_many, deserialize_optional_one_or_many, PermissiveStruct,
};
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/** Represents the Dockerfile main stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Image {
    // Common part
    #[serde(alias = "image")]
    pub from: Option<ImageName>,
    pub user: Option<User>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub copy: Option<Vec<CopyResource>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub run: Option<Vec<String>>,
    #[serde(alias = "caches")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub cache: Option<Vec<String>>,
    // Specific part
    pub builders: Option<Vec<Builder>>,
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub context: Option<Vec<String>>,
    #[serde(alias = "ignores")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub ignore: Option<Vec<String>>,
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub entrypoint: Option<Vec<String>>,
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub cmd: Option<Vec<String>>,
    #[serde(alias = "port", alias = "ports")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub expose: Option<Vec<Port>>,
    pub healthcheck: Option<Healthcheck>,
}

/** Represents a Dockerfile builder stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Builder {
    // Common part
    #[serde(alias = "image")]
    pub from: ImageName,
    pub user: Option<User>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub copy: Option<Vec<CopyResource>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub run: Option<Vec<String>>,
    #[serde(alias = "caches")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
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
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub run: Option<Vec<String>>,
    #[serde(alias = "caches")]
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
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
#[cfg_attr(feature = "permissive", serde(from = "PermissiveStruct<ImageName>"))]
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
#[serde(untagged)]
#[cfg_attr(feature = "permissive", serde(from = "PermissiveStruct<CopyResource>"))]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum CopyResource {
    Copy(Copy),
    AddGitRepo(AddGitRepo),
    Add(Add),
}

/// Represents the COPY instruction in a Dockerfile.
/// See https://docs.docker.com/reference/dockerfile/#copy
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Copy {
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_one_or_many", default)
    )]
    pub paths: Vec<String>,
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<User>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
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
    // pub repo: GitRepo,
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<User>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_optional_one_or_many", default)
    )]
    pub exclude: Option<Vec<String>>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: Option<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir
    pub keep_git_dir: Option<bool>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Add {
    #[cfg_attr(
        feature = "permissive",
        serde(deserialize_with = "deserialize_one_or_many", default)
    )]
    pub files: Vec<String>,
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#add---checksum
    pub checksum: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<User>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "permissive", serde(from = "PermissiveStruct<User>"))]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct User {
    pub user: String,
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "permissive", serde(from = "PermissiveStruct<Port>"))]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Port {
    pub port: u16,
    pub protocol: Option<PortProtocol>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum PortProtocol {
    Tcp,
    Udp,
}

// #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
// #[cfg_attr(feature = "json_schema", derive(JsonSchema))]
// pub enum GitRepo {
//     Http(HttpGitRepo),
//     Ssh(SshGitRepo),
// }

// #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
// #[cfg_attr(feature = "json_schema", derive(JsonSchema))]
// pub struct HttpGitRepo {
//     pub url: String,
//     /// The branch or tag to checkout
//     pub reference: Option<String>,
// }

// #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
// #[cfg_attr(feature = "json_schema", derive(JsonSchema))]
// pub struct SshGitRepo {
//     pub url: String,
//     pub user: String,
// }
#[cfg(test)]
mod test {
    use super::*;

    mod copy_resource {
        use super::*;

        #[test]
        fn deserialize_copy() {
            let json_data = r#"{
            "paths": ["file1.txt", "file2.txt"],
            "target": "destination/",
            "chown": {
                "user": "root",
                "group": "root"
            },
            "chmod": "755",
            "exclude": ["file3.txt"],
            "link": true,
            "parents": true,
            "from": "source/"
        }"#;

            let copy_resource: CopyResource = serde_yaml::from_str(json_data).unwrap();

            assert_eq!(
                copy_resource,
                CopyResource::Copy(Copy {
                    paths: vec!["file1.txt".to_string(), "file2.txt".to_string()],
                    target: Some("destination/".to_string()),
                    chown: Some(User {
                        user: "root".to_string(),
                        group: Some("root".to_string())
                    }),
                    chmod: Some("755".to_string()),
                    exclude: Some(vec!["file3.txt".to_string()]),
                    link: Some(true),
                    parents: Some(true),
                    from: Some("source/".to_string())
                })
            );
        }

        #[cfg(feature = "permissive")]
        #[test]
        fn deserialize_copy_from_str() {
            let json_data = "file1.txt destination/";

            let copy_resource: CopyResource = serde_yaml::from_str(json_data).unwrap();

            assert_eq!(
                copy_resource,
                CopyResource::Copy(Copy {
                    paths: vec!["file1.txt".to_string()],
                    target: Some("destination/".to_string()),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn deserialize_add_git_repo() {
            let json_data = r#"{
            "repo": "https://github.com/example/repo.git",
            "target": "destination/",
            "chown": {
                "user": "root",
                "group": "root"
            },
            "chmod": "755",
            "exclude": ["file3.txt"],
            "link": true,
            "keep_git_dir": true
        }"#;

            let copy_resource: CopyResource = serde_yaml::from_str(json_data).unwrap();

            assert_eq!(
                copy_resource,
                CopyResource::AddGitRepo(AddGitRepo {
                    repo: "https://github.com/example/repo.git".to_string(),
                    target: Some("destination/".to_string()),
                    chown: Some(User {
                        user: "root".to_string(),
                        group: Some("root".to_string())
                    }),
                    chmod: Some("755".to_string()),
                    exclude: Some(vec!["file3.txt".to_string()]),
                    link: Some(true),
                    keep_git_dir: Some(true)
                })
            );
        }

        #[test]
        fn deserialize_add() {
            let json_data = r#"{
            "files": ["file1.txt", "file2.txt"],
            "target": "destination/",
            "checksum": "sha256:abcdef123456",
            "chown": {
                "user": "root",
                "group": "root"
            },
            "chmod": "755",
            "link": true
        }"#;

            let copy_resource: CopyResource = serde_yaml::from_str(json_data).unwrap();

            assert_eq!(
                copy_resource,
                CopyResource::Add(Add {
                    files: vec!["file1.txt".to_string(), "file2.txt".to_string()],
                    target: Some("destination/".to_string()),
                    checksum: Some("sha256:abcdef123456".to_string()),
                    chown: Some(User {
                        user: "root".to_string(),
                        group: Some("root".to_string())
                    }),
                    chmod: Some("755".to_string()),
                    link: Some(true)
                })
            );
        }
    }
}

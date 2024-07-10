#[cfg(feature = "permissive")]
use crate::serde_permissive::{OneOrManyVec, ParsableStruct};
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "permissive")]
pub type PermissiveStruct<T> = ParsableStruct<T>;
#[cfg(not(feature = "permissive"))]
pub type PermissiveStruct<T> = Box<T>;

#[cfg(feature = "permissive")]
pub type PermissiveVec<T> = OneOrManyVec<T>;
#[cfg(not(feature = "permissive"))]
pub type PermissiveVec<T> = Box<Vec<T>>;

/** Represents the Dockerfile main stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Image {
    // Common part
    #[serde(alias = "image")]
    pub from: Option<PermissiveStruct<ImageName>>,
    pub user: Option<PermissiveStruct<User>>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: Option<PermissiveVec<PermissiveStruct<CopyResource>>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Option<PermissiveVec<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<PermissiveVec<String>>,
    // Specific part
    #[serde(alias = "extends", default)]
    pub extend: Option<PermissiveVec<String>>,
    pub builders: Option<Vec<Builder>>,
    pub context: Option<PermissiveVec<String>>,
    #[serde(alias = "ignores")]
    pub ignore: Option<PermissiveVec<String>>,
    pub entrypoint: Option<PermissiveVec<String>>,
    pub cmd: Option<PermissiveVec<String>>,
    #[serde(alias = "port", alias = "ports")]
    pub expose: Option<PermissiveVec<PermissiveStruct<Port>>>,
    pub healthcheck: Option<Healthcheck>,
}

/** Represents a Dockerfile builder stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Builder {
    // Common part
    #[serde(alias = "image")]
    pub from: Option<PermissiveStruct<ImageName>>,
    pub user: Option<PermissiveStruct<User>>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: Option<PermissiveVec<PermissiveStruct<CopyResource>>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Option<PermissiveVec<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<PermissiveVec<String>>,
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
    pub run: Option<PermissiveVec<String>>,
    #[serde(alias = "caches")]
    pub cache: Option<PermissiveVec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Healthcheck {
    pub cmd: Option<String>,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub start: Option<String>,
    pub retries: Option<u16>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct ImageName {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub version: Option<ImageVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum ImageVersion {
    Tag(String),
    Digest(String),
}

#[derive(Serialize, Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum CopyResource {
    Copy(Copy),
    AddGitRepo(AddGitRepo),
    Add(Add),
}

/// Represents the COPY instruction in a Dockerfile.
/// See https://docs.docker.com/reference/dockerfile/#copy
#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Copy {
    pub paths: Option<PermissiveVec<String>>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: Option<PermissiveVec<String>>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---parents
    pub parents: Option<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    pub from: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct AddGitRepo {
    pub repo: Option<String>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: Option<PermissiveVec<String>>,
    /// See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir
    pub keep_git_dir: Option<bool>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Add {
    pub files: Option<PermissiveVec<String>>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#add---checksum
    pub checksum: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct CopyOptions {
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: Option<User>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: Option<bool>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct User {
    pub user: Option<String>,
    pub group: Option<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Port {
    pub port: Option<u16>,
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

    mod deserialize {
        use super::*;

        mod user {
            use super::*;

            #[test]
            fn name_and_group() {
                let json_data = r#"{
    "user": "test",
    "group": "test"
}"#;

                let user: User = serde_yaml::from_str(json_data).unwrap();

                assert_eq!(
                    user,
                    User {
                        user: Some("test".into()),
                        group: Some("test".into())
                    }
                );
            }
        }

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
                        paths: Some(vec!["file1.txt".into(), "file2.txt".into()].into()),
                        options: CopyOptions {
                            target: Some("destination/".into()),
                            chown: Some(User {
                                user: Some("root".into()),
                                group: Some("root".into())
                            }),
                            chmod: Some("755".into()),
                            link: Some(true),
                        },
                        exclude: Some(vec!["file3.txt".into()].into()),
                        parents: Some(true),
                        from: Some("source/".into())
                    })
                );
            }

            #[cfg(feature = "permissive")]
            #[test]
            fn deserialize_copy_from_str() {
                use std::ops::Deref;

                let json_data = "file1.txt destination/";

                let copy_resource: PermissiveStruct<CopyResource> =
                    serde_yaml::from_str(json_data).unwrap();

                assert_eq!(
                    copy_resource.deref(),
                    &CopyResource::Copy(Copy {
                        paths: Some(vec!["file1.txt".into()].into()),
                        options: CopyOptions {
                            target: Some("destination/".into()),
                            ..Default::default()
                        },
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
                        repo: Some("https://github.com/example/repo.git".into()),
                        options: CopyOptions {
                            target: Some("destination/".into()),
                            chown: Some(User {
                                user: Some("root".into()),
                                group: Some("root".into())
                            }),
                            chmod: Some("755".into()),
                            link: Some(true),
                        },
                        exclude: Some(vec!["file3.txt".into()].into()),
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
                        files: Some(vec!["file1.txt".into(), "file2.txt".into()].into()),
                        options: CopyOptions {
                            target: Some("destination/".into()),
                            chown: Some(User {
                                user: Some("root".into()),
                                group: Some("root".into())
                            }),
                            chmod: Some("755".into()),
                            link: Some(true),
                        },
                        checksum: Some("sha256:abcdef123456".into()),
                    })
                );
            }
        }
    }
}

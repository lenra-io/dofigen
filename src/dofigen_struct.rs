use crate::merge::OptionalField;
#[cfg(feature = "permissive")]
use crate::serde_permissive::{OneOrManyVec, ParsableStruct};
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use url::Url;

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
    pub from: OptionalField<PermissiveStruct<ImageName>>,
    pub user: OptionalField<PermissiveStruct<User>>,
    pub workdir: OptionalField<String>,
    #[serde(alias = "envs")]
    pub env: OptionalField<HashMap<String, String>>,
    pub artifacts: OptionalField<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: OptionalField<PermissiveVec<PermissiveStruct<CopyResource>>>,
    pub root: OptionalField<Root>,
    #[serde(alias = "script")]
    pub run: OptionalField<PermissiveVec<String>>,
    #[serde(alias = "caches")]
    pub cache: OptionalField<PermissiveVec<String>>,
    // Specific part
    #[serde(alias = "extends")]
    pub extend: OptionalField<PermissiveVec<Resource>>,
    pub builders: OptionalField<Vec<Builder>>,
    pub context: OptionalField<PermissiveVec<String>>,
    #[serde(alias = "ignores")]
    pub ignore: OptionalField<PermissiveVec<String>>,
    pub entrypoint: OptionalField<PermissiveVec<String>>,
    pub cmd: OptionalField<PermissiveVec<String>>,
    #[serde(alias = "port", alias = "ports")]
    pub expose: OptionalField<PermissiveVec<PermissiveStruct<Port>>>,
    pub healthcheck: OptionalField<Healthcheck>,
}

/** Represents a Dockerfile builder stage */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Builder {
    // Common part
    #[serde(alias = "image")]
    pub from: OptionalField<PermissiveStruct<ImageName>>,
    pub user: OptionalField<PermissiveStruct<User>>,
    pub workdir: OptionalField<String>,
    #[serde(alias = "envs")]
    pub env: OptionalField<HashMap<String, String>>,
    pub artifacts: OptionalField<Vec<Artifact>>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: OptionalField<PermissiveVec<PermissiveStruct<CopyResource>>>,
    pub root: OptionalField<Root>,
    #[serde(alias = "script")]
    pub run: OptionalField<PermissiveVec<String>>,
    #[serde(alias = "caches")]
    pub cache: OptionalField<PermissiveVec<String>>,
    // Specific part
    pub name: OptionalField<String>,
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
    pub run: OptionalField<PermissiveVec<String>>,
    #[serde(alias = "caches")]
    pub cache: OptionalField<PermissiveVec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Healthcheck {
    pub cmd: OptionalField<String>,
    pub interval: OptionalField<String>,
    pub timeout: OptionalField<String>,
    pub start: OptionalField<String>,
    pub retries: OptionalField<u16>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct ImageName {
    pub host: OptionalField<String>,
    pub port: OptionalField<u16>,
    pub path: OptionalField<String>,
    pub version: OptionalField<ImageVersion>,
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
    pub paths: OptionalField<PermissiveVec<String>>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: OptionalField<PermissiveVec<String>>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---parents
    pub parents: OptionalField<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    pub from: OptionalField<String>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct AddGitRepo {
    pub repo: OptionalField<String>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: OptionalField<PermissiveVec<String>>,
    /// See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir
    pub keep_git_dir: OptionalField<bool>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Add {
    pub files: OptionalField<PermissiveVec<Resource>>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#add---checksum
    pub checksum: OptionalField<String>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct CopyOptions {
    pub target: OptionalField<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chown: OptionalField<User>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: OptionalField<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: OptionalField<bool>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct User {
    pub user: OptionalField<String>,
    pub group: OptionalField<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default, Deserialize)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Port {
    pub port: OptionalField<u16>,
    pub protocol: OptionalField<PortProtocol>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
// #[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum Resource {
    File(PathBuf),
    Url(Url),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum GitRepo {
    Http(Url),
    Ssh(SshGitRepo),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct SshGitRepo {
    pub user: String,
    pub host: String,
    pub path: String,
}

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
                        user: OptionalField::Present("test".into()),
                        group: OptionalField::Present("test".into())
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
                        paths: OptionalField::Present(vec!["file1.txt".into(), "file2.txt".into()].into()),
                        options: CopyOptions {
                            target: OptionalField::Present("destination/".into()),
                            chown: OptionalField::Present(User {
                                user: OptionalField::Present("root".into()),
                                group: OptionalField::Present("root".into())
                            }),
                            chmod: OptionalField::Present("755".into()),
                            link: OptionalField::Present(true),
                        },
                        exclude: OptionalField::Present(vec!["file3.txt".into()].into()),
                        parents: OptionalField::Present(true),
                        from: OptionalField::Present("source/".into())
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
                        paths: OptionalField::Present(vec!["file1.txt".into()].into()),
                        options: CopyOptions {
                            target: OptionalField::Present("destination/".into()),
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
                        repo: OptionalField::Present("https://github.com/example/repo.git".into()),
                        options: CopyOptions {
                            target: OptionalField::Present("destination/".into()),
                            chown: OptionalField::Present(User {
                                user: OptionalField::Present("root".into()),
                                group: OptionalField::Present("root".into())
                            }),
                            chmod: OptionalField::Present("755".into()),
                            link: OptionalField::Present(true),
                        },
                        exclude: OptionalField::Present(vec!["file3.txt".into()].into()),
                        keep_git_dir: OptionalField::Present(true)
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
                        files: OptionalField::Present(
                            vec![
                                Resource::File("file1.txt".into()),
                                Resource::File("file2.txt".into())
                            ]
                            .into()
                        ),
                        options: CopyOptions {
                            target: OptionalField::Present("destination/".into()),
                            chown: OptionalField::Present(User {
                                user: OptionalField::Present("root".into()),
                                group: OptionalField::Present("root".into())
                            }),
                            chmod: OptionalField::Present("755".into()),
                            link: OptionalField::Present(true),
                        },
                        checksum: OptionalField::Present("sha256:abcdef123456".into()),
                    })
                );
            }
        }
    }
}

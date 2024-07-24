#[cfg(feature = "permissive")]
use crate::serde_permissive::{OneOrManyVec as Vec, ParsableStruct};
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};
use struct_patch::Patch;
use url::Url;

#[cfg(feature = "permissive")]
pub type PermissiveStruct<T> = ParsableStruct<T>;
#[cfg(not(feature = "permissive"))]
pub type PermissiveStruct<T> = Box<T>;

/** Represents the Dockerfile main stage */
#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct Image {
    // Common part
    #[serde(alias = "image")]
    pub from: Option<PermissiveStruct<ImageName>>,
    pub user: Option<PermissiveStruct<User>>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Vec<Artifact>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: Vec<PermissiveStruct<CopyResource>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Vec<String>,
    #[serde(alias = "caches")]
    pub cache: Vec<String>,
    // Specific part
    #[serde(alias = "extends", default)]
    pub extend: Vec<Resource>,
    pub builders: Vec<Builder>,
    pub context: Vec<String>,
    #[serde(alias = "ignores")]
    pub ignore: Vec<String>,
    pub entrypoint: Vec<String>,
    pub cmd: Vec<String>,
    #[serde(alias = "port", alias = "ports")]
    pub expose: Vec<PermissiveStruct<Port>>,
    pub healthcheck: Option<Healthcheck>,
}

/** Represents a Dockerfile builder stage */
#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct Builder {
    // Common part
    #[serde(alias = "image")]
    pub from: PermissiveStruct<ImageName>,
    pub user: Option<PermissiveStruct<User>>,
    pub workdir: Option<String>,
    #[serde(alias = "envs")]
    pub env: Option<HashMap<String, String>>,
    pub artifacts: Vec<Artifact>,
    #[serde(alias = "add", alias = "adds")]
    pub copy: Vec<PermissiveStruct<CopyResource>>,
    pub root: Option<Root>,
    #[serde(alias = "script")]
    pub run: Vec<String>,
    #[serde(alias = "caches")]
    pub cache: Vec<String>,
    // Specific part
    pub name: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct Artifact {
    pub builder: String,
    pub source: String,
    #[serde(alias = "destination")]
    pub target: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct Root {
    #[serde(alias = "script")]
    pub run: Vec<String>,
    #[serde(alias = "caches")]
    pub cache: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct Healthcheck {
    pub cmd: String,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub start: Option<String>,
    pub retries: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct ImageName {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub path: String,
    pub version: Option<ImageVersion>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum ImageVersion {
    Tag(String),
    Digest(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum CopyResource {
    Copy(Copy),
    AddGitRepo(AddGitRepo),
    Add(Add),
}

/// Represents the COPY instruction in a Dockerfile.
/// See https://docs.docker.com/reference/dockerfile/#copy
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Copy {
    pub paths: Vec<String>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: Vec<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---parents
    pub parents: Option<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    pub from: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct AddGitRepo {
    pub repo: String,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    pub exclude: Vec<String>,
    /// See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir
    pub keep_git_dir: Option<bool>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Add {
    pub files: Vec<Resource>,
    #[serde(flatten)]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#add---checksum
    pub checksum: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
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

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct User {
    pub user: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Port {
    pub port: u16,
    pub protocol: Option<PortProtocol>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum Resource {
    File(PathBuf),
    Url(Url),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum GitRepo {
    Http(Url),
    Ssh(SshGitRepo),
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch_derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct SshGitRepo {
    pub user: String,
    pub host: String,
    pub path: String,
}

// #[cfg(feature = "json_schema")]
// mod json_schema {
//     use super::*;

//     pub trait CustomSchema: JsonSchema {
//         fn schema_name() -> String;
//         fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema;
//     }

//     impl CustomSchema for Url {
//         fn schema_name() -> String {
//             "Url".to_string()
//         }

//         fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
//             <String as JsonSchema>::json_schema(gen)
//         }
//     }
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
                        user: "test".into(),
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
                        paths: vec!["file1.txt".into(), "file2.txt".into()].into(),
                        options: CopyOptions {
                            target: Some("destination/".into()),
                            chown: Some(User {
                                user: "root".into(),
                                group: Some("root".into())
                            }),
                            chmod: Some("755".into()),
                            link: Some(true),
                        },
                        exclude: vec!["file3.txt".into()].into(),
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
                        paths: vec!["file1.txt".into()].into(),
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
                        repo: "https://github.com/example/repo.git".into(),
                        options: CopyOptions {
                            target: Some("destination/".into()),
                            chown: Some(User {
                                user: "root".into(),
                                group: Some("root".into())
                            }),
                            chmod: Some("755".into()),
                            link: Some(true),
                        },
                        exclude: vec!["file3.txt".into()].into(),
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
                        files: vec![
                            Resource::File("file1.txt".into()),
                            Resource::File("file2.txt".into())
                        ]
                        .into(),
                        options: CopyOptions {
                            target: Some("destination/".into()),
                            chown: Some(User {
                                user: "root".into(),
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

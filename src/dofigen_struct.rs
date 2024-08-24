use crate::deserialize::*;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use struct_patch::Patch;
use url::Url;

/** Represents the Dockerfile main stage */
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[serde(rename_all = "camelCase")]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    // attribute(serde(deny_unknown_fields)),
    attribute(serde(default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Image", rename = "Image"))
    )
)]
pub struct Image {
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,

    #[patch(name = "VecPatch<String>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "ignores"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,

    #[patch(name = "VecDeepPatch<Stage, StagePatch>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub builders: Vec<Stage>,

    #[patch(name = "StagePatch", attribute(serde(flatten)))]
    #[serde(flatten)]
    pub stage: Stage,

    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entrypoint: Vec<String>,

    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cmd: Vec<String>,

    #[cfg_attr(
        feature = "permissive",
        patch(name = "VecDeepPatch<Port, ParsableStruct<PortPatch>>")
    )]
    #[cfg_attr(
        not(feature = "permissive"),
        patch(name = "VecDeepPatch<Port, PortPatch>")
    )]
    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(alias = "port", alias = "ports")))
    )]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expose: Vec<Port>,

    #[patch(name = "Option<HealthcheckPatch>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthcheck: Option<Healthcheck>,
}

/// Represents a Dockerfile stage
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    // attribute(serde(deny_unknown_fields)),
    attribute(serde(default)),
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Stage", rename = "Stage"))
    )
)]
pub struct Stage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[cfg_attr(
        feature = "permissive",
        patch(name = "Option<ParsableStruct<ImageNamePatch>>")
    )]
    #[cfg_attr(not(feature = "permissive"), patch(name = "Option<ImageNamePatch>"))]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "image"))))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ImageName>,

    #[cfg_attr(
        feature = "permissive",
        patch(name = "Option<ParsableStruct<UserPatch>>")
    )]
    #[cfg_attr(not(feature = "permissive"), patch(name = "Option<UserPatch>"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,

    #[patch(name = "HashMapPatch<String, String>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "envs"))))]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(
            alias = "add",
            alias = "adds",
            alias = "artifact",
            alias = "artifacts"
        )))
    )]
    #[cfg_attr(
        feature = "permissive",
        patch(name = "VecDeepPatch<CopyResource, ParsableStruct<CopyResourcePatch>>")
    )]
    #[cfg_attr(
        not(feature = "permissive"),
        patch(name = "VecDeepPatch<CopyResource, CopyResourcePatch>")
    )]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub copy: Vec<CopyResource>,

    #[patch(name = "Option<RunPatch>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<Run>,

    #[patch(name = "RunPatch", attribute(serde(flatten)))]
    #[serde(flatten)]
    pub run: Run,
}

/// Represents a run executed as root
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    // attribute(serde(deny_unknown_fields)),
    attribute(serde(default)),
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Run", rename = "Run"))
    )
)]
pub struct Run {
    #[patch(name = "VecPatch<String>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "script"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub run: Vec<String>,

    #[cfg_attr(
        feature = "permissive",
        patch(name = "VecDeepPatch<Cache, ParsableStruct<CachePatch>>")
    )]
    #[cfg_attr(
        not(feature = "permissive"),
        patch(name = "VecDeepPatch<Cache, CachePatch>")
    )]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "caches"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cache: Vec<Cache>,

    #[cfg_attr(
        feature = "permissive",
        patch(name = "VecDeepPatch<Bind, ParsableStruct<BindPatch>>")
    )]
    #[cfg_attr(
        not(feature = "permissive"),
        patch(name = "VecDeepPatch<Bind, BindPatch>")
    )]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "binds"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bind: Vec<Bind>,
}

/// Represents a cache definition during a run
/// See https://docs.docker.com/reference/dockerfile/#run---mounttypecache
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Cache", rename = "Cache"))
    )
)]
pub struct Cache {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    pub target: String,

    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "ro"))))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing: Option<CacheSharing>,

    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chmod: Option<String>,

    #[patch(name = "Option<UserPatch>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chown: Option<User>,
}

/// Represents file system binding during a run
/// See https://docs.docker.com/reference/dockerfile/#run---mounttypebind
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Bind", rename = "Bind"))
    )
)]
pub struct Bind {
    pub target: String,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    #[patch(name = "Option<FromContextPatch>", attribute(serde(flatten)))]
    pub from: Option<FromContext>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "rw"))))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readwrite: Option<bool>,
}

/// Represents the Dockerfile healthcheck instruction
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Healthcheck", rename = "Healthcheck"))
    )
)]
pub struct Healthcheck {
    pub cmd: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u16>,
}

/// Represents a Docker image name
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch, Hash, Eq, PartialOrd)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "ImageName", rename = "ImageName"))
    )
)]
pub struct ImageName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    pub path: String,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    #[patch(attribute(serde(flatten)))]
    pub version: Option<ImageVersion>,
}

/// Represents the COPY instruction in a Dockerfile.
/// See https://docs.docker.com/reference/dockerfile/#copy
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Copy", rename = "Copy"))
    )
)]
pub struct Copy {
    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    #[patch(name = "Option<FromContextPatch>", attribute(serde(flatten)))]
    pub from: Option<FromContext>,

    #[patch(name = "VecPatch<String>")]
    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(alias = "path", alias = "source")))
    )]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,

    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,

    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,

    /// See https://docs.docker.com/reference/dockerfile/#copy---parents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<bool>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default, rename_all = "camelCase"))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "AddGitRepo", rename = "AddGitRepo"))
    )
)]
pub struct AddGitRepo {
    pub repo: String,

    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,

    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,

    /// See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_git_dir: Option<bool>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Add", rename = "Add"))
    )
)]
pub struct Add {
    #[patch(name = "VecPatch<Resource>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "file"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<Resource>,

    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,

    /// See https://docs.docker.com/reference/dockerfile/#add---checksum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "CopyOptions", rename = "CopyOptions"))
    )
)]
pub struct CopyOptions {
    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(alias = "destination")))
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    #[patch(name = "Option<UserPatch>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chown: Option<User>,

    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chmod: Option<String>,

    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<bool>,
}

/// Represents user and group definition
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "User", rename = "User"))
    )
)]
pub struct User {
    pub user: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// Represents a port definition
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "Port", rename = "Port"))
    )
)]
pub struct Port {
    pub port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<PortProtocol>,
}

///////////////// Enums //////////////////

/// Represents a Docker image version
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Hash, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum ImageVersion {
    Tag(String),
    Digest(String),
}

/// Represents a copy origin
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FromContext {
    Image(ImageName),
    Builder(String),
    Context(String),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum CopyResource {
    Copy(Copy),
    AddGitRepo(AddGitRepo),
    Add(Add),
}

/// Represents a cache sharing strategy
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum CacheSharing {
    Shared,
    Private,
    Locked,
}

/// Represents a port protocol
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum PortProtocol {
    Tcp,
    Udp,
}

/// Represents a resource
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Hash, Eq, PartialOrd)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum Resource {
    Url(Url),
    File(PathBuf),
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod deserialize {
        use super::*;

        mod image {
            use super::*;

            #[test]
            fn empty() {
                let data = r#""#;

                let image: ImagePatch = serde_yaml::from_str(data).unwrap();
                let image: Image = image.into();

                assert_eq_sorted!(image, Image::default());
            }

            #[test]
            fn from() {
                let data = r#"
                from:
                  path: ubuntu
                "#;

                let image: ImagePatch = serde_yaml::from_str(data).unwrap();
                let image: Image = image.into();

                assert_eq_sorted!(
                    image,
                    Image {
                        stage: Stage {
                            from: Some(ImageName {
                                path: "ubuntu".into(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );
            }

            #[ignore]
            // Not managed yet by serde: https://serde.rs/field-attrs.html#flatten
            #[test]
            fn duplicate_from() {
                let data = r#"
                from:
                    path: ubuntu
                from:
                    path: alpine
                "#;

                let image: serde_yaml::Result<ImagePatch> = serde_yaml::from_str(data);

                println!("{:?}", image);

                assert!(image.is_err());
            }

            #[ignore]
            // Not managed yet by serde: https://serde.rs/field-attrs.html#flatten
            #[test]
            fn duplicate_from_and_alias() {
                let data = r#"
                from:
                  path: ubuntu
                image:
                  path: alpine
                "#;

                let image: serde_yaml::Result<ImagePatch> = serde_yaml::from_str(data);

                println!("{:?}", image);

                assert!(image.is_err());
            }
        }

        mod stage {
            use super::*;

            #[test]
            fn empty() {
                let data = r#""#;

                let stage: StagePatch = serde_yaml::from_str(data).unwrap();
                let stage: Stage = stage.into();

                assert_eq_sorted!(stage, Stage::default());
            }

            #[test]
            fn from() {
                let data = r#"
                from:
                  path: ubuntu
                "#;

                let stage: StagePatch = serde_yaml::from_str(data).unwrap();
                let stage: Stage = stage.into();

                assert_eq_sorted!(
                    stage,
                    Stage {
                        from: Some(ImageName {
                            path: "ubuntu".into(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }
                );
            }

            #[test]
            fn duplicate_from() {
                let data = r#"
                from:
                    path: ubuntu
                from:
                    path: alpine
                "#;

                let stage: serde_yaml::Result<StagePatch> = serde_yaml::from_str(data);

                assert!(stage.is_err());
            }

            #[test]
            fn duplicate_from_and_alias() {
                let data = r#"
                from:
                  path: ubuntu
                image:
                  path: alpine
                "#;

                let stage: serde_yaml::Result<StagePatch> = serde_yaml::from_str(data);

                assert!(stage.is_err());
            }
        }

        mod user {
            use super::*;

            #[test]
            fn name_and_group() {
                let json_data = r#"{
    "user": "test",
    "group": "test"
}"#;

                let user: UserPatch = serde_yaml::from_str(json_data).unwrap();
                let user: User = user.into();

                assert_eq_sorted!(
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
            fn copy() {
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
    "image": {"path": "my-image"}
}"#;

                let copy_resource: CopyResourcePatch = serde_yaml::from_str(json_data).unwrap();
                let copy_resource: CopyResource = copy_resource.into();

                assert_eq_sorted!(
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
                        from: Some(FromContext::Image(ImageName {
                            path: "my-image".into(),
                            ..Default::default()
                        }))
                    })
                );
            }

            #[cfg(feature = "permissive")]
            #[test]
            fn deserialize_copy_from_str() {
                let json_data = "file1.txt destination/";

                let copy_resource: ParsableStruct<CopyResourcePatch> =
                    serde_yaml::from_str(json_data).unwrap();
                let copy_resource: CopyResource = copy_resource.into();

                assert_eq_sorted!(
                    copy_resource,
                    CopyResource::Copy(Copy {
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
            fn add_git_repo() {
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
            "keepGitDir": true
        }"#;

                let copy_resource: CopyResourcePatch = serde_yaml::from_str(json_data).unwrap();
                let copy_resource: CopyResource = copy_resource.into();

                assert_eq_sorted!(
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
            fn add() {
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

                let copy_resource: CopyResourcePatch = serde_yaml::from_str(json_data).unwrap();
                let copy_resource: CopyResource = copy_resource.into();

                assert_eq_sorted!(
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

        mod builder {
            use super::*;

            #[test]
            fn with_bind() {
                let json_data = r#"
from:
  path: clux/muslrust:stable
workdir: /app
bind:
  - target: /app
run:
  - cargo build --release -F cli -F permissive
  - mv target/x86_64-unknown-linux-musl/release/dofigen /app/
"#;

                let builder: Stage = serde_yaml::from_str::<StagePatch>(json_data)
                    .unwrap()
                    .into();

                assert_eq!(
                    builder,
                    Stage {
                        from: ImageName {
                            path: "clux/muslrust:stable".into(),
                            ..Default::default()
                        }
                        .into(),
                        workdir: Some("/app".into()),
                        run: Run {
                            bind: vec![Bind {
                                target: "/app".into(),
                                ..Default::default()
                            }],
                            run: vec![
                                "cargo build --release -F cli -F permissive".into(),
                                "mv target/x86_64-unknown-linux-musl/release/dofigen /app/".into()
                            ],
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );
            }
        }
    }
}

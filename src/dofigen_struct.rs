use crate::deserialize::*;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use struct_patch::Patch;
use url::Url;

/** Represents the Dockerfile main stage */
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    // attribute(serde(deny_unknown_fields)),
    attribute(serde(default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,

    #[patch(attribute(serde(alias = "ignores")))]
    #[patch(name = "VecPatch<String>")]
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
    #[patch(attribute(serde(alias = "port", alias = "ports")))]
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
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]

pub struct Stage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[cfg_attr(
        feature = "permissive",
        patch(name = "Option<ParsableStruct<ImageNamePatch>>")
    )]
    #[cfg_attr(not(feature = "permissive"), patch(name = "Option<ImageNamePatch>"))]
    #[patch(attribute(serde(alias = "image")))]
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

    #[patch(attribute(serde(alias = "envs")))]
    // TODO: handle patching for map
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    #[patch(name = "VecDeepPatch<Artifact, ArtifactPatch>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,

    #[patch(attribute(serde(alias = "add", alias = "adds")))]
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

    #[patch(name = "Option<RootPatch>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<Root>,

    #[patch(attribute(serde(alias = "script")))]
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub run: Vec<String>,

    #[patch(attribute(serde(alias = "caches")))]
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cache: Vec<String>,
}

/// Represents an artifact to be copied to the stage from another one
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]
pub struct Artifact {
    pub builder: String,

    pub source: String,

    #[patch(attribute(serde(alias = "destination")))]
    pub target: String,
}

/// Represents a run executed as root
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]
pub struct Root {
    #[serde(alias = "script")]
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub run: Vec<String>,

    #[serde(alias = "caches")]
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cache: Vec<String>,
}

/// Represents the Dockerfile healthcheck instruction
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
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
#[derive(Serialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
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

/// Represents a Docker image version
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum ImageVersion {
    Tag(String),
    Digest(String),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum CopyResource {
    Copy(Copy),
    AddGitRepo(AddGitRepo),
    Add(Add),
}

/// Represents the COPY instruction in a Dockerfile.
/// See https://docs.docker.com/reference/dockerfile/#copy
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]
pub struct Copy {
    #[patch(name = "VecPatch<String>")]
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

    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
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
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]
pub struct Add {
    #[patch(name = "VecPatch<Resource>")]
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
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]
pub struct CopyOptions {
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
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
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
    attribute(serde(deny_unknown_fields, default)),
    attribute(cfg_attr(feature = "json_schema", derive(JsonSchema)))
)]
pub struct Port {
    pub port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<PortProtocol>,
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
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Hash, Eq)]
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
                        from: Some("source/".into())
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
    }
}

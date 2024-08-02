use crate::deserialize_struct::{OptionPatch, VecDeepPatch, VecPatch};
#[cfg(feature = "permissive")]
use crate::serde_permissive::ParsableStruct;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};
use struct_patch::Patch;
use url::Url;

// #[cfg(feature = "permissive")]
// pub type PermissiveStruct<T> = ParsableStruct<T>;
// #[cfg(not(feature = "permissive"))]
// pub type PermissiveStruct<T> = Box<T>;

/** Represents the Dockerfile main stage */
#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[patch(attribute(serde(deny_unknown_fields, default)))]
pub struct Image {
    #[patch(attribute(serde(flatten)))]
    #[patch(name = "StagePatch")]
    pub stage: Stage,
    #[patch(name = "VecDeepPatch<Stage, StagePatch>")]
    pub builders: Vec<Stage>,
    #[patch(name = "VecPatch<String>")]
    pub context: Vec<String>,
    #[patch(attribute(serde(alias = "ignores")))]
    #[patch(name = "VecPatch<String>")]
    pub ignore: Vec<String>,
    #[patch(name = "VecPatch<String>")]
    pub entrypoint: Vec<String>,
    #[patch(name = "VecPatch<String>")]
    pub cmd: Vec<String>,
    #[patch(attribute(serde(alias = "port", alias = "ports")))]
    #[patch(name = "VecDeepPatch<Port, PortPatch>")]
    pub expose: Vec<Port>,
    pub healthcheck: Option<Healthcheck>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]

pub struct Stage {
    pub name: Option<String>,
    #[patch(attribute(serde(alias = "image")))]
    #[cfg_attr(
        feature = "permissive",
        patch(name = "OptionPatch<ParsableStruct<ImageNamePatch>>")
    )]
    #[cfg_attr(
        not(feature = "permissive"),
        patch(name = "OptionPatch<ImageNamePatch>")
    )]
    pub from: Option<ImageName>,
    #[cfg_attr(
        feature = "permissive",
        patch(name = "OptionPatch<ParsableStruct<UserPatch>>")
    )]
    #[cfg_attr(
        not(feature = "permissive"),
        patch(name = "OptionPatch<UserPatch>")
    )]
    pub user: Option<User>,
    pub workdir: Option<String>,
    #[patch(attribute(serde(alias = "envs")))]
    // TODO: handle patching for map
    pub env: HashMap<String, String>,
    #[patch(name = "VecDeepPatch<Artifact, ArtifactPatch>")]
    pub artifacts: Vec<Artifact>,
    // #[patch(attribute(serde(alias = "add", alias = "adds")))]
    // #[cfg_attr(
    //     feature = "permissive",
    //     patch(name = "VecDeepPatch<CopyResource, ParsableStruct<CopyResourcePatch>>>")
    // )]
    // #[cfg_attr(
    //     not(feature = "permissive"),
    //     patch(name = "VecDeepPatch<CopyResource, CopyResourcePatch>")
    // )]
    pub copy: Vec<CopyResource>,
    #[patch(name = "OptionPatch<RootPatch>")]
    pub root: Option<Root>,
    #[patch(attribute(serde(alias = "script")))]
    #[patch(name = "VecPatch<String>")]
    pub run: Vec<String>,
    #[patch(attribute(serde(alias = "caches")))]
    #[patch(name = "VecPatch<String>")]
    pub cache: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct Artifact {
    pub builder: String,
    pub source: String,
    #[serde(alias = "destination")]
    pub target: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
#[serde(deny_unknown_fields, default)]
pub struct Root {
    #[serde(alias = "script")]
    #[patch(name = "VecPatch<String>")]
    pub run: Vec<String>,
    #[serde(alias = "caches")]
    #[patch(name = "VecPatch<String>")]
    pub cache: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
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
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum CopyResourcePatch {
    Copy(CopyPatch),
    AddGitRepo(AddGitRepoPatch),
    Add(AddPatch),
    Unknown(UnknownPatch),
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct UnknownPatch {
    #[serde(flatten)]
    pub options: Option<CopyOptionsPatch>,
    pub exclude: Option<VecPatch<String>>,
}

/// Represents the COPY instruction in a Dockerfile.
/// See https://docs.docker.com/reference/dockerfile/#copy
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Copy {
    // #[patch(name = "VecPatch<String>")]
    pub paths: Vec<String>,
    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    #[patch(name = "VecPatch<String>")]
    pub exclude: Vec<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---parents
    pub parents: Option<bool>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    pub from: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile specific for Git repo.
/// See https://docs.docker.com/reference/dockerfile/#adding-private-git-repositories
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct AddGitRepo {
    pub repo: String,
    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    #[patch(name = "VecPatch<String>")]
    pub exclude: Vec<String>,
    /// See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir
    pub keep_git_dir: Option<bool>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct Add {
    #[patch(name = "VecPatch<Resource>")]
    pub files: Vec<Resource>,
    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,
    /// See https://docs.docker.com/reference/dockerfile/#add---checksum
    pub checksum: Option<String>,
}

/// Represents the ADD instruction in a Dockerfile file from URLs or uncompress an archive.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct CopyOptions {
    pub target: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    #[patch(name = "OptionPatch<UserPatch>")]
    pub chown: Option<User>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    pub chmod: Option<String>,
    /// See https://docs.docker.com/reference/dockerfile/#copy---link
    pub link: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct User {
    pub user: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Patch)]
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
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
#[patch(attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)))]
#[serde(deny_unknown_fields, default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct SshGitRepo {
    pub user: String,
    pub host: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Extend<T> {
    pub extend: Vec<Resource>,
    pub value: T,
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

// macro_rules! impl_from_patch {
//     ($struct:ty, $patch:ty) => {
//         impl From<$patch> for $struct {
//             fn from(patch: $patch) -> Self {
//                 let mut struct_data = <$struct>::default();
//                 struct_data.apply(patch);
//                 struct_data
//             }
//         }

//         impl From<$struct> for $patch {
//             fn from(value: $struct) -> Self {
//                 value.into()
//             }
//         }
//     };
// }

// impl_from_patch!(ImageName, ImageNamePatch);
// impl_from_patch!(User, UserPatch);
// impl_from_patch!(Root, RootPatch);

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

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

                let copy_resource: CopyResource = serde_yaml::from_str(json_data).unwrap();

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

            // #[cfg(feature = "permissive")]
            // #[test]
            // fn deserialize_copy_from_str() {
            //     use std::ops::Deref;

            //     let json_data = "file1.txt destination/";

            //     let copy_resource: PermissiveStruct<CopyResource> =
            //         serde_yaml::from_str(json_data).unwrap();

            //     assert_eq_sorted!(
            //         copy_resource.deref(),
            //         &CopyResource::Copy(Copy {
            //             paths: vec!["file1.txt".into()].into(),
            //             options: CopyOptions {
            //                 target: Some("destination/".into()),
            //                 ..Default::default()
            //             },
            //             ..Default::default()
            //         })
            //     );
            // }

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

                let copy_resource: CopyResource = serde_yaml::from_str(json_data).unwrap();

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

        mod extend {
            use super::*;

            #[derive(Deserialize, Patch)]
            #[patch(
                attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
                attribute(serde(deny_unknown_fields, default))
            )]
            struct TestStruct {
                pub name: Option<String>,
                #[serde(flatten)]
                #[patch(name = "TestSubStructPatch", attribute(serde(flatten)))]
                pub sub: TestSubStruct,
            }

            #[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
            #[patch(
                attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
                attribute(serde(deny_unknown_fields, default))
            )]
            struct TestSubStruct {
                pub level: u16,
            }

            #[test]
            fn empty() {
                let data = r#"{}"#;

                let extend_image: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        extend: vec![],
                        value: TestStructPatch {
                            sub: Some(TestSubStructPatch::default()),
                            ..Default::default()
                        }
                    }
                );
            }

            #[test]
            fn only_name() {
                let data = r#"
                name: ok
                "#;

                let extend: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend,
                    Extend {
                        extend: vec![],
                        value: TestStructPatch {
                            name: Some(Some("ok".into())),
                            sub: Some(TestSubStructPatch::default()),
                            ..Default::default()
                        }
                    }
                );
            }

            #[test]
            fn only_sub() {
                let data = r#"
                level: 1
                "#;

                let extend: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend,
                    Extend {
                        extend: vec![],
                        value: TestStructPatch {
                            sub: Some(TestSubStructPatch {
                                level: Some(1),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                    }
                );
            }
        }

        mod extend_image {
            use super::*;

            #[test]
            fn empty() {
                let data = r#"{}"#;

                let extend_image: Extend<ImagePatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        extend: vec![],
                        value: ImagePatch {
                            stage: Some(StagePatch::default()),
                            ..Default::default()
                        }
                    }
                );
            }

            #[test]
            fn only_from() {
                let data = r#"
                from:
                    path: ubuntu
                "#;

                let extend_image: Extend<ImagePatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        extend: vec![],
                        value: ImagePatch {
                            stage: Some(StagePatch {
                                from: Some(OptionPatch::new(Some(
                                    ImageNamePatch {
                                        path: Some("ubuntu".into()),
                                        ..Default::default()
                                    }
                                    .into() // To manage permissive
                                ))),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                    }
                );
            }
        }
    }
}

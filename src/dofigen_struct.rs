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
        attribute(schemars(title = "Dofigen", rename = "Dofigen"))
    )
)]
pub struct Dofigen {
    /// The context of the Docker build
    /// This is used to generate a .dockerignore file
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,

    /// The elements to ignore from the build context
    /// This is used to generate a .dockerignore file
    #[patch(name = "VecPatch<String>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "ignores"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,

    /// The builder stages of the Dockerfile
    #[patch(name = "HashMapDeepPatch<String, StagePatch>")]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub builders: HashMap<String, Stage>,

    /// The runtime stage of the Dockerfile
    #[patch(name = "StagePatch", attribute(serde(flatten)))]
    #[serde(flatten)]
    pub stage: Stage,

    /// The entrypoint of the Dockerfile
    /// See https://docs.docker.com/reference/dockerfile/#entrypoint
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entrypoint: Vec<String>,

    /// The default command of the Dockerfile
    /// See https://docs.docker.com/reference/dockerfile/#cmd
    #[patch(name = "VecPatch<String>")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cmd: Vec<String>,

    /// The ports exposed by the Dockerfile
    /// See https://docs.docker.com/reference/dockerfile/#expose
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

    /// The healthcheck of the Dockerfile
    /// See https://docs.docker.com/reference/dockerfile/#healthcheck
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
    /// The base of the stage
    /// See https://docs.docker.com/reference/dockerfile/#from
    #[serde(flatten, skip_serializing_if = "FromContext::is_empty")]
    #[patch(name = "FromContextPatch", attribute(serde(flatten, default)))]
    pub from: FromContext,

    /// The user and group of the stage
    /// See https://docs.docker.com/reference/dockerfile/#user
    #[cfg_attr(
        feature = "permissive",
        patch(name = "Option<ParsableStruct<UserPatch>>")
    )]
    #[cfg_attr(not(feature = "permissive"), patch(name = "Option<UserPatch>"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,

    /// The working directory of the stage
    /// See https://docs.docker.com/reference/dockerfile/#workdir
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,

    /// The build args that can be used in the stage
    /// See https://docs.docker.com/reference/dockerfile/#arg
    #[patch(name = "HashMapPatch<String, String>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "args"))))]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub arg: HashMap<String, String>,

    /// The environment variables of the stage
    /// See https://docs.docker.com/reference/dockerfile/#env
    #[patch(name = "HashMapPatch<String, String>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "envs"))))]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// The copy instructions of the stage
    /// See https://docs.docker.com/reference/dockerfile/#copy and https://docs.docker.com/reference/dockerfile/#add
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

    /// The run instructions of the stage as root user
    #[patch(name = "Option<RunPatch>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<Run>,

    /// The run instructions of the stage
    /// See https://docs.docker.com/reference/dockerfile/#run
    #[patch(name = "RunPatch", attribute(serde(flatten)))]
    #[serde(flatten)]
    pub run: Run,
}

/// Represents a run command
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
    /// The commands to run
    #[patch(name = "VecPatch<String>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "script"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub run: Vec<String>,

    /// The cache definitions during the run
    /// See https://docs.docker.com/reference/dockerfile/#run---mounttypecache
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

    /// The file system bindings during the run
    /// This is used to mount a file or directory from the host into the container only during the run and it's faster than a copy
    /// See https://docs.docker.com/reference/dockerfile/#run---mounttypebind
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
    /// The id of the cache
    /// This is used to share the cache between different stages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The target path of the cache
    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(alias = "dst", alias = "destination")))
    )]
    pub target: String,

    /// Defines if the cache is readonly
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "ro"))))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,

    /// The sharing strategy of the cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing: Option<CacheSharing>,

    /// Build stage, context, or image name to use as a base of the cache mount. Defaults to empty directory.
    #[serde(flatten, skip_serializing_if = "FromContext::is_empty")]
    #[patch(name = "FromContextPatch", attribute(serde(flatten)))]
    pub from: FromContext,

    /// Subpath in the from to mount. Defaults to the root of the from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// The permissions of the cache
    #[cfg_attr(
        feature = "permissive",
        patch(attribute(serde(
            deserialize_with = "deserialize_from_optional_string_or_number",
            default
        )))
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chmod: Option<String>,

    /// The user and group that own the cache
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
    /// The target path of the bind
    pub target: String,

    /// The base of the bind
    #[serde(flatten, skip_serializing_if = "FromContext::is_empty")]
    #[patch(name = "FromContextPatch", attribute(serde(flatten)))]
    pub from: FromContext,

    /// Source path in the from. Defaults to the root of the from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Defines if the bind is read and write
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "rw"))))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readwrite: Option<bool>,
}

/// Represents the Dockerfile healthcheck instruction
/// See https://docs.docker.com/reference/dockerfile/#healthcheck
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
    /// The test to run
    pub cmd: String,

    /// The interval between two tests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,

    /// The timeout of the test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// The start period of the test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    /// The number of retries
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
    /// The host of the image registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    /// The port of the image registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// The path of the image repository
    pub path: String,

    /// The version of the image
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
    /// The origin of the copy
    /// See https://docs.docker.com/reference/dockerfile/#copy---from
    #[serde(flatten, skip_serializing_if = "FromContext::is_empty")]
    #[patch(name = "FromContextPatch", attribute(serde(flatten)))]
    pub from: FromContext,

    /// The paths to copy
    #[patch(name = "VecPatch<String>")]
    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(alias = "path", alias = "source")))
    )]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,

    /// The options of the copy
    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,
    // excludes are not supported yet: minimal version 1.7-labs
    // /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    // #[patch(name = "VecPatch<String>")]
    // #[serde(skip_serializing_if = "Vec::is_empty")]
    // pub exclude: Vec<String>,

    // parents are not supported yet: minimal version 1.7-labs
    // /// See https://docs.docker.com/reference/dockerfile/#copy---parents
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub parents: Option<bool>,
}

/// Represents the COPY instruction in a Dockerfile from file content.
/// See https://docs.docker.com/reference/dockerfile/#example-creating-inline-files
#[derive(Debug, Clone, PartialEq, Default, Serialize, Patch)]
#[patch(
    attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
    attribute(serde(deny_unknown_fields, default))
)]
#[cfg_attr(
    feature = "json_schema",
    patch(
        attribute(derive(JsonSchema)),
        attribute(schemars(title = "CopyContent", rename = "CopyContent"))
    )
)]
pub struct CopyContent {
    /// Content of the file to copy
    pub content: String,

    /// If true, replace variables in the content at build time. Default is true.
    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(alias = "subst")))
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substitute: Option<bool>,

    /// The options of the copy
    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,
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
    /// The URL of the Git repository
    pub repo: String,

    /// The options of the copy
    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,

    // excludes are not supported yet: minimal version 1.7-labs
    // /// See https://docs.docker.com/reference/dockerfile/#copy---exclude
    // #[patch(name = "VecPatch<String>")]
    // #[serde(skip_serializing_if = "Vec::is_empty")]
    // pub exclude: Vec<String>,
    /// Keep the git directory
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
    /// The files to add
    #[patch(name = "VecPatch<Resource>")]
    #[cfg_attr(not(feature = "strict"), patch(attribute(serde(alias = "file"))))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<Resource>,

    /// The options of the copy
    #[serde(flatten)]
    #[patch(name = "CopyOptionsPatch", attribute(serde(flatten)))]
    pub options: CopyOptions,

    /// The checksum of the files
    /// See https://docs.docker.com/reference/dockerfile/#add---checksum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// Represents the options of a COPY/ADD instructions
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
    /// The target path of the copied files
    #[cfg_attr(
        not(feature = "strict"),
        patch(attribute(serde(alias = "destination")))
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// The user and group that own the copied files
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    #[patch(name = "Option<UserPatch>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chown: Option<User>,

    /// The permissions of the copied files
    /// See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod
    #[cfg_attr(
        feature = "permissive",
        patch(attribute(serde(
            deserialize_with = "deserialize_from_optional_string_or_number",
            default
        )))
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chmod: Option<String>,

    /// Use of the link flag
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
    /// The user name or ID
    /// The ID is preferred
    pub user: String,

    /// The group name or ID
    /// The ID is preferred
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
    /// The port number
    pub port: u16,

    /// The protocol of the port
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
    FromImage(ImageName),
    FromBuilder(String),
    FromContext(Option<String>),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum CopyResource {
    Copy(Copy),
    Content(CopyContent),
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

///////////////// Enum Patches //////////////////

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum FromContextPatch {
    #[cfg(not(feature = "permissive"))]
    #[cfg_attr(not(feature = "strict"), serde(alias = "image"))]
    FromImage(ImageNamePatch),

    #[cfg(feature = "permissive")]
    #[cfg_attr(not(feature = "strict"), serde(alias = "image"))]
    FromImage(ParsableStruct<ImageNamePatch>),

    #[cfg_attr(not(feature = "strict"), serde(alias = "builder"))]
    FromBuilder(String),

    #[cfg_attr(not(feature = "strict"), serde(alias = "from"))]
    FromContext(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub enum CopyResourcePatch {
    Copy(CopyPatch),
    Content(CopyContentPatch),
    AddGitRepo(AddGitRepoPatch),
    Add(AddPatch),
    Unknown(UnknownPatch),
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema))]
pub struct UnknownPatch {
    #[serde(flatten)]
    pub options: Option<CopyOptionsPatch>,
    // exclude are not supported yet: minimal version 1.7-labs
    // pub exclude: Option<VecPatch<String>>,
}

///////////////// Tests //////////////////

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod deserialize {
        use super::*;

        mod dofigen {
            use super::*;

            #[test]
            fn empty() {
                let data = r#""#;

                let dofigen: DofigenPatch = serde_yaml::from_str(data).unwrap();
                let dofigen: Dofigen = dofigen.into();

                assert_eq_sorted!(dofigen, Dofigen::default());
            }

            #[test]
            fn from() {
                let data = r#"
                fromImage:
                  path: ubuntu
                "#;

                let dofigen: DofigenPatch = serde_yaml::from_str(data).unwrap();
                let dofigen: Dofigen = dofigen.into();

                assert_eq_sorted!(
                    dofigen,
                    Dofigen {
                        stage: Stage {
                            from: FromContext::FromImage(ImageName {
                                path: "ubuntu".into(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );
            }

            #[ignore = "Not managed yet by serde because of multilevel flatten: https://serde.rs/field-attrs.html#flatten"]
            #[test]
            fn duplicate_from() {
                let data = r#"
                from:
                    path: ubuntu
                from:
                    path: alpine
                "#;

                let dofigen: serde_yaml::Result<DofigenPatch> = serde_yaml::from_str(data);

                println!("{:?}", dofigen);

                assert!(dofigen.is_err());
            }

            #[ignore = "Not managed yet by serde because of multilevel flatten: https://serde.rs/field-attrs.html#flatten"]
            #[test]
            fn duplicate_from_and_alias() {
                let data = r#"
                from:
                  path: ubuntu
                image:
                  path: alpine
                "#;

                let dofigen: serde_yaml::Result<DofigenPatch> = serde_yaml::from_str(data);

                println!("{:?}", dofigen);

                assert!(dofigen.is_err());
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
                fromImage:
                  path: ubuntu
                "#;

                let stage: StagePatch = serde_yaml::from_str(data).unwrap();
                let stage: Stage = stage.into();

                assert_eq_sorted!(
                    stage,
                    Stage {
                        from: FromContext::FromImage(ImageName {
                            path: "ubuntu".into(),
                            ..Default::default()
                        })
                        .into(),
                        ..Default::default()
                    }
                );
            }

            #[ignore = "Not managed yet by serde because of multilevel flatten: https://serde.rs/field-attrs.html#flatten"]
            #[test]
            fn duplicate_from() {
                let data = r#"
                fromImage:
                    path: ubuntu
                fromImage:
                    path: alpine
                "#;

                let stage: serde_yaml::Result<StagePatch> = serde_yaml::from_str(data);

                assert!(stage.is_err());
            }

            #[ignore = "Not managed yet by serde because of multilevel flatten: https://serde.rs/field-attrs.html#flatten"]
            #[test]
            fn duplicate_from_and_alias() {
                let data = r#"
                fromImage:
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
    "link": true,
    "fromImage": {"path": "my-image"}
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
                        from: FromContext::FromImage(ImageName {
                            path: "my-image".into(),
                            ..Default::default()
                        })
                    })
                );
            }

            #[test]
            fn copy_simple() {
                let json_data = r#"{
    "paths": ["file1.txt"]
}"#;

                let copy_resource: CopyResourcePatch = serde_yaml::from_str(json_data).unwrap();

                assert_eq_sorted!(
                    copy_resource,
                    CopyResourcePatch::Copy(CopyPatch {
                        paths: Some(vec!["file1.txt".into()].into_patch()),
                        options: Some(CopyOptionsPatch::default()),
                        ..Default::default()
                    })
                );

                let copy_resource: CopyResource = copy_resource.into();

                assert_eq_sorted!(
                    copy_resource,
                    CopyResource::Copy(Copy {
                        paths: vec!["file1.txt".into()].into(),
                        options: CopyOptions::default(),
                        ..Default::default()
                    })
                );
            }

            #[cfg(feature = "permissive")]
            #[test]
            fn copy_chmod_int() {
                let json_data = r#"{
    "paths": ["file1.txt"],
    "chmod": 755
}"#;

                let copy_resource: CopyPatch = serde_yaml::from_str(json_data).unwrap();

                assert_eq_sorted!(
                    copy_resource,
                    CopyPatch {
                        paths: Some(vec!["file1.txt".into()].into_patch()),
                        options: Some(CopyOptionsPatch {
                            chmod: Some(Some("755".into())),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }
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
            fn copy_content() {
                let json_data = r#"{
    "content": "echo coucou",
    "substitute": false,
    "target": "test.sh",
    "chown": {
        "user": "1001",
        "group": "1001"
    },
    "chmod": "555",
    "link": true
}"#;

                let copy_resource: CopyResourcePatch = serde_yaml::from_str(json_data).unwrap();
                let copy_resource: CopyResource = copy_resource.into();

                assert_eq_sorted!(
                    copy_resource,
                    CopyResource::Content(CopyContent {
                        content: "echo coucou".into(),
                        substitute: Some(false),
                        options: CopyOptions {
                            target: Some("test.sh".into()),
                            chown: Some(User {
                                user: "1001".into(),
                                group: Some("1001".into())
                            }),
                            chmod: Some("555".into()),
                            link: Some(true),
                        }
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
fromImage:
  path: clux/muslrust:stable
workdir: /app
bind:
  - target: /app
run:
  - cargo build --release
  - mv target/x86_64-unknown-linux-musl/release/dofigen /app/
"#;

                let builder: Stage = serde_yaml::from_str::<StagePatch>(json_data)
                    .unwrap()
                    .into();

                assert_eq_sorted!(
                    builder,
                    Stage {
                        from: FromContext::FromImage(ImageName {
                            path: "clux/muslrust:stable".into(),
                            ..Default::default()
                        }),
                        workdir: Some("/app".into()),
                        run: Run {
                            bind: vec![Bind {
                                target: "/app".into(),
                                ..Default::default()
                            }],
                            run: vec![
                                "cargo build --release".into(),
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

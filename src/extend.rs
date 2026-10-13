#[cfg(feature = "permissive")]
use crate::OneOrMany;
#[cfg(feature = "parse")]
use crate::{DockerFile, DockerIgnore, Dofigen};
use crate::{DofigenContext, Error, Result, dofigen_struct::*};
use relative_path::RelativePath;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{Deserialize, de::DeserializeOwned};
use std::iter;
use std::path::Path;
use struct_patch::Merge;

#[cfg(feature = "permissive")]
type VecType<T> = OneOrMany<T>;

#[cfg(not(feature = "permissive"))]
type VecType<T> = Vec<T>;

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
// #[serde(deny_unknown_fields)]
#[serde(default)]
#[cfg_attr(
    feature = "json_schema",
    derive(JsonSchema),
    schemars(rename = "Extend<{T}>", default)
)]
pub struct Extend<T: Default + Merge> {
    #[serde(alias = "extends")]
    pub extend: VecType<Resource>,

    // Can't use #[serde(flatten)] because of nested flattening is not managed by serde
    #[serde(flatten)]
    pub value: T,
}

impl<P> Extend<P>
where
    P: Default + DeserializeOwned + Clone + Merge,
{
    pub fn merge(&self, context: &mut DofigenContext) -> Result<P> {
        if self.extend.is_empty() {
            return Ok(self.value.clone().into());
        }

        // load extends files
        let merged: Option<P> = self
            .extend
            .iter()
            .map(|extend| {
                let ret = extend.load::<Self>(context)?.merge(context)?;
                context.pop_resource_stack();
                Ok(ret)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .chain(iter::once(self.value.clone()))
            .reduce(|a, b| a.merge(b));

        Ok(merged.expect("Since we have at least one value, we should have a merged value"))
    }
}

impl Resource {
    pub fn is_dockerfile_name(name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        if name.ends_with(".yml") || name.ends_with(".yaml") || name.ends_with(".json") {
            return false;
        }
        name == "dockerfile" || name.starts_with("dockerfile.") || name.ends_with(".dockerfile")
    }

    pub fn looks_like_dockerfile(&self) -> bool {
        match self {
            Resource::File(path) => path
                .file_name()
                .and_then(|n| n.to_str())
                .map(Self::is_dockerfile_name)
                .unwrap_or(false),
            Resource::Url(url) => url
                .path_segments()
                .and_then(|mut segments| segments.next_back())
                .map(Self::is_dockerfile_name)
                .unwrap_or(false),
        }
    }

    fn resolve_resource(&self, context: &DofigenContext) -> Result<Resource> {
        Ok(match self {
            Resource::File(path) => {
                if path.is_absolute() {
                    Resource::File(path.clone())
                } else if let Some(current_resource) = context.current_resource() {
                    match current_resource {
                        Resource::File(file) => {
                            if file.is_absolute() {
                                let parent = file.parent().unwrap_or_else(|| Path::new(""));
                                Resource::File(parent.join(path))
                            } else {
                                let current_file_relative_path =
                                    RelativePath::from_path(file).map_err(Error::display)?;
                                let relative_path =
                                    RelativePath::from_path(path).map_err(Error::display)?;
                                let relative_path = current_file_relative_path
                                    .join("..")
                                    .join_normalized(relative_path);
                                Resource::File(relative_path.to_path(""))
                            }
                        }
                        Resource::Url(url) => {
                            Resource::Url(url.join(path.to_str().unwrap()).unwrap())
                        }
                    }
                } else {
                    Resource::File(path.clone())
                }
            }
            Resource::Url(url) => Resource::Url(url.clone()),
        })
    }

    fn load_resource_content(&self, context: &mut DofigenContext) -> Result<String> {
        let resource = self.resolve_resource(context)?;

        // push the resource to the stack
        context.push_resource_stack(resource.clone())?;

        // load the resource content
        context.get_resource_content(resource)
    }

    pub fn load<T>(&self, context: &mut DofigenContext) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let content = self.load_resource_content(context)?;

        match self {
            Resource::File(path)
                if path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("toml")) =>
            {
                Ok(toml::from_str(content.as_str()).map_err(|err| {
                    Error::Custom(format!(
                        "Could not deserialize resource {:?}: {}",
                        self, err
                    ))
                })?)
            }

            _ => Ok(serde_yaml::from_str(content.as_str()).map_err(|err| {
                Error::Custom(format!(
                    "Could not deserialize resource {:?}: {}",
                    self, err
                ))
            })?),
        }
        #[cfg(feature = "parse")]
        {
            let resolved = context
                .current_resource()
                .cloned()
                .unwrap_or_else(|| self.clone());
            if resolved.looks_like_dockerfile() {
                return Self::load_from_dockerfile_content(&content, context);
            }
        }

        serde_yaml::from_str(content.as_str()).map_err(|err| {
            Error::Custom(format!(
                "Could not deserialize resource {:?}: {}",
                self, err
            ))
        })
    }

    #[cfg(feature = "parse")]
    fn load_from_dockerfile_content<T>(content: &str, context: &DofigenContext) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let dockerfile: DockerFile = content.parse().map_err(|err| {
            Error::Custom(format!("Could not parse extended Dockerfile: {}", err))
        })?;

        let dockerignore = match context.current_resource() {
            Some(Resource::File(path)) => Self::load_companion_dockerignore(path)?,
            _ => None,
        };

        let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore)?;

        // Round-trip through YAML so any deserializable Dofigen-shaped target works
        let yaml = serde_yaml::to_string(&dofigen).map_err(|err| {
            Error::Custom(format!(
                "Could not serialize Dockerfile as Dofigen: {}",
                err
            ))
        })?;

        serde_yaml::from_str(&yaml).map_err(|err| {
            Error::Custom(format!(
                "Could not convert Dockerfile to Dofigen configuration: {}",
                err
            ))
        })
    }

    #[cfg(feature = "parse")]
    fn load_companion_dockerignore(dockerfile_path: &Path) -> Result<Option<DockerIgnore>> {
        let Some(filename) = dockerfile_path.file_name().and_then(|n| n.to_str()) else {
            return Ok(None);
        };
        let mut ignorefile = dockerfile_path.with_file_name(format!("{}.dockerignore", filename));
        if filename.eq_ignore_ascii_case("Dockerfile") && !ignorefile.exists() {
            ignorefile = dockerfile_path.with_file_name(".dockerignore");
        }
        if !ignorefile.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&ignorefile).map_err(|err| {
            Error::Custom(format!("Could not read {}: {}", ignorefile.display(), err))
        })?;
        let dockerignore = content.parse().map_err(|err| {
            Error::Custom(format!("Could not parse {}: {}", ignorefile.display(), err))
        })?;
        Ok(Some(dockerignore))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod deserialize {
        use super::*;
        use struct_patch::Patch;

        mod extend {

            use super::*;

            #[derive(Deserialize, Debug, Clone, PartialEq, Default, Patch)]
            #[patch(
                attribute(derive(Deserialize, Debug, Clone, PartialEq, Default)),
                attribute(serde(default))
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
                        value: TestStructPatch {
                            sub: Some(TestSubStructPatch::default()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );
            }

            #[test]
            fn only_name() {
                let data = "name: ok";

                let extend: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend,
                    Extend {
                        value: TestStructPatch {
                            name: Some(Some("ok".into())),
                            sub: Some(TestSubStructPatch::default()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );
            }

            #[test]
            fn only_sub() {
                let data = "level: 1";

                let extend: Extend<TestStructPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend,
                    Extend {
                        value: TestStructPatch {
                            sub: Some(TestSubStructPatch {
                                level: Some(1),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );

                let merged = extend.merge(&mut DofigenContext::new()).unwrap();

                let mut resulting = TestStruct::default();
                resulting.apply(merged);

                assert_eq_sorted!(
                    resulting,
                    TestStruct {
                        name: None,
                        sub: TestSubStruct { level: 1 },
                    }
                );
            }
        }

        mod extend_dofigen {
            use super::*;
            use crate::{DofigenPatch, ImageNamePatch, RunPatch, StagePatch};

            // #[ignore = "Not managed yet by serde because of multilevel flatten: https://serde.rs/field-attrs.html#flatten"]
            #[test]
            fn empty() {
                let data = r#"{}"#;

                let extend_image: Extend<DofigenPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        value: DofigenPatch {
                            stage: Some(StagePatch {
                                run: Some(RunPatch::default()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );
            }

            #[test]
            fn only_from() {
                let data = r#"
fromImage:
  path: ubuntu
"#;

                let extend_image: Extend<DofigenPatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        value: DofigenPatch {
                            stage: Some(StagePatch {
                                from: Some(FromContextPatch::FromImage(
                                    ImageNamePatch {
                                        path: Some("ubuntu".into()),
                                        version: Some(None),
                                        ..Default::default()
                                    }
                                    .into() // To manage permissive
                                )),
                                run: Some(RunPatch::default()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                );
            }
        }
    }

    mod dockerfile_name_detection {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn recognizes_dockerfile_names() {
            assert!(Resource::is_dockerfile_name("Dockerfile"));
            assert!(Resource::is_dockerfile_name("dockerfile"));
            assert!(Resource::is_dockerfile_name("DOCKERFILE"));
            assert!(Resource::is_dockerfile_name("Dockerfile.prod"));
            assert!(Resource::is_dockerfile_name("Dockerfile.base"));
            assert!(Resource::is_dockerfile_name("app.Dockerfile"));
            assert!(Resource::is_dockerfile_name("backend.dockerfile"));
        }

        #[test]
        fn rejects_dofigen_and_other_names() {
            assert!(!Resource::is_dockerfile_name("dofigen.yml"));
            assert!(!Resource::is_dockerfile_name("dofigen.yaml"));
            assert!(!Resource::is_dockerfile_name("dofigen.json"));
            assert!(!Resource::is_dockerfile_name("Dockerfile.yml"));
            assert!(!Resource::is_dockerfile_name("Dockerfile.yaml"));
            assert!(!Resource::is_dockerfile_name("Dockerfile.json"));
            assert!(!Resource::is_dockerfile_name("base.yml"));
            assert!(!Resource::is_dockerfile_name("README.md"));
            assert!(!Resource::is_dockerfile_name("Containerfile"));
        }

        #[test]
        fn looks_like_dockerfile_for_file_and_url() {
            assert!(Resource::File(PathBuf::from("path/to/Dockerfile")).looks_like_dockerfile());
            assert!(
                Resource::File(PathBuf::from("./legacy/app.Dockerfile")).looks_like_dockerfile()
            );
            assert!(!Resource::File(PathBuf::from("dofigen.yml")).looks_like_dockerfile());

            let url: url::Url = "https://example.com/repo/Dockerfile".parse().unwrap();
            assert!(Resource::Url(url).looks_like_dockerfile());

            let url: url::Url = "https://example.com/repo/dofigen.yml".parse().unwrap();
            assert!(!Resource::Url(url).looks_like_dockerfile());
        }
    }

    #[cfg(feature = "parse")]
    mod extend_from_dockerfile {
        use super::*;
        use crate::{Dofigen, FromContext, GenerationContext, ImageName, ImageVersion};
        use assert_fs::TempDir;
        use assert_fs::prelude::*;
        use std::path::PathBuf;

        fn write_temp_dockerfile(content: &str) -> (TempDir, PathBuf) {
            let dir = TempDir::new().unwrap();
            let path = dir.child("Dockerfile");
            path.write_str(content).unwrap();
            let path = path.path().to_path_buf();
            (dir, path)
        }

        #[test]
        fn load_dockerfile_as_extend_patch() {
            let (_dir, path) = write_temp_dockerfile(
                r#"FROM alpine:3.19
WORKDIR /app
RUN echo hello
"#,
            );

            let mut context = DofigenContext::new();
            context.display_updates = false;

            let extend: Extend<crate::DofigenPatch> =
                Resource::File(path).load(&mut context).unwrap();

            let dofigen: Dofigen = extend.merge(&mut context).unwrap().into();

            assert_eq_sorted!(
                dofigen.stage.from,
                FromContext::FromImage(ImageName {
                    path: "alpine".into(),
                    version: Some(ImageVersion::Tag("3.19".into())),
                    ..Default::default()
                })
            );
            assert_eq_sorted!(dofigen.stage.workdir, Some("/app".into()));
            assert_eq_sorted!(dofigen.stage.run.run, vec!["echo hello".to_string()]);
        }

        #[test]
        fn merge_dofigen_overrides_on_top_of_dockerfile() {
            let dir = TempDir::new().unwrap();
            dir.child("Dockerfile")
                .write_str(
                    r#"FROM alpine:3.19 AS runtime
WORKDIR /app
USER app
CMD ["echo", "from-dockerfile"]
"#,
                )
                .unwrap();

            // Dofigen file next to the Dockerfile that extends it and overrides fields
            let dofigen_path = dir.child("dofigen.yml");
            dofigen_path
                .write_str(
                    r#"
extend:
  - Dockerfile
workdir: /opt/app
user: 1001
cmd:
  - echo
  - from-dofigen
"#,
                )
                .unwrap();

            let mut context = DofigenContext::new();
            context.display_updates = false;

            let dofigen = context
                .parse_from_resource(Resource::File(dofigen_path.path().to_path_buf()))
                .unwrap();

            assert_eq_sorted!(
                dofigen.stage.from,
                FromContext::FromImage(ImageName {
                    path: "alpine".into(),
                    version: Some(ImageVersion::Tag("3.19".into())),
                    ..Default::default()
                })
            );
            // Overrides win
            assert_eq_sorted!(dofigen.stage.workdir, Some("/opt/app".into()));
            assert_eq_sorted!(
                dofigen.stage.user,
                Some(crate::User {
                    user: "1001".into(),
                    group: None,
                })
            );
            assert_eq_sorted!(
                dofigen.cmd,
                vec!["echo".to_string(), "from-dofigen".to_string()]
            );
        }

        #[test]
        fn extend_dockerfile_with_multistage_and_append_run() {
            let dir = TempDir::new().unwrap();
            dir.child("Dockerfile.base")
                .write_str(
                    r#"FROM golang:1.22 AS builder
WORKDIR /src
RUN go build -o /out/app

FROM alpine:3.19 AS runtime
COPY --from=builder /out/app /bin/app
"#,
                )
                .unwrap();

            let dofigen_path = dir.child("app.yml");
            // Append an extra run command on top of the imported Dockerfile
            dofigen_path
                .write_str(
                    r#"
extend:
  - Dockerfile.base
run:
  +:
    - chmod +x /bin/app
entrypoint:
  - /bin/app
"#,
                )
                .unwrap();

            let mut context = DofigenContext::new();
            context.display_updates = false;

            let dofigen = context
                .parse_from_resource(Resource::File(dofigen_path.path().to_path_buf()))
                .unwrap();

            assert!(dofigen.builders.contains_key("builder"));
            assert_eq_sorted!(
                dofigen.stage.from,
                FromContext::FromImage(ImageName {
                    path: "alpine".into(),
                    version: Some(ImageVersion::Tag("3.19".into())),
                    ..Default::default()
                })
            );
            assert_eq_sorted!(dofigen.stage.run.run, vec!["chmod +x /bin/app".to_string()]);
            assert_eq_sorted!(dofigen.entrypoint, vec!["/bin/app".to_string()]);

            // Generated Dockerfile should still be valid multi-stage
            let dockerfile = GenerationContext::from(dofigen)
                .generate_dockerfile()
                .unwrap();
            assert!(dockerfile.contains("FROM golang:1.22 AS builder"));
            assert!(dockerfile.contains("FROM alpine:3.19 AS runtime"));
            assert!(dockerfile.contains("chmod +x /bin/app"));
        }

        #[test]
        fn extend_dockerfile_loads_companion_dockerignore() {
            let dir = TempDir::new().unwrap();
            dir.child("Dockerfile")
                .write_str("FROM alpine:3.19\n")
                .unwrap();
            dir.child(".dockerignore")
                .write_str("target\n*.log\n")
                .unwrap();

            let dofigen_path = dir.child("dofigen.yml");
            dofigen_path
                .write_str(
                    r#"
extend:
  - Dockerfile
"#,
                )
                .unwrap();

            let mut context = DofigenContext::new();
            context.display_updates = false;

            let dofigen = context
                .parse_from_resource(Resource::File(dofigen_path.path().to_path_buf()))
                .unwrap();

            assert_eq_sorted!(
                dofigen.ignore,
                vec!["target".to_string(), "*.log".to_string()]
            );
        }

        #[test]
        fn yaml_extend_still_works_alongside_dockerfile_support() {
            let dir = TempDir::new().unwrap();
            dir.child("base.yml")
                .write_str(
                    r#"
fromImage:
  path: alpine
  tag: "3.19"
workdir: /app
"#,
                )
                .unwrap();

            let dofigen_path = dir.child("dofigen.yml");
            dofigen_path
                .write_str(
                    r#"
extend:
  - base.yml
cmd:
  - sh
"#,
                )
                .unwrap();

            let mut context = DofigenContext::new();
            context.display_updates = false;

            let dofigen = context
                .parse_from_resource(Resource::File(dofigen_path.path().to_path_buf()))
                .unwrap();

            assert_eq_sorted!(dofigen.stage.workdir, Some("/app".into()));
            assert_eq_sorted!(dofigen.cmd, vec!["sh".to_string()]);
        }
    }
}

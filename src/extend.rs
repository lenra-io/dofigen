#[cfg(feature = "permissive")]
use crate::OneOrMany;
use crate::{dofigen_struct::*, DofigenContext, Error, Result};
use relative_path::RelativePath;
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize};
use std::iter;
use struct_patch::Merge;

#[cfg(feature = "permissive")]
type VecType<T> = OneOrMany<T>;

#[cfg(not(feature = "permissive"))]
type VecType<T> = Vec<T>;

/// Extends a list of resources with a patch
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
// #[serde(deny_unknown_fields)]
#[serde(default)]
#[cfg_attr(feature = "json_schema", derive(JsonSchema), schemars(default))]
pub struct Extend<T: Default> {
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
    fn load_resource_content(&self, context: &mut DofigenContext) -> Result<String> {
        let resource = match self {
            Resource::File(path) => {
                if path.is_absolute() {
                    Resource::File(path.clone())
                } else {
                    if let Some(current_resource) = context.current_resource() {
                        match current_resource {
                            Resource::File(file) => {
                                let current_file_relative_path =
                                    RelativePath::from_path(file).map_err(Error::display)?;
                                let relative_path =
                                    RelativePath::from_path(path).map_err(Error::display)?;
                                let relative_path = current_file_relative_path
                                    .join("..")
                                    .join_normalized(relative_path);
                                Resource::File(relative_path.to_path(""))
                            }
                            Resource::Url(url) => {
                                Resource::Url(url.join(path.to_str().unwrap()).unwrap())
                            }
                        }
                    } else {
                        Resource::File(path.clone())
                    }
                }
            }
            Resource::Url(url) => Resource::Url(url.clone()),
        };

        // push the resource to the stack
        context.push_resource_stack(resource.clone())?;

        // load the resource content
        context.get_resource_content(resource)
    }

    pub fn load<T>(&self, context: &mut DofigenContext) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Ok(
            serde_yaml::from_str(self.load_resource_content(context)?.as_str()).map_err(|err| {
                Error::Custom(format!(
                    "Could not deserialize resource {:?}: {}",
                    self, err
                ))
            })?,
        )
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

            #[derive(Deserialize, Patch)]
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
            }
        }

        mod extend_image {
            use crate::{ImageNamePatch, ImagePatch, RunPatch, StagePatch};

            use super::*;

            #[test]
            fn empty() {
                let data = r#"{}"#;

                let extend_image: Extend<ImagePatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        value: ImagePatch {
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
from:
  path: ubuntu
"#;

                let extend_image: Extend<ImagePatch> = serde_yaml::from_str(data).unwrap();

                assert_eq_sorted!(
                    extend_image,
                    Extend {
                        value: ImagePatch {
                            stage: Some(StagePatch {
                                from: Some(Some(
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
}

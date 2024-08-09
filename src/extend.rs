#[cfg(feature = "permissive")]
use crate::OneOrMany;
use crate::{dofigen_struct::*, Error, Result, VecPatch};
#[cfg(feature = "json_schema")]
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize};
use std::{collections::HashMap, fs, ops::Add};

const MAX_LOAD_STACK_SIZE: usize = 10;

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
    P: Default + DeserializeOwned + Clone + Add<P, Output = P>,
{
    pub fn merge(&self, context: &mut LoadContext) -> Result<P> {
        if self.extend.is_empty() {
            return Ok(self.value.clone().into());
        }

        // load extends files
        let merged: Option<P> = self
            .extend
            .iter()
            .map(|extend| {
                let ret = extend.load::<Self>(context)?.merge(context)?;
                context.load_resource_stack.pop();
                Ok(ret)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .reduce(|a, b| a + b);

        Ok(if let Some(merged) = merged {
            merged + self.value.clone()
        } else {
            self.value.clone()
        })
    }
}

pub struct LoadContext {
    load_resource_stack: Vec<Resource>,
    resources: HashMap<Resource, String>,
}

impl LoadContext {
    pub fn new() -> Self {
        Self {
            load_resource_stack: vec![],
            resources: HashMap::new(),
        }
    }

    pub fn from_resource(resource: Resource) -> Self {
        Self {
            load_resource_stack: vec![resource],
            resources: HashMap::new(),
        }
    }
}

impl Resource {
    fn load_resource_content(&self, context: &mut LoadContext) -> Result<String> {
        let resource = match self {
            Resource::File(path) => {
                if path.is_absolute() {
                    Resource::File(path.clone())
                } else {
                    if let Some(current_resource) = context.load_resource_stack.last() {
                        match current_resource {
                            Resource::File(file) => Resource::File(
                                file.parent()
                                    .ok_or(Error::Custom(format!(
                                        "The current resource does not have parent dir {:?}",
                                        file
                                    )))?
                                    .join(path),
                            ),
                            Resource::Url(url) => {
                                Resource::Url(url.join(path.to_str().unwrap()).unwrap())
                            }
                        }
                    } else {
                        Resource::File(path.canonicalize().unwrap())
                    }
                }
            }
            Resource::Url(url) => Resource::Url(url.clone()),
        };
        if context.load_resource_stack.contains(&resource) {
            // push the resource to format the error message
            context.load_resource_stack.push(resource.clone());
            return Err(Error::Custom(format!(
                "Circular dependency detected while loading resource {}",
                context
                    .load_resource_stack
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            )));
        }

        // push the resource to the stack
        context.load_resource_stack.push(resource.clone());

        // check the stack size
        if context.load_resource_stack.len() > MAX_LOAD_STACK_SIZE {
            return Err(Error::Custom(format!(
                "Max load stack size exceeded while loading resource {}",
                context
                    .load_resource_stack
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            )));
        }

        // load the resource content
        match resource.clone() {
            Resource::File(path) => {
                if let Some(value) = context.resources.get(&resource) {
                    Ok(value.clone())
                } else {
                    let str = fs::read_to_string(path.clone()).map_err(|err| {
                        Error::Custom(format!("Could not read file {:?}: {}", path, err))
                    })?;
                    context.resources.insert(resource, str.clone());
                    Ok(str)
                }
            }
            Resource::Url(url) => {
                if let Some(value) = context.resources.get(&resource) {
                    Ok(value.clone())
                } else {
                    let response = reqwest::blocking::get(url.as_ref()).map_err(|err| {
                        Error::Custom(format!("Could not get url {:?}: {}", url, err))
                    })?;
                    Ok(response.text().map_err(|err| {
                        Error::Custom(format!(
                            "Could not read response from url {:?}: {}",
                            url, err
                        ))
                    })?)
                }
            }
        }
    }

    pub fn load<T>(&self, context: &mut LoadContext) -> Result<T>
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

macro_rules! add_patch {
    ($opt_a: expr, $opt_b: expr) => {
        match ($opt_a, $opt_b) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    };
}

macro_rules! add_optional_add {
    ($opt_a: expr, $opt_b: expr) => {
        match ($opt_a, $opt_b) {
            (Some(Some(a)), Some(Some(b))) => Some(Some(a + b)),
            (_, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        }
    };
}

macro_rules! add_option {
    ($opt_a: expr, $opt_b: expr) => {
        match ($opt_a, $opt_b) {
            (_, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (None, None) => None,
        }
    };
}

impl Add<Self> for ImagePatch {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            stage: add_patch!(self.stage, rhs.stage),
            context: add_patch!(self.context, rhs.context),
            ignore: add_patch!(self.ignore, rhs.ignore),
            builders: add_patch!(self.builders, rhs.builders),
            entrypoint: add_patch!(self.entrypoint, rhs.entrypoint),
            cmd: add_patch!(self.cmd, rhs.cmd),
            expose: add_patch!(self.expose, rhs.expose),
            healthcheck: add_optional_add!(self.healthcheck, rhs.healthcheck),
        }
    }
}

impl Add<Self> for StagePatch {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            from: add_optional_add!(self.from, rhs.from),
            run: add_patch!(self.run, rhs.run),
            name: add_option!(self.name, rhs.name),
            user: add_optional_add!(self.user, rhs.user),
            workdir: add_option!(self.workdir, rhs.workdir),
            env: match (self.env, rhs.env) {
                (Some(a), Some(b)) => {
                    // TODO: merge maps
                    todo!()
                },
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
            artifacts: add_patch!(self.artifacts, rhs.artifacts),
            copy: add_patch!(self.copy, rhs.copy),
            root: add_optional_add!(self.root, rhs.root),
        }
    }
}

impl Add<Self> for RunPatch {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            run: add_patch!(self.run, rhs.run),
            cache: add_patch!(self.cache, rhs.cache),
        }
    }
}

impl Add<Self> for ImageNamePatch {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            host: add_option!(self.host, rhs.host),
            path: add_option!(self.path, rhs.path),
            version: add_option!(self.version, rhs.version),
            port: add_option!(self.port, rhs.port),
        }
    }
}

impl Add<Self> for HealthcheckPatch {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            cmd: add_option!(self.cmd, rhs.cmd),
            interval: add_option!(self.interval, rhs.interval),
            timeout: add_option!(self.timeout, rhs.timeout),
            start: add_option!(self.start, rhs.start),
            retries: add_option!(self.retries, rhs.retries),
        }
    }
}

impl Add<Self> for UserPatch {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            user: add_option!(self.user, rhs.user),
            group: add_option!(self.group, rhs.group),
        }
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

            impl Add<Self> for TestStructPatch {
                type Output = Self;

                fn add(self, rhs: Self) -> Self {
                    Self {
                        name: rhs.name.or(self.name),
                        sub: match (self.sub, rhs.sub) {
                            (Some(a), Some(b)) => Some(a + b),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        },
                    }
                }
            }

            impl Add<Self> for TestSubStructPatch {
                type Output = Self;

                fn add(self, rhs: Self) -> Self {
                    Self {
                        level: rhs.level.or(self.level),
                    }
                }
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

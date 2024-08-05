use std::{collections::HashMap, fs};

use serde::de::DeserializeOwned;
use struct_patch::Patch;

use crate::{
    dofigen_struct::Stage, generator::GenerationContext, script_runner::ScriptRunner, Error,
    Extend, ImageName, ImagePatch, Resource, Result, User,
};

pub trait BaseStage: ScriptRunner {
    fn name(&self, context: &GenerationContext) -> String;
    fn from(&self, context: &GenerationContext) -> ImageName;
    fn user(&self, context: &GenerationContext) -> Option<User>;
}

impl BaseStage for Stage {
    fn name(&self, context: &GenerationContext) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| context.default_stage_name.clone())
    }
    fn from(&self, context: &GenerationContext) -> ImageName {
        self.from.clone().unwrap_or(context.default_from.clone())
    }

    fn user(&self, context: &GenerationContext) -> Option<User> {
        self.user.clone().or(context.user.clone())
    }
}

impl<P> Extend<P>
where
    P: DeserializeOwned + Clone,
{
    pub fn merge<T>(&self, context: &mut LoadContext) -> Result<T>
    where
        T: Patch<P> + From<P>,
    {
        if self.extend.is_empty() {
            return Ok(self.value.clone().into());
        }

        // load extends files
        let mut patches: Vec<T> = self
            .extend
            .iter()
            .map(|extend| extend.load::<Self>(context)?.merge(context))
            .collect::<Result<Vec<_>>>()?;

        // for each extends file, merge it with self
        let mut merged = patches.remove(0);
        for patch in patches {
            merged.apply(patch.into_patch());
        }
        merged.apply(self.value.clone());
        Ok(merged)
    }
}

pub struct LoadContext {
    resources: HashMap<String, String>,
}

impl LoadContext {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }
}

impl Resource {
    pub fn load<T>(&self, context: &mut LoadContext) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let ret = match self {
            Resource::File(path) => {
                let canonical_path = fs::canonicalize(path)
                    .map_err(|err| {
                        Error::Custom(format!("Could not canonicalize path {:?}: {}", path, err))
                    })?
                    .to_str()
                    .unwrap()
                    .to_string();
                let str = if let Some(value) = context.resources.get(&canonical_path) {
                    value.clone()
                } else {
                    fs::read_to_string(path).map_err(|err| {
                        Error::Custom(format!("Could not read file {:?}: {}", path, err))
                    })?
                };
                context.resources.insert(canonical_path, str.clone());
                serde_yaml::from_str(str.as_str()).map_err(Error::from)?
            }
            Resource::Url(url) => todo!(),
        };
        Ok(ret)
    }
}

impl User {
    pub fn uid(&self) -> Option<u16> {
        self.user.parse::<u16>().ok()
    }

    pub fn gid(&self) -> Option<u16> {
        self.group
            .as_ref()
            .map(|group| group.parse::<u16>().ok())
            .flatten()
    }

    pub fn into(&self) -> String {
        let name = self.user.clone();
        match &self.group {
            Some(group) => format!("{}:{}", name, group),
            _ => name,
        }
    }

    // Static methods

    pub fn new(user: &str) -> Self {
        Self {
            user: user.into(),
            group: Some(user.into()),
        }
    }

    pub fn new_without_group(user: &str) -> Self {
        Self {
            user: user.into(),
            group: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::ImageName;

    use super::*;

    #[test]
    fn test_builder_name_with_name() {
        let builder = Stage {
            name: Some(String::from("my-builder")),
            ..Default::default()
        };
        let name = builder.name(&GenerationContext {
            previous_builders: vec!["builder-0".into()],
            ..Default::default()
        });
        assert_eq_sorted!(name, "my-builder");
    }

    #[test]
    fn test_builder_name_without_name() {
        let builder = Stage::default();
        let name = builder.name(&GenerationContext {
            previous_builders: vec!["builder-0".into(), "bob".into()],
            default_stage_name: "builder-2".into(),
            ..Default::default()
        });
        assert_eq_sorted!(name, "builder-2");
    }

    #[test]
    fn test_builder_user_with_user() {
        let builder = Stage {
            user: Some(
                User {
                    user: "my-user".into(),
                    ..Default::default()
                }
                .into(),
            ),
            ..Default::default()
        };
        let user = builder.user(&GenerationContext::default());
        assert_eq_sorted!(
            user,
            Some(User {
                user: "my-user".into(),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_builder_user_without_user() {
        let builder = Stage::default();
        let user = builder.user(&GenerationContext::default());
        assert_eq_sorted!(user, None);
    }

    #[test]
    fn test_image_name() {
        let stage = Stage {
            from: Some(
                ImageName {
                    path: String::from("my-image"),
                    ..Default::default()
                }
                .into(),
            ),
            ..Default::default()
        };
        let name = stage.name(&GenerationContext {
            previous_builders: vec!["builder-0".into(), "bob".into(), "john".into()],
            default_stage_name: "runtime".into(),
            ..Default::default()
        });
        assert_eq_sorted!(name, "runtime");
    }

    #[test]
    fn test_image_user_with_user() {
        let stage = Stage {
            user: Some(
                User {
                    user: "my-user".into(),
                    ..Default::default()
                }
                .into(),
            ),
            from: Some(
                ImageName {
                    path: String::from("my-image"),
                    ..Default::default()
                }
                .into(),
            ),
            ..Default::default()
        };
        let user = stage.user(&GenerationContext::default());
        assert_eq_sorted!(
            user,
            Some(User {
                user: "my-user".into(),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_image_user_without_user() {
        let stage = Stage {
            from: Some(
                ImageName {
                    path: String::from("my-image"),
                    ..Default::default()
                }
                .into(),
            ),
            ..Default::default()
        };
        let user = stage.user(&GenerationContext {
            user: Some(User::new("1000")),
            ..Default::default()
        });
        assert_eq_sorted!(
            user,
            Some(User {
                user: "1000".into(),
                group: Some("1000".into()),
            })
        );
    }
}

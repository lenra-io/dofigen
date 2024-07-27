use std::{collections::HashMap, fs};

use serde::de::DeserializeOwned;
use struct_patch::Patch;

use crate::{
    dofigen_struct::Stage, generator::GenerationContext, script_runner::ScriptRunner, Error,
    Extend, ImageName, Resource, Result, User,
};

pub trait BaseStage: ScriptRunner {
    fn name(&self, context: &GenerationContext) -> String;
    fn from(&self, context: &GenerationContext) -> ImageName;
    fn user(&self) -> Option<User>;
}

impl BaseStage for Stage {
    fn name(&self, context: &GenerationContext) -> String {
        match self.name.as_ref() {
            Some(name) => String::from(name),
            _ => format!("builder-{}", context.previous_builders.len()),
        }
    }
    fn from(&self, context: &GenerationContext) -> ImageName {
        self.from.clone().unwrap_or(context.default_from.clone())
    }

    fn user(&self) -> Option<User> {
        self.user.as_ref().map(|user| user.clone())
    }
}

impl<'de, P> Extend<P>
where
    P: DeserializeOwned,
{
    pub fn merge<T>(self, context: &mut LoadContext) -> Result<T>
    where
        T: Patch<P> + DeserializeOwned + Default,
    {
        let extends = self.extend.to_vec();
        if extends.is_empty() {
            let mut ret = T::default();
            ret.apply(self.value);
            return Ok(ret);
        }

        // load extends files
        let mut patchs: Vec<T> = extends
            .into_iter()
            .map(|extend| extend.load::<Self>(context)?.merge(context))
            .collect::<Result<Vec<_>>>()?;

        // for each extends file, merge it with self
        let mut merged = patchs.remove(0);
        for patch in patchs {
            merged.apply(patch.into_patch());
        }
        merged.apply(self.value);
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
        assert_eq!(name, "my-builder");
    }

    #[test]
    fn test_builder_name_without_name() {
        let builder = Stage::default();
        let name = builder.name(&GenerationContext {
            previous_builders: vec!["builder-0".into(), "bob".into()],
            ..Default::default()
        });
        assert_eq!(name, "builder-2");
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
        let user = builder.user();
        assert_eq!(
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
        let user = builder.user();
        assert_eq!(user, None);
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
            ..Default::default()
        });
        assert_eq!(name, "runtime");
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
        let user = stage.user();
        assert_eq!(
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
        let user = stage.user();
        assert_eq!(
            user,
            Some(User {
                user: "1000".into(),
                group: Some("1000".into()),
            })
        );
    }
}

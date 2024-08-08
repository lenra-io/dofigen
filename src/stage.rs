use crate::{
    dofigen_struct::Stage, generator::GenerationContext, script_runner::ScriptRunner, Error,
    Extend, ImageName, Resource, Result, User,
};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, fs};
use struct_patch::Patch;

const MAX_LOAD_STACK_SIZE: usize = 10;

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
        P: Default,
    {
        if self.extend.is_empty() {
            return Ok(self.value.clone().into());
        }

        // load extends files
        let mut patches: Vec<T> = self
            .extend
            .iter()
            .map(|extend| {
                let ret = extend.load::<Self>(context)?.merge(context)?;
                context.load_resource_stack.pop();
                Ok(ret)
            })
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

use std::{collections::HashMap, ops::Deref};

use crate::{
    dofigen_struct::{Builder, Image},
    generator::GenerationContext,
    script_runner::ScriptRunner,
    Artifact, CopyResource, ImageName, PermissiveStruct, Root, User,
};

pub trait BaseStage: ScriptRunner {
    fn name(&self, context: &GenerationContext) -> String;
    fn from(&self) -> ImageName;
    fn user(&self) -> Option<User>;
}
pub trait Stage: BaseStage {
    fn workdir(&self) -> Option<&String>;
    fn env(&self) -> Option<&HashMap<String, String>>;
    fn copy(&self) -> std::vec::Vec<PermissiveStruct<CopyResource>>;
    fn artifacts(&self) -> std::vec::Vec<Artifact>;
    fn root(&self) -> Option<&Root>;
}

impl BaseStage for Builder {
    fn name(&self, context: &GenerationContext) -> String {
        match self.name.as_ref() {
            Some(name) => String::from(name),
            _ => format!("builder-{}", context.previous_builders.len()),
        }
    }
    fn from(&self) -> ImageName {
        self.from.deref().clone()
    }

    fn user(&self) -> Option<User> {
        self.user.as_ref().map(|user| user.deref().clone())
    }
}

impl BaseStage for Image {
    fn name(&self, _context: &GenerationContext) -> String {
        String::from("runtime")
    }
    fn from(&self) -> ImageName {
        if let Some(image_name) = &self.from {
            image_name.deref().clone()
        } else {
            ImageName {
                path: String::from("scratch"),
                ..Default::default()
            }
        }
    }
    fn user(&self) -> Option<User> {
        self.user
            .as_ref()
            .map(|user| user.deref().clone())
            .or(Some(User::new("1000")))
    }
}

macro_rules! impl_Stage {
    (for $($t:ty),+) => {
        $(impl Stage for $t {
            fn workdir(&self) -> Option<&String> {
                self.workdir.as_ref()
            }

            fn env(&self) -> Option<&HashMap<String, String>> {
                self.env.as_ref()
            }

            fn copy(&self) -> Vec<PermissiveStruct<CopyResource>> {
                self.copy.to_vec()
            }

            fn artifacts(&self) -> Vec<Artifact> {
                self.artifacts.to_vec()
            }

            fn root(&self) -> Option<&Root> {
                self.root.as_ref()
            }
        })*
    }
}

impl_Stage!(for Builder, Image);

impl Image {
    pub fn apply_extends(&self) -> &Self {
        let extends = self.extend.to_vec();
        if extends.is_empty() {
            return self;
        }
        // TODO: load extends files

        // TODO: for each extends file, merge it with self
        todo!()
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
    use crate::{ImageName, PermissiveStruct};

    use super::*;

    #[test]
    fn test_builder_name_with_name() {
        let builder = Builder {
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
        let builder = Builder::default();
        let name = builder.name(&GenerationContext {
            previous_builders: vec!["builder-0".into(), "bob".into()],
            ..Default::default()
        });
        assert_eq!(name, "builder-2");
    }

    #[test]
    fn test_builder_user_with_user() {
        let builder = Builder {
            user: Some(PermissiveStruct::new(User {
                user: "my-user".into(),
                ..Default::default()
            })),
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
        let builder = Builder::default();
        let user = builder.user();
        assert_eq!(user, None);
    }

    #[test]
    fn test_image_name() {
        let image = Image {
            from: Some(PermissiveStruct::new(ImageName {
                path: String::from("my-image"),
                ..Default::default()
            })),
            ..Default::default()
        };
        let name = image.name(&GenerationContext {
            previous_builders: vec!["builder-0".into(), "bob".into(), "john".into()],
            ..Default::default()
        });
        assert_eq!(name, "runtime");
    }

    #[test]
    fn test_image_user_with_user() {
        let image = Image {
            user: Some(PermissiveStruct::new(User {
                user: "my-user".into(),
                ..Default::default()
            })),
            from: Some(PermissiveStruct::new(ImageName {
                path: String::from("my-image"),
                ..Default::default()
            })),
            ..Default::default()
        };
        let user = image.user();
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
        let image = Image {
            from: Some(PermissiveStruct::new(ImageName {
                path: String::from("my-image"),
                ..Default::default()
            })),
            ..Default::default()
        };
        let user = image.user();
        assert_eq!(
            user,
            Some(User {
                user: "1000".into(),
                group: Some("1000".into()),
            })
        );
    }
}

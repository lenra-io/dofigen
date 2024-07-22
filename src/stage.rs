use std::{collections::HashMap, ops::Deref};

use crate::{
    dofigen_struct::{Builder, Image},
    generator::GenerationContext,
    merge::OptionalField,
    script_runner::ScriptRunner,
    Artifact, CopyResource, ImageName, PermissiveStruct, Root, User,
};

pub trait BaseStage: ScriptRunner {
    fn name(&self, context: &GenerationContext) -> String;
    fn from(&self) -> ImageName;
    fn user(&self) -> Option<User>;
}
pub trait Stage: BaseStage {
    fn workdir(&self) -> OptionalField<&String>;
    fn env(&self) -> OptionalField<&HashMap<String, String>>;
    fn copy(&self) -> OptionalField<&Vec<PermissiveStruct<CopyResource>>>;
    fn artifacts(&self) -> OptionalField<&Vec<Artifact>>;
    fn root(&self) -> OptionalField<&Root>;
}

impl BaseStage for Builder {
    fn name(&self, context: &GenerationContext) -> String {
        match self.name.as_ref() {
            OptionalField::Present(name) => String::from(name),
            _ => format!("builder-{}", context.previous_builders.len()),
        }
    }
    fn from(&self) -> ImageName {
        self.from
            .as_ref()
            .expect("Builder must have a from field")
            .deref()
            .clone()
    }

    fn user(&self) -> Option<User> {
        self.user
            .as_ref()
            .to_option()
            .map(|user| user.deref().clone())
    }
}

impl BaseStage for Image {
    fn name(&self, _context: &GenerationContext) -> String {
        String::from("runtime")
    }
    fn from(&self) -> ImageName {
        if let OptionalField::Present(image_name) = &self.from {
            image_name.deref().clone()
        } else {
            ImageName {
                path: OptionalField::Present(String::from("scratch")),
                ..Default::default()
            }
        }
    }
    fn user(&self) -> Option<User> {
        self.user
            .as_ref()
            .to_option()
            .map(|user| user.deref().clone())
            .or(Some(User::new("1000")))
    }
}

macro_rules! impl_Stage {
    (for $($t:ty),+) => {
        $(impl Stage for $t {
            fn workdir(&self) -> OptionalField<&String> {
                self.workdir.as_ref()
            }

            fn env(&self) -> OptionalField<&HashMap<String, String>> {
                self.env.as_ref()
            }

            fn copy(&self) -> OptionalField<&Vec<PermissiveStruct<CopyResource>>> {
                self.copy.as_ref().map(|vec|vec.deref())
            }

            fn artifacts(&self) -> OptionalField<&Vec<Artifact>> {
                self.artifacts.as_ref()
            }

            fn root(&self) -> OptionalField<&Root> {
                self.root.as_ref()
            }
        })*
    }
}

impl_Stage!(for Builder, Image);

impl Image {
    pub fn apply_extends(&self) -> &Self {
        if let OptionalField::Present(extends) = &self.extend {
            let extends = extends.to_vec();
            // TODO: load extends files

            // TODO: for each extends file, merge it with self
            todo!()
        } else {
            self
        }
    }
}

impl User {
    pub fn uid(&self) -> Option<u16> {
        self.user
            .as_ref()
            .to_option()
            .expect("User user field is required")
            .parse::<u16>()
            .ok()
    }

    pub fn gid(&self) -> Option<u16> {
        self.group
            .as_ref()
            .to_option()
            .map(|group| group.parse::<u16>().ok())
            .flatten()
    }

    pub fn into(&self) -> String {
        let name = self.user.clone().expect("User user field is required");
        match &self.group {
            OptionalField::Present(group) => format!("{}:{}", name, group),
            _ => name,
        }
    }

    // Static methods

    pub fn new(user: &str) -> Self {
        Self {
            user: OptionalField::Present(user.into()),
            group: OptionalField::Present(user.into()),
        }
    }

    pub fn new_without_group(user: &str) -> Self {
        Self {
            user: OptionalField::Present(user.into()),
            group: OptionalField::Missing,
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
            name: OptionalField::Present(String::from("my-builder")),
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
            user: OptionalField::Present(PermissiveStruct::new(User {
                user: OptionalField::Present("my-user".into()),
                ..Default::default()
            })),
            ..Default::default()
        };
        let user = builder.user();
        assert_eq!(
            user,
            Some(User {
                user: OptionalField::Present("my-user".into()),
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
            from: OptionalField::Present(PermissiveStruct::new(ImageName {
                path: OptionalField::Present(String::from("my-image")),
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
            user: OptionalField::Present(PermissiveStruct::new(User {
                user: OptionalField::Present("my-user".into()),
                ..Default::default()
            })),
            from: OptionalField::Present(PermissiveStruct::new(ImageName {
                path: OptionalField::Present(String::from("my-image")),
                ..Default::default()
            })),
            ..Default::default()
        };
        let user = image.user();
        assert_eq!(
            user,
            Some(User {
                user: OptionalField::Present("my-user".into()),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_image_user_without_user() {
        let image = Image {
            from: OptionalField::Present(PermissiveStruct::new(ImageName {
                path: OptionalField::Present(String::from("my-image")),
                ..Default::default()
            })),
            ..Default::default()
        };
        let user = image.user();
        assert_eq!(
            user,
            Some(User {
                user: OptionalField::Present("1000".into()),
                group: OptionalField::Present("1000".into()),
            })
        );
    }
}

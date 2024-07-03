use crate::{Add, AddGitRepo, Chown, Copy, CopyResources, ImageName, ImageVersion, Result};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenerationContext {
    pub user: Option<String>,
}

pub trait DockerfileGenerator {
    fn to_dockerfile_content(&self, context: &GenerationContext) -> Result<String>;
}

impl DockerfileGenerator for ImageName {
    fn to_dockerfile_content(&self, _context: &GenerationContext) -> Result<String> {
        let mut registry = String::new();
        if let Some(host) = &self.host {
            registry.push_str(host);
            if self.port.is_some() {
                registry.push_str(":");
                registry.push_str(self.port.unwrap().to_string().as_str());
            }
            registry.push_str("/");
        }
        let mut version = String::new();
        match &self.version {
            Some(ImageVersion::Tag(tag)) => {
                version.push_str(":");
                version.push_str(tag);
            }
            Some(ImageVersion::Digest(digest)) => {
                version.push_str("@");
                version.push_str(digest);
            }
            None => {}
        }
        Ok(format!("{registry}{path}{version}", path = self.path))
    }
}

impl DockerfileGenerator for CopyResources {
    fn to_dockerfile_content(&self, context: &GenerationContext) -> Result<String> {
        match self {
            CopyResources::Copy(copy) => copy.to_dockerfile_content(context),
            CopyResources::Add(add_web_file) => add_web_file.to_dockerfile_content(context),
            CopyResources::AddGitRepo(add_git_repo) => add_git_repo.to_dockerfile_content(context),
        }
    }
}

impl DockerfileGenerator for Copy {
    fn to_dockerfile_content(&self, context: &GenerationContext) -> Result<String> {
        let paths = self.paths.clone().to_vec().join(" ");
        let target = self.target.clone().unwrap_or("./".to_string());
        let mut options = String::new();
        push_conditional_str_option(&mut options, "from", &self.from);
        push_chown_option(
            &mut options,
            &self
                .chown
                .clone()
                .or(context.user.clone().map(|user| Chown {
                    user,
                    ..Default::default()
                })),
        );
        push_conditional_str_option(&mut options, "chmod", &self.chmod);
        if let Some(exclude) = &self.exclude {
            for path in exclude.clone().to_vec() {
                push_str_option(&mut options, "exclude", &path);
            }
        }
        push_bool_option(&mut options, "link", &self.link.unwrap_or(true));
        push_conditional_bool_option(&mut options, "parents", &self.parents);
        Ok(format!("COPY{options} {paths} {target}"))
    }
}

impl DockerfileGenerator for Add {
    fn to_dockerfile_content(&self, context: &GenerationContext) -> Result<String> {
        let urls = self.paths.clone().to_vec().join(" ");
        let mut options = String::new();
        push_chown_option(
            &mut options,
            &self
                .chown
                .clone()
                .or(context.user.clone().map(|user| Chown {
                    user,
                    ..Default::default()
                })),
        );
        push_conditional_str_option(&mut options, "chmod", &self.chmod);
        push_bool_option(&mut options, "link", &self.link.unwrap_or(true));
        Ok(format!(
            "ADD{options} {urls} {target}",
            target = self.target.clone().unwrap_or(".".to_string())
        ))
    }
}

impl DockerfileGenerator for AddGitRepo {
    fn to_dockerfile_content(&self, context: &GenerationContext) -> Result<String> {
        let mut options = String::new();
        push_chown_option(
            &mut options,
            &self
                .chown
                .clone()
                .or(context.user.clone().map(|user| Chown {
                    user,
                    ..Default::default()
                })),
        );
        push_conditional_str_option(&mut options, "chmod", &self.chmod);
        if let Some(exclude) = &self.exclude {
            for path in exclude.clone().to_vec() {
                push_str_option(&mut options, "exclude", &path);
            }
        }
        push_bool_option(&mut options, "link", &self.link.unwrap_or(true));
        Ok(format!(
            "ADD{options} {repo} {target}",
            repo = self.repo,
            target = self.target.clone().unwrap_or(".".to_string())
        ))
    }
}

// Push option functions

pub fn push_chown(options: &mut String, chown: &Chown) {
    options.push_str(" --chown=");
    options.push_str(chown.user.as_str());
    if let Some(group) = &chown.group {
        options.push_str(":");
        options.push_str(group);
    }
}

pub fn push_chown_option(options: &mut String, chown: &Option<Chown>) {
    if let Some(c) = chown {
        push_chown(options, c);
    }
}

pub fn push_conditional_str_option(options: &mut String, name: &str, value: &Option<String>) {
    if let Some(v) = value {
        push_str_option(options, name, v);
    }
}

pub fn push_str_option(options: &mut String, name: &str, value: &String) {
    options.push_str(" --");
    options.push_str(name);
    options.push_str("=");
    options.push_str(value);
}

pub fn push_conditional_bool_option(options: &mut String, name: &str, value: &Option<bool>) {
    if let Some(v) = value {
        push_bool_option(options, name, v);
    }
}

pub fn push_bool_option(options: &mut String, name: &str, &value: &bool) {
    if value {
        options.push_str(" --");
        options.push_str(name);
    }
}

// #[cfg(test)]
// mod test {
//     use crate::*;

// mod builder {
//     use super::*;

//     #[test]
//     fn name_with_name() {
//         let builder = Builder {
//             name: Some(String::from("my-builder")),
//             ..Default::default()
//         };
//         let position = 1;
//         let name = builder.name(position);
//         assert_eq!(name, "my-builder");
//     }

//     #[test]
//     fn name_without_name() {
//         let builder = Builder::default();
//         let position = 2;
//         let name = builder.name(position);
//         assert_eq!(name, "builder-2");
//     }

//     #[test]
//     fn user_with_user() {
//         let builder = Builder {
//             user: Some(String::from("my-user")),
//             ..Default::default()
//         };
//         let user = builder.user();
//         assert_eq!(user, Some(String::from("my-user")));
//     }

//     #[test]
//     fn user_without_user() {
//         let builder = Builder::default();
//         let user = builder.user();
//         assert_eq!(user, None);
//     }
// }

// mod image_name {
//     use super::*;

//     #[test]
//     fn test_image_name() {
//         let image = Image {
//             from: Some(ImageName {
//                 path: String::from("my-image"),
//                 ..Default::default()
//             }),
//             ..Default::default()
//         };
//         let position = 3;
//         let name = image.name(position);
//         assert_eq!(name, "runtime");
//     }

//     #[test]
//     fn test_image_user_with_user() {
//         let image = Image {
//             user: Some(String::from("my-user")),
//             from: Some(ImageName {
//                 path: String::from("my-image"),
//                 ..Default::default()
//             }),
//             ..Default::default()
//         };
//         let user = image.user();
//         assert_eq!(user, Some(String::from("my-user")));
//     }

//     #[test]
//     fn test_image_user_without_user() {
//         let image = Image {
//             from: Some(ImageName {
//                 path: String::from("my-image"),
//                 ..Default::default()
//             }),
//             ..Default::default()
//         };
//         let user = image.user();
//         assert_eq!(user, Some(String::from("1000")));
//     }
// }
// }

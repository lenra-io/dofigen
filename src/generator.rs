use std::fmt::Write;

use crate::{
    dockerfile::{
        DockerfileInsctruction, DockerfileLine, InstructionOption, InstructionOptionOption,
    },
    runners::ScriptRunner,
    string_vec_to_string, Add, AddGitRepo, Chown, Copy, CopyResources, Error, Image, ImageName,
    ImageVersion, Result, StageGenerator,
};

trait CommandOption {
    fn command_option_to_str(&self) -> &str;
}

trait CommandOptionParameter {
    fn command_option_parameter_to_str(&self) -> &str;
}

pub struct GenerationContext {
    pub user: Option<String>,
    pub previous_builders: Vec<String>,
}

// impl std::fmt::Write for GenerationContext {
//     fn write_str(&mut self, s: &str) -> std::fmt::Result {
//         self.writer
//             .write_all(s.as_bytes())
//             .map_err(|_err| std::fmt::Error::default())
//     }

//     fn write_char(&mut self, c: char) -> std::fmt::Result {
//         self.writer
//             .write_all(c.to_string().as_bytes())
//             .map_err(|_err| std::fmt::Error::default())
//     }

//     fn write_fmt(&mut self, args: std::fmt::Arguments) -> std::fmt::Result {
//         self.writer
//             .write_all(std::fmt::format(args).as_bytes())
//             .map_err(|_err| std::fmt::Error::default())
//     }
// }

// impl GenerationContext {
//     // Push option functions

//     pub fn write_chown_option(&mut self, chown: &Chown) {
//         self.write_str(" --chown=");
//         self.write_str(chown.user.as_str());
//         if let Some(group) = &chown.group {
//             self.write_str(":");
//             self.write_str(group);
//         }
//     }

//     pub fn write_optional_chown_option(&mut self, chown: &Option<Chown>) {
//         if let Some(c) = chown {
//             self.write_chown_option(c);
//         }
//     }

//     pub fn write_conditional_str_option(&mut self, name: &str, value: &Option<String>) {
//         if let Some(v) = value {
//             self.write_str_option(name, v);
//         }
//     }

//     pub fn write_str_option(&mut self, name: &str, value: &String) {
//         self.write_str(" --");
//         self.write_str(name);
//         self.write_str("=");
//         self.write_str(value);
//     }

//     pub fn write_conditional_bool_option(&mut self, name: &str, value: &Option<bool>) {
//         if let Some(v) = value {
//             self.write_bool_option(name, v);
//         }
//     }

//     pub fn write_bool_option(&mut self, name: &str, &value: &bool) {
//         if value {
//             self.write_str(" --");
//             self.write_str(name);
//         }
//     }

//     pub fn new(writer: &'static mut dyn std::io::Write, user: Option<String>) -> Self {
//         GenerationContext { writer, user }
//     }
// }

pub trait DockerfileGenerator {
    // fn generate_content(&self, context: &mut GenerationContext) -> Result<()>;
    fn generate_dockerfile_lines(&self, context: GenerationContext) -> Result<Vec<DockerfileLine>>;
}

impl ToString for ImageName {
    fn to_string(&self) -> String {
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
        format!(
            "{registry}{path}{version}",
            path = self.path,
            registry = registry,
            version = version
        )
    }
}

impl ToString for Chown {
    fn to_string(&self) -> String {
        let mut chown = String::new();
        chown.push_str(self.user.as_str());
        if let Some(group) = &self.group {
            chown.push_str(":");
            chown.push_str(group);
        }
        chown
    }
}

impl DockerfileGenerator for CopyResources {
    fn generate_dockerfile_lines(&self, context: GenerationContext) -> Result<Vec<DockerfileLine>> {
        match self {
            CopyResources::Copy(copy) => copy.generate_dockerfile_lines(context),
            CopyResources::Add(add_web_file) => add_web_file.generate_dockerfile_lines(context),
            CopyResources::AddGitRepo(add_git_repo) => {
                add_git_repo.generate_dockerfile_lines(context)
            }
        }
    }

    // fn generate_content(&self, context: &mut GenerationContext) -> Result<()> {
    //     match self {
    //         CopyResources::Copy(copy) => copy.generate_content(context),
    //         CopyResources::Add(add_web_file) => add_web_file.generate_content(context),
    //         CopyResources::AddGitRepo(add_git_repo) => add_git_repo.generate_content(context),
    //     }
    // }
}

impl DockerfileGenerator for Copy {
    fn generate_dockerfile_lines(
        &self,
        _context: GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        if let Some(from) = &self.from {
            options.push(InstructionOption::WithValue(
                "from".to_string(),
                from.to_string(),
            ));
        }
        if let Some(chown) = &self.chown {
            options.push(InstructionOption::WithValue(
                "chown".to_string(),
                chown.to_string(),
            ));
        }
        if let Some(chmod) = &self.chmod {
            options.push(InstructionOption::WithValue(
                "chmod".to_string(),
                chmod.to_string(),
            ));
        }
        // excludes are not supported yet: minimal version 1.7-labs
        // if let Some(exclude) = &self.exclude {
        //     for path in exclude.clone().to_vec() {
        //         options.push(InstructionOption::WithValue("exclude".to_string(), path));
        //     }
        // }
        if self.link.unwrap_or(true) {
            options.push(InstructionOption::NameOnly("link".to_string()));
        }
        // parents are not supported yet: minimal version 1.7-labs
        // if self.parents.unwrap_or(false) {
        //     options.push(InstructionOption::NameOnly("parents".to_string()));
        // }

        Ok(vec![DockerfileLine::Instruction(DockerfileInsctruction {
            command: "COPY".to_string(),
            content: format!(
                "{paths} {target}",
                paths = self.paths.join(" "),
                target = self.target.clone().unwrap_or("./".to_string())
            ),
            options,
        })])
    }
    // fn generate_content(&self, context: &mut GenerationContext) -> Result<()> {
    //     let paths = self.paths.clone().to_vec().join(" ");
    //     let target = self.target.clone().unwrap_or("./".to_string());
    //     let mut options = String::new();
    //     context.write_conditional_str_option("from", &self.from);
    //     context.write_optional_chown_option(&self.chown.clone().or(context.user.clone().map(
    //         |user| Chown {
    //             user,
    //             ..Default::default()
    //         },
    //     )));
    //     context.write_conditional_str_option("chmod", &self.chmod);
    //     if let Some(exclude) = &self.exclude {
    //         for path in exclude.clone().to_vec() {
    //             context.write_str_option("exclude", &path);
    //         }
    //     }
    //     context.write_bool_option("link", &self.link.unwrap_or(true));
    //     context.write_conditional_bool_option("parents", &self.parents);
    //     context
    //         .write_fmt(format_args!(
    //             "COPY{options} {paths} {target}",
    //             paths = paths,
    //             target = target
    //         ))
    //         .map_err(|err| Error::Format(err))?;
    //     Ok(())
    // }
}

impl DockerfileGenerator for Add {
    fn generate_dockerfile_lines(
        &self,
        _context: GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        if let Some(checksum) = &self.checksum {
            options.push(InstructionOption::WithValue(
                "checksum".to_string(),
                checksum.to_string(),
            ));
        }
        if let Some(chown) = &self.chown {
            options.push(InstructionOption::WithValue(
                "chown".to_string(),
                chown.to_string(),
            ));
        }
        if let Some(chmod) = &self.chmod {
            options.push(InstructionOption::WithValue(
                "chmod".to_string(),
                chmod.to_string(),
            ));
        }
        if self.link.unwrap_or(true) {
            options.push(InstructionOption::NameOnly("link".to_string()));
        }

        Ok(vec![DockerfileLine::Instruction(DockerfileInsctruction {
            command: "ADD".to_string(),
            content: format!(
                "{paths} {target}",
                paths = self.paths.join(" "),
                target = self.target.clone().unwrap_or("./".to_string())
            ),
            options,
        })])
    }

    // fn generate_content(&self, context: &mut GenerationContext) -> Result<()> {
    //     let urls = self.paths.clone().to_vec().join(" ");
    //     let mut options = String::new();
    //     context.write_optional_chown_option(&self.chown.clone().or(context.user.clone().map(
    //         |user| Chown {
    //             user,
    //             ..Default::default()
    //         },
    //     )));
    //     context.write_conditional_str_option("chmod", &self.chmod);
    //     context.write_bool_option("link", &self.link.unwrap_or(true));
    //     context
    //         .write_fmt(format_args!(
    //             "ADD{options} {urls} {target}",
    //             urls = urls,
    //             target = self.target.clone().unwrap_or(".".to_string())
    //         ))
    //         .map_err(|err| Error::Format(err))?;
    //     Ok(())
    // }
}

impl DockerfileGenerator for AddGitRepo {
    fn generate_dockerfile_lines(
        &self,
        _context: GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        if let Some(chown) = &self.chown {
            options.push(InstructionOption::WithValue(
                "chown".to_string(),
                chown.to_string(),
            ));
        }
        if let Some(chmod) = &self.chmod {
            options.push(InstructionOption::WithValue(
                "chmod".to_string(),
                chmod.to_string(),
            ));
        }
        // excludes are not supported yet: minimal version 1.7-labs
        // if let Some(exclude) = &self.exclude {
        //     for path in exclude.clone().to_vec() {
        //         options.push(InstructionOption::WithValue("exclude".to_string(), path));
        //     }
        // }
        if let Some(keep_git_dir) = &self.keep_git_dir {
            options.push(InstructionOption::WithValue(
                "keep-git-dir".to_string(),
                keep_git_dir.to_string(),
            ));
        }
        if self.link.unwrap_or(true) {
            options.push(InstructionOption::NameOnly("link".to_string()));
        }

        Ok(vec![DockerfileLine::Instruction(DockerfileInsctruction {
            command: "ADD".to_string(),
            content: format!(
                "{repo} {target}",
                repo = self.repo,
                target = self.target.clone().unwrap_or(".".to_string())
            ),
            options,
        })])
    }

    // fn generate_content(&self, context: &mut GenerationContext) -> Result<()> {
    //     let mut options = String::new();
    //     context.write_optional_chown_option(&self.chown.clone().or(context.user.clone().map(
    //         |user| Chown {
    //             user,
    //             ..Default::default()
    //         },
    //     )));
    //     context.write_conditional_str_option("chmod", &self.chmod);
    //     if let Some(exclude) = &self.exclude {
    //         for path in exclude.clone().to_vec() {
    //             context.write_str_option("exclude", &path);
    //         }
    //     }
    //     context.write_bool_option("link", &self.link.unwrap_or(true));

    //     context
    //         .write_fmt(format_args!(
    //             "ADD{options} {repo} {target}",
    //             repo = self.repo,
    //             target = self.target.clone().unwrap_or(".".to_string())
    //         ))
    //         .map_err(|err| Error::Format(err))?;
    //     Ok(())
    // }
}

impl DockerfileGenerator for dyn StageGenerator {
    fn generate_dockerfile_lines(&self, context: GenerationContext) -> Result<Vec<DockerfileLine>> {
        // TODO: get builder position or give context to the function
        let stage_name = self.name(&context);
        let mut lines = vec![
            DockerfileLine::Comment(stage_name.clone()),
            DockerfileLine::Instruction(DockerfileInsctruction {
                command: "FROM".to_string(),
                content: format!(
                    "{image_name} AS {stage_name}",
                    image_name = self.from().to_string()
                ),
                options: vec![],
            }),
        ];
        if let Some(run) = self.to_run_inscruction(&context)? {
            lines.push(DockerfileLine::Instruction(run));
        }
        Ok(lines)
    }
}

impl DockerfileGenerator for Image {
    fn generate_dockerfile_lines(&self, context: GenerationContext) -> Result<Vec<DockerfileLine>> {
        let mut lines = <dyn StageGenerator>::generate_dockerfile_lines(self, context)?;
        if let Some(expose) = &self.expose {
            expose.iter().for_each(|port| {
                lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "EXPOSE".to_string(),
                    content: port.to_string(),
                    options: vec![],
                }))
            });
        }
        if let Some(healthcheck) = &self.healthcheck {
            let mut options = vec![];
            if let Some(interval) = &healthcheck.interval {
                options.push(InstructionOption::WithValue(
                    "interval".to_string(),
                    interval.to_string(),
                ));
            }
            if let Some(timeout) = &healthcheck.timeout {
                options.push(InstructionOption::WithValue(
                    "timeout".to_string(),
                    timeout.to_string(),
                ));
            }
            if let Some(start_period) = &healthcheck.start {
                options.push(InstructionOption::WithValue(
                    "start-period".to_string(),
                    start_period.to_string(),
                ));
            }
            if let Some(retries) = &healthcheck.retries {
                options.push(InstructionOption::WithValue(
                    "retries".to_string(),
                    retries.to_string(),
                ));
            }
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "HEALTHCHECK".to_string(),
                content: format!("CMD {}\n", healthcheck.cmd),
                options,
            }))
        }
        if let Some(entrypoint) = &self.entrypoint {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "ENTRYPOINT".to_string(),
                content: string_vec_to_string(entrypoint),
                options: vec![],
            }))
        }
        if let Some(cmd) = &self.cmd {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "CMD".to_string(),
                content: string_vec_to_string(cmd),
                options: vec![],
            }))
        }
        Ok(lines)
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

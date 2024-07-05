use crate::{
    dockerfile_struct::{DockerfileInsctruction, DockerfileLine, InstructionOption},
    script_runner::ScriptRunner,
    Add, AddGitRepo, Artifact, BaseStage, Copy, CopyResources, Image, ImageName, ImageVersion,
    Port, PortProtocol, Result, Stage, User, DOCKERFILE_VERSION,
};

pub const LINE_SEPARATOR: &str = " \\\n    ";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenerationContext {
    pub user: Option<User>,
    pub previous_builders: Vec<String>,
}
pub trait DockerfileGenerator {
    fn generate_dockerfile_lines(&self, context: &GenerationContext)
        -> Result<Vec<DockerfileLine>>;
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

impl ToString for User {
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

impl ToString for Port {
    fn to_string(&self) -> String {
        match &self.protocol {
            Some(protocol) => format!(
                "{port}/{protocol}",
                port = self.port,
                protocol = protocol.to_string()
            ),
            None => self.port.to_string(),
        }
    }
}

impl ToString for PortProtocol {
    fn to_string(&self) -> String {
        match self {
            PortProtocol::Tcp => "tcp".to_string(),
            PortProtocol::Udp => "udp".to_string(),
        }
    }
}

impl DockerfileGenerator for CopyResources {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        match self {
            CopyResources::Copy(copy) => copy.generate_dockerfile_lines(context),
            CopyResources::Add(add_web_file) => add_web_file.generate_dockerfile_lines(context),
            CopyResources::AddGitRepo(add_git_repo) => {
                add_git_repo.generate_dockerfile_lines(context)
            }
        }
    }
}

impl DockerfileGenerator for Copy {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        if let Some(from) = &self.from {
            options.push(InstructionOption::WithValue(
                "from".to_string(),
                from.to_string(),
            ));
        }
        if let Some(chown) = self.chown.as_ref().or(context.user.as_ref()) {
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
            content: copy_paths_to_string(&self.paths, &self.target),
            options,
        })])
    }
}

impl DockerfileGenerator for Add {
    fn generate_dockerfile_lines(
        &self,
        _context: &GenerationContext,
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
            content: copy_paths_to_string(&self.paths, &self.target),
            options,
        })])
    }
}

impl DockerfileGenerator for AddGitRepo {
    fn generate_dockerfile_lines(
        &self,
        _context: &GenerationContext,
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
            content: copy_paths_to_string(&vec![self.repo.clone()], &self.target),
            options,
        })])
    }
}

impl Artifact {
    fn to_copy(&self) -> Copy {
        Copy {
            paths: vec![self.source.clone()],
            target: Some(self.target.clone()),
            from: Some(self.builder.clone()),
            ..Default::default()
        }
    }
}

impl DockerfileGenerator for Artifact {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        self.to_copy().generate_dockerfile_lines(context)
    }
}

impl DockerfileGenerator for dyn Stage {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let context = GenerationContext {
            user: self.user(),
            previous_builders: context.previous_builders.clone(),
        };
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
        if let Some(env) = self.env() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "ENV".to_string(),
                content: env
                    .into_iter()
                    .map(|(key, value)| format!("{}=\"{}\"", key, value))
                    .collect::<Vec<String>>()
                    .join(LINE_SEPARATOR),
                options: vec![],
            }));
        }
        if let Some(workdir) = self.workdir() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "WORKDIR".to_string(),
                content: workdir.clone(),
                options: vec![],
            }));
        }
        if let Some(copies) = self.copy() {
            for copy in copies {
                lines.append(&mut copy.generate_dockerfile_lines(&context)?);
            }
        }
        if let Some(artifacts) = self.artifacts() {
            for artifact in artifacts {
                lines.append(&mut artifact.generate_dockerfile_lines(&context)?);
            }
        }
        if let Some(root) = self.root() {
            let root_context = GenerationContext {
                user: Some(User::new("0")),
                previous_builders: context.previous_builders.clone(),
            };
            if let Some(instruction) = root.to_run_inscruction(&root_context)? {
                lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "USER".to_string(),
                    content: root_context.user.unwrap().to_string(),
                    options: vec![],
                }));
                lines.push(DockerfileLine::Instruction(instruction));
            }
        }
        if let Some(user) = self.user() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "USER".to_string(),
                content: user.to_string(),
                options: vec![],
            }));
        }
        if let Some(run) = self.to_run_inscruction(&context)? {
            lines.push(DockerfileLine::Instruction(run));
        }
        Ok(lines)
    }
}

impl DockerfileGenerator for Image {
    fn generate_dockerfile_lines(
        &self,
        _context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut context: GenerationContext = GenerationContext {
            user: self.user(),
            previous_builders: vec![],
        };
        let mut lines = vec![
            DockerfileLine::Comment(format!("syntax=docker/dockerfile:{}", DOCKERFILE_VERSION)),
            DockerfileLine::Empty,
        ];
        if let Some(builders) = self.builders.as_ref() {
            for builder in builders {
                lines.append(&mut <dyn Stage>::generate_dockerfile_lines(
                    builder, &context,
                )?);
                lines.push(DockerfileLine::Empty);
                context.previous_builders.push(builder.name(&context));
            }
        }
        lines.append(&mut <dyn Stage>::generate_dockerfile_lines(self, &context)?);
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
                content: format!("CMD {}", healthcheck.cmd),
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

fn copy_paths_to_string(paths: &Vec<String>, target: &Option<String>) -> String {
    let mut parts = paths.clone();
    parts.push(target.clone().unwrap_or("./".to_string()));
    parts
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<String>>()
        .join(" ")
}

fn string_vec_to_string(string_vec: &Vec<String>) -> String {
    format!(
        "[{}]",
        string_vec
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

#[cfg(test)]
mod test {
    use crate::*;

    mod builder {
        use super::*;

        #[test]
        fn name_with_name() {
            let builder = Builder {
                name: Some(String::from("my-builder")),
                ..Default::default()
            };
            let name = builder.name(&GenerationContext {
                previous_builders: vec!["builder-0".to_string()],
                ..Default::default()
            });
            assert_eq!(name, "my-builder");
        }

        #[test]
        fn name_without_name() {
            let builder = Builder::default();
            let name = builder.name(&GenerationContext {
                previous_builders: vec!["builder-0".to_string(), "bob".to_string()],
                ..Default::default()
            });
            assert_eq!(name, "builder-2");
        }

        #[test]
        fn user_with_user() {
            let builder = Builder {
                user: Some(User::new_without_group("my-user")),
                ..Default::default()
            };
            let user = builder.user();
            assert_eq!(
                user,
                Some(User {
                    user: "my-user".into(),
                    group: None
                })
            );
        }

        #[test]
        fn user_without_user() {
            let builder = Builder::default();
            let user = builder.user();
            assert_eq!(user, None);
        }
    }

    mod image_name {
        use super::*;

        #[test]
        fn test_image_name() {
            let image = Image {
                from: Some(ImageName {
                    path: String::from("my-image"),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let name = image.name(&GenerationContext {
                previous_builders: vec![
                    "builder-0".to_string(),
                    "builder-1".to_string(),
                    "builder-2".to_string(),
                ],
                ..Default::default()
            });
            assert_eq!(name, "runtime");
        }

        #[test]
        fn test_image_user_with_user() {
            let image = Image {
                user: Some(User::new_without_group("my-user")),
                from: Some(ImageName {
                    path: String::from("my-image"),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let user = image.user();
            assert_eq!(
                user,
                Some(User {
                    user: String::from("my-user"),
                    group: None,
                })
            );
        }

        #[test]
        fn test_image_user_without_user() {
            let image = Image {
                from: Some(ImageName {
                    path: String::from("my-image"),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let user = image.user();
            assert_eq!(
                user,
                Some(User {
                    user: String::from("1000"),
                    group: Some(String::from("1000")),
                })
            );
        }
    }
}

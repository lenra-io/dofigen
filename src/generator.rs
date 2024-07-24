use crate::{
    dockerfile_struct::{DockerfileInsctruction, DockerfileLine, InstructionOption},
    script_runner::ScriptRunner,
    Add, AddGitRepo, Artifact, BaseStage, Copy, CopyOptions, CopyResource, Image, ImageName,
    ImageVersion, Port, PortProtocol, Resource, Result, Stage, User, DOCKERFILE_VERSION,
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
            if let Some(port) = self.port.clone() {
                registry.push_str(":");
                registry.push_str(port.to_string().as_str());
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
            _ => {}
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
            Some(protocol) => {
                format!(
                    "{port}/{protocol}",
                    port = self.port,
                    protocol = protocol.to_string()
                )
            }
            _ => self.port.to_string(),
        }
    }
}

impl ToString for PortProtocol {
    fn to_string(&self) -> String {
        match self {
            PortProtocol::Tcp => "tcp".into(),
            PortProtocol::Udp => "udp".into(),
        }
    }
}

impl ToString for Resource {
    fn to_string(&self) -> String {
        match self {
            Resource::File(file) => file.to_string_lossy().to_string(),
            Resource::Url(url) => url.to_string(),
        }
    }
}

impl DockerfileGenerator for CopyResource {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        match self {
            CopyResource::Copy(copy) => copy.generate_dockerfile_lines(context),
            CopyResource::Add(add_web_file) => add_web_file.generate_dockerfile_lines(context),
            CopyResource::AddGitRepo(add_git_repo) => {
                add_git_repo.generate_dockerfile_lines(context)
            }
        }
    }
}

fn add_copy_options(
    inst_options: &mut Vec<InstructionOption>,
    copy_options: &CopyOptions,
    context: &GenerationContext,
) {
    if let Some(chown) = copy_options.chown.as_ref().or(context.user.as_ref().into()) {
        inst_options.push(InstructionOption::WithValue("chown".into(), chown.into()));
    }
    if let Some(chmod) = &copy_options.chmod {
        inst_options.push(InstructionOption::WithValue("chmod".into(), chmod.into()));
    }
    if *copy_options.link.as_ref().unwrap_or(&true) {
        inst_options.push(InstructionOption::NameOnly("link".into()));
    }
}

impl DockerfileGenerator for Copy {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        if let Some(from) = &self.from {
            options.push(InstructionOption::WithValue("from".into(), from.into()));
        }
        add_copy_options(&mut options, &self.options, context);
        // excludes are not supported yet: minimal version 1.7-labs
        // if let Patch::Present(exclude) = &self.exclude {
        //     for path in exclude.clone().to_vec() {
        //         options.push(InstructionOption::WithValue("exclude".into(), path));
        //     }
        // }
        // parents are not supported yet: minimal version 1.7-labs
        // if self.parents.unwrap_or(false) {
        //     options.push(InstructionOption::NameOnly("parents".into()));
        // }
        Ok(vec![DockerfileLine::Instruction(DockerfileInsctruction {
            command: "COPY".into(),
            content: copy_paths_into(self.paths.to_vec(), &self.options.target),
            options,
        })])
    }
}

impl DockerfileGenerator for Add {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        if let Some(checksum) = &self.checksum {
            options.push(InstructionOption::WithValue(
                "checksum".into(),
                checksum.into(),
            ));
        }
        add_copy_options(&mut options, &self.options, context);

        Ok(vec![DockerfileLine::Instruction(DockerfileInsctruction {
            command: "ADD".into(),
            content: copy_paths_into(
                self.files
                    .iter()
                    .map(|file| file.to_string())
                    .collect::<Vec<String>>(),
                &self.options.target,
            ),
            options,
        })])
    }
}

impl DockerfileGenerator for AddGitRepo {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        add_copy_options(&mut options, &self.options, context);

        // excludes are not supported yet: minimal version 1.7-labs
        // if let Patch::Present(exclude) = &self.exclude {
        //     for path in exclude.clone().to_vec() {
        //         options.push(InstructionOption::WithValue("exclude".into(), path));
        //     }
        // }
        if let Some(keep_git_dir) = &self.keep_git_dir {
            options.push(InstructionOption::WithValue(
                "keep-git-dir".into(),
                keep_git_dir.to_string(),
            ));
        }

        Ok(vec![DockerfileLine::Instruction(DockerfileInsctruction {
            command: "ADD".into(),
            content: copy_paths_into(vec![self.repo.clone()], &self.options.target),
            options,
        })])
    }
}

impl Artifact {
    fn to_copy(&self) -> Copy {
        Copy {
            paths: vec![self.source.clone()].into(),
            options: CopyOptions {
                target: Some(self.target.clone()),
                ..Default::default()
            },
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
            user: self.user().into(),
            previous_builders: context.previous_builders.clone(),
        };
        let stage_name = self.name(&context);
        let mut lines = vec![
            DockerfileLine::Comment(stage_name.clone()),
            DockerfileLine::Instruction(DockerfileInsctruction {
                command: "FROM".into(),
                content: format!(
                    "{image_name} AS {stage_name}",
                    image_name = self.from().to_string()
                ),
                options: vec![],
            }),
        ];
        if let Some(env) = self.env() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "ENV".into(),
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
                command: "WORKDIR".into(),
                content: workdir.clone(),
                options: vec![],
            }));
        }
        for copy in self.copy().iter() {
            lines.append(&mut copy.generate_dockerfile_lines(&context)?);
        }
        for artifact in self.artifacts().iter() {
            lines.append(&mut artifact.generate_dockerfile_lines(&context)?);
        }
        if let Some(root) = self.root() {
            let root_context = GenerationContext {
                user: Some(User::new("0")),
                previous_builders: context.previous_builders.clone(),
            };
            if let Some(instruction) = root.to_run_inscruction(&root_context)? {
                lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "USER".into(),
                    content: root_context.user.unwrap().to_string(),
                    options: vec![],
                }));
                lines.push(DockerfileLine::Instruction(instruction));
            }
        }
        if let Some(user) = self.user() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "USER".into(),
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
            user: self.user().into(),
            previous_builders: vec![],
        };
        let mut lines = vec![
            DockerfileLine::Comment(format!("syntax=docker/dockerfile:{}", DOCKERFILE_VERSION)),
            DockerfileLine::Empty,
        ];
        for builder in self.builders.iter() {
            lines.append(&mut <dyn Stage>::generate_dockerfile_lines(
                builder, &context,
            )?);
            lines.push(DockerfileLine::Empty);
            context.previous_builders.push(builder.name(&context));
        }
        lines.append(&mut <dyn Stage>::generate_dockerfile_lines(self, &context)?);
        self.expose.iter().for_each(|port| {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "EXPOSE".into(),
                content: port.to_string(),
                options: vec![],
            }))
        });
        if let Some(healthcheck) = &self.healthcheck {
            let mut options = vec![];
            if let Some(interval) = &healthcheck.interval {
                options.push(InstructionOption::WithValue(
                    "interval".into(),
                    interval.into(),
                ));
            }
            if let Some(timeout) = &healthcheck.timeout {
                options.push(InstructionOption::WithValue(
                    "timeout".into(),
                    timeout.into(),
                ));
            }
            if let Some(start_period) = &healthcheck.start {
                options.push(InstructionOption::WithValue(
                    "start-period".into(),
                    start_period.into(),
                ));
            }
            if let Some(retries) = &healthcheck.retries {
                options.push(InstructionOption::WithValue(
                    "retries".into(),
                    retries.to_string(),
                ));
            }
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "HEALTHCHECK".into(),
                content: format!("CMD {}", healthcheck.cmd.clone()),
                options,
            }))
        }
        if !self.entrypoint.is_empty() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "ENTRYPOINT".into(),
                content: string_vec_into(self.entrypoint.to_vec()),
                options: vec![],
            }))
        }
        if !self.cmd.is_empty() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "CMD".into(),
                content: string_vec_into(self.cmd.to_vec()),
                options: vec![],
            }))
        }
        Ok(lines)
    }
}

fn copy_paths_into(paths: Vec<String>, target: &Option<String>) -> String {
    let mut parts = paths.clone();
    parts.push(target.clone().unwrap_or("./".into()));
    parts
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<String>>()
        .join(" ")
}

fn string_vec_into(string_vec: Vec<String>) -> String {
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
                previous_builders: vec!["builder-0".into()],
                ..Default::default()
            });
            assert_eq!(name, "my-builder");
        }

        #[test]
        fn name_without_name() {
            let builder = Builder::default();
            let name = builder.name(&GenerationContext {
                previous_builders: vec!["builder-0".into(), "bob".into()],
                ..Default::default()
            });
            assert_eq!(name, "builder-2");
        }

        #[test]
        fn user_with_user() {
            let builder = Builder {
                user: Some(PermissiveStruct::new(User::new_without_group("my-user"))),
                ..Default::default()
            };
            let user = builder.user();
            assert_eq!(
                user,
                Some(User {
                    user: "my-user".into(),
                    group: None,
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
        use PermissiveStruct;

        use super::*;

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
                previous_builders: vec!["builder-0".into(), "builder-1".into(), "builder-2".into()],
                ..Default::default()
            });
            assert_eq!(name, "runtime");
        }

        #[test]
        fn test_image_user_with_user() {
            let image = Image {
                user: Some(PermissiveStruct::new(User::new_without_group("my-user"))),
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
                    user: String::from("my-user"),
                    group: None,
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
                    user: String::from("1000"),
                    group: Some(String::from("1000")),
                })
            );
        }
    }
}

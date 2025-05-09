use std::collections::HashMap;

use crate::errors::Error;

use crate::lock::DEFAULT_PORT;
use crate::{
    DOCKERFILE_VERSION, FILE_HEADER_COMMENTS, LintMessage, LintSession, Result,
    dockerfile_struct::*, dofigen_struct::*,
};

pub const LINE_SEPARATOR: &str = " \\\n    ";
pub const DEFAULT_FROM: &str = "scratch";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenerationContext {
    dofigen: Dofigen,
    pub(crate) user: Option<User>,
    pub(crate) stage_name: String,
    pub(crate) default_from: FromContext,
    state_stack: Vec<GenerationContextState>,
    pub(crate) lint_session: LintSession,
}

impl GenerationContext {
    pub fn get_lint_messages(&self) -> Vec<LintMessage> {
        self.lint_session.messages()
    }

    fn push_state(&mut self, state: GenerationContextState) {
        let mut prev_state = GenerationContextState::default();
        if let Some(user) = &state.user {
            prev_state.user = Some(self.user.clone());
            self.user = user.clone();
        }
        if let Some(stage_name) = &state.stage_name {
            prev_state.stage_name = Some(self.stage_name.clone());
            self.stage_name = stage_name.clone();
        }
        if let Some(default_from) = &state.default_from {
            prev_state.default_from = Some(self.default_from.clone());
            self.default_from = default_from.clone();
        }
        self.state_stack.push(prev_state);
    }

    fn pop_state(&mut self) {
        let prev_state = self.state_stack.pop().expect("The state stack is empty");
        if let Some(user) = prev_state.user {
            self.user = user;
        }
        if let Some(stage_name) = prev_state.stage_name {
            self.stage_name = stage_name;
        }
        if let Some(default_from) = prev_state.default_from {
            self.default_from = default_from;
        }
    }

    pub fn from(dofigen: Dofigen) -> Self {
        let lint_session = LintSession::analyze(&dofigen);
        Self {
            dofigen,
            lint_session,
            ..Default::default()
        }
    }

    pub fn generate_dockerfile_struct(&mut self) -> Result<DockerFile> {
        Ok(DockerFile {
            lines: self.dofigen.clone().generate_dockerfile_lines(self)?,
        })
    }

    pub fn generate_dockerfile(&mut self) -> Result<String> {
        Ok(format!(
            "{}\n",
            self.generate_dockerfile_struct()?.to_string()
        ))
    }

    pub fn generate_dockerignore_struct(&self) -> Result<DockerIgnore> {
        let mut lines = vec![];

        for line in FILE_HEADER_COMMENTS {
            lines.push(DockerIgnoreLine::Comment(line.to_string()));
        }
        lines.push(DockerIgnoreLine::Empty);

        if !self.dofigen.context.is_empty() {
            lines.push(DockerIgnoreLine::Pattern("**".into()));
            self.dofigen.context.iter().for_each(|path| {
                lines.push(DockerIgnoreLine::NegatePattern(path.clone()));
            });
        }
        self.dofigen.ignore.iter().for_each(|path| {
            lines.push(DockerIgnoreLine::Pattern(path.clone()));
        });
        Ok(DockerIgnore { lines })
    }

    pub fn generate_dockerignore(&self) -> Result<String> {
        Ok(format!(
            "{}\n",
            self.generate_dockerignore_struct()?.to_string()
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenerationContextState {
    user: Option<Option<User>>,
    stage_name: Option<String>,
    default_from: Option<FromContext>,
}

pub trait DockerfileGenerator {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>>;
}

impl Dofigen {
    pub fn get_base_image(&self) -> Option<ImageName> {
        let mut stage = &self.stage;
        while let FromContext::FromBuilder(builder_name) = &stage.from {
            if let Some(builder) = self.builders.get(builder_name) {
                stage = builder;
            } else {
                return None;
            }
        }
        match &stage.from {
            FromContext::FromImage(image) => Some(image.clone()),
            // For basic context we can't know if it's an image, a builder not strongly typed or a build context
            FromContext::FromContext(_) => None,
            FromContext::FromBuilder(_) => unreachable!(),
        }
    }
}

impl Stage {
    pub fn from(&self, context: &GenerationContext) -> FromContext {
        match &self.from {
            FromContext::FromImage(image) => FromContext::FromImage(image.clone()),
            FromContext::FromBuilder(builder) => FromContext::FromBuilder(builder.clone()),
            FromContext::FromContext(Some(context)) => {
                FromContext::FromContext(Some(context.clone()))
            }
            _ => match &context.default_from {
                FromContext::FromImage(image) => FromContext::FromImage(image.clone()),
                FromContext::FromBuilder(builder) => FromContext::FromBuilder(builder.clone()),
                FromContext::FromContext(context) => {
                    FromContext::FromContext(context.clone().or(Some(DEFAULT_FROM.to_string())))
                }
            },
        }
    }

    pub fn user(&self, context: &GenerationContext) -> Option<User> {
        self.user.clone().or(context.user.clone())
    }
}

impl Run {
    pub fn is_empty(&self) -> bool {
        self.run.is_empty()
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

impl ToString for ImageName {
    fn to_string(&self) -> String {
        let mut registry = String::new();
        if let Some(host) = &self.host {
            registry.push_str(host);
            if let Some(port) = self.port.clone() {
                if port != DEFAULT_PORT {
                    registry.push_str(":");
                    registry.push_str(port.to_string().as_str());
                }
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

impl ToString for CacheSharing {
    fn to_string(&self) -> String {
        match self {
            CacheSharing::Shared => "shared".into(),
            CacheSharing::Private => "private".into(),
            CacheSharing::Locked => "locked".into(),
        }
    }
}

impl ToString for Network {
    fn to_string(&self) -> String {
        match self {
            Network::Default => "default".into(),
            Network::None => "none".into(),
            Network::Host => "host".into(),
        }
    }
}

impl ToString for Security {
    fn to_string(&self) -> String {
        match self {
            Security::Sandbox => "sandbox".into(),
            Security::Insecure => "insecure".into(),
        }
    }
}

impl ToString for FromContext {
    fn to_string(&self) -> String {
        match self {
            FromContext::FromBuilder(name) => name.clone(),
            FromContext::FromImage(image) => image.to_string(),
            FromContext::FromContext(context) => context.clone().unwrap_or_default(),
        }
    }
}

impl DockerfileGenerator for Dofigen {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        context.push_state(GenerationContextState {
            default_from: Some(self.stage.from(context).clone()),
            ..Default::default()
        });
        let mut lines = vec![DockerFileLine::Comment(format!(
            "syntax=docker/dockerfile:{}",
            DOCKERFILE_VERSION
        ))];
        for line in FILE_HEADER_COMMENTS {
            lines.push(DockerFileLine::Comment(line.to_string()));
        }

        // Arg
        if !self.global_arg.is_empty() {
            lines.push(DockerFileLine::Empty);
            let args = generate_arg_command(&self.global_arg);
            lines.extend(args);
        }

        let builder_names = context.lint_session.get_sorted_builders();

        for name in builder_names {
            context.push_state(GenerationContextState {
                stage_name: Some(name.clone()),
                ..Default::default()
            });
            let builder = self
                .builders
                .get(&name)
                .expect(format!("The builder '{}' not found", name).as_str());

            lines.push(DockerFileLine::Empty);
            lines.append(&mut Stage::generate_dockerfile_lines(builder, context)?);
            context.pop_state();
        }

        context.push_state(GenerationContextState {
            user: Some(Some(User::new("1000"))),
            stage_name: Some("runtime".into()),
            default_from: Some(FromContext::default()),
        });
        lines.push(DockerFileLine::Empty);
        lines.append(&mut self.stage.generate_dockerfile_lines(context)?);
        context.pop_state();

        self.volume.iter().for_each(|volume| {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::VOLUME,
                content: volume.clone(),
                options: vec![],
            }))
        });

        self.expose.iter().for_each(|port| {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::EXPOSE,
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
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::HEALTHCHECK,
                content: format!("CMD {}", healthcheck.cmd.clone()),
                options,
            }))
        }
        if !self.entrypoint.is_empty() {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::ENTRYPOINT,
                content: string_vec_into(self.entrypoint.to_vec()),
                options: vec![],
            }))
        }
        if !self.cmd.is_empty() {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::CMD,
                content: string_vec_into(self.cmd.to_vec()),
                options: vec![],
            }))
        }
        Ok(lines)
    }
}

impl DockerfileGenerator for Stage {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        context.push_state(GenerationContextState {
            user: Some(self.user(context)),
            ..Default::default()
        });
        let stage_name = context.stage_name.clone();

        // From
        let mut lines = vec![
            DockerFileLine::Comment(stage_name.clone()),
            DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::FROM,
                content: format!(
                    "{image_name} AS {stage_name}",
                    image_name = self.from(context).to_string()
                ),
                options: match &self.from {
                    FromContext::FromImage(ImageName {
                        platform: Some(platform),
                        ..
                    }) => {
                        vec![InstructionOption::WithValue(
                            "platform".into(),
                            platform.clone(),
                        )]
                    }
                    _ => vec![],
                },
            }),
        ];

        // Arg
        if !self.arg.is_empty() {
            let args = generate_arg_command(&self.arg);
            lines.extend(args);
        }

        // Label
        if !self.label.is_empty() {
            let mut keys = self.label.keys().collect::<Vec<&String>>();
            keys.sort();
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::LABEL,
                content: keys
                    .iter()
                    .map(|&key| {
                        format!(
                            "{}=\"{}\"",
                            key,
                            self.label.get(key).unwrap().replace("\n", "\\\n")
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(LINE_SEPARATOR),
                options: vec![],
            }));
        }

        // Env
        if !self.env.is_empty() {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::ENV,
                content: self
                    .env
                    .iter()
                    .map(|(key, value)| format!("{}=\"{}\"", key, value))
                    .collect::<Vec<String>>()
                    .join(LINE_SEPARATOR),
                options: vec![],
            }));
        }

        // Workdir
        if let Some(workdir) = &self.workdir {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::WORKDIR,
                content: workdir.clone(),
                options: vec![],
            }));
        }

        // Copy resources
        for copy in self.copy.iter() {
            lines.append(&mut copy.generate_dockerfile_lines(context)?);
        }

        // Root
        if let Some(root) = &self.root {
            if !root.is_empty() {
                let root_user = User::new("0");
                // User
                lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::USER,
                    content: root_user.to_string(),
                    options: vec![],
                }));

                context.push_state(GenerationContextState {
                    user: Some(Some(root_user)),
                    ..Default::default()
                });
                // Run
                lines.append(&mut root.generate_dockerfile_lines(context)?);
                context.pop_state();
            }
        }

        // User
        if let Some(user) = self.user(context) {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::USER,
                content: user.to_string(),
                options: vec![],
            }));
        }

        // Run
        lines.append(&mut self.run.generate_dockerfile_lines(context)?);

        context.pop_state();

        Ok(lines)
    }
}

impl DockerfileGenerator for Run {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        let script = &self.run;
        if script.is_empty() {
            return Ok(vec![]);
        }
        let script_lines = script
            .iter()
            .flat_map(|command| command.lines())
            .collect::<Vec<&str>>();
        let content = match script_lines.len() {
            0 => {
                return Ok(vec![]);
            }
            1 => script_lines[0].into(),
            _ => format!("<<EOF\n{}\nEOF", script_lines.join("\n")),
        };
        let mut options = vec![];

        // Mount binds
        self.bind.iter().for_each(|bind| {
            let mut bind_options = vec![
                InstructionOptionOption::new("type", "bind".into()),
                InstructionOptionOption::new("target", bind.target.clone()),
            ];
            let from = match &bind.from {
                FromContext::FromImage(image) => Some(image.to_string()),
                FromContext::FromBuilder(builder) => Some(builder.clone()),
                FromContext::FromContext(context) => context.clone(),
            };
            if let Some(from) = from {
                bind_options.push(InstructionOptionOption::new("from", from));
            }
            if let Some(source) = bind.source.as_ref() {
                bind_options.push(InstructionOptionOption::new("source", source.clone()));
            }
            if bind.readwrite.unwrap_or(false) {
                bind_options.push(InstructionOptionOption::new_flag("readwrite"));
            }
            options.push(InstructionOption::WithOptions("mount".into(), bind_options));
        });

        // Mount caches
        for cache in self.cache.iter() {
            let target = cache.target.clone();

            let mut cache_options = vec![
                InstructionOptionOption::new("type", "cache".into()),
                InstructionOptionOption::new("target", target),
            ];
            if let Some(id) = cache.id.as_ref() {
                cache_options.push(InstructionOptionOption::new("id", id.clone()));
            }
            let from = match &cache.from {
                FromContext::FromImage(image) => Some(image.to_string()),
                FromContext::FromBuilder(builder) => Some(builder.clone()),
                FromContext::FromContext(context) => context.clone(),
            };
            if let Some(from) = from {
                cache_options.push(InstructionOptionOption::new("from", from));
                if let Some(source) = cache.source.as_ref() {
                    cache_options.push(InstructionOptionOption::new("source", source.clone()));
                }
            }
            if let Some(user) = cache.chown.as_ref().or(context.user.as_ref()) {
                if let Some(uid) = user.uid() {
                    cache_options.push(InstructionOptionOption::new("uid", uid.to_string()));
                }
                if let Some(gid) = user.gid() {
                    cache_options.push(InstructionOptionOption::new("gid", gid.to_string()));
                }
            }
            if let Some(chmod) = cache.chmod.as_ref() {
                cache_options.push(InstructionOptionOption::new("chmod", chmod.clone()));
            }
            cache_options.push(InstructionOptionOption::new(
                "sharing",
                cache.sharing.clone().unwrap_or_default().to_string(),
            ));
            if cache.readonly.unwrap_or(false) {
                cache_options.push(InstructionOptionOption::new_flag("readonly"));
            }

            options.push(InstructionOption::WithOptions(
                "mount".into(),
                cache_options,
            ));
        }

        // Mount tmpfs
        self.tmpfs.iter().for_each(|mount| {
            let mut mount_options = vec![
                InstructionOptionOption::new("type", "tmpfs".into()),
                InstructionOptionOption::new("target", mount.target.clone()),
            ];
            if let Some(size) = mount.size.as_ref() {
                mount_options.push(InstructionOptionOption::new("size", size.clone()));
            }
            options.push(InstructionOption::WithOptions(
                "mount".into(),
                mount_options,
            ));
        });

        // Mount secrets
        self.secret.iter().for_each(|mount| {
            let mut mount_options = vec![InstructionOptionOption::new("type", "secret".into())];
            if let Some(id) = mount.id.as_ref() {
                mount_options.push(InstructionOptionOption::new("id", id.clone()));
            }
            if let Some(target) = mount.target.as_ref() {
                mount_options.push(InstructionOptionOption::new("target", target.clone()));
            }
            if let Some(env) = mount.env.as_ref() {
                mount_options.push(InstructionOptionOption::new("env", env.clone()));
            }
            if let Some(required) = mount.required.as_ref() {
                mount_options.push(InstructionOptionOption::new(
                    "required",
                    required.to_string(),
                ));
            }
            if let Some(mode) = mount.mode.as_ref() {
                mount_options.push(InstructionOptionOption::new("mode", mode.to_string()));
            }
            if let Some(uid) = mount.uid.as_ref() {
                mount_options.push(InstructionOptionOption::new("uid", uid.to_string()));
            }
            if let Some(gid) = mount.gid.as_ref() {
                mount_options.push(InstructionOptionOption::new("gid", gid.to_string()));
            }

            options.push(InstructionOption::WithOptions(
                "mount".into(),
                mount_options,
            ));
        });

        // Mount ssh
        self.ssh.iter().for_each(|mount| {
            let mut mount_options = vec![InstructionOptionOption::new("type", "ssh".into())];
            if let Some(id) = mount.id.as_ref() {
                mount_options.push(InstructionOptionOption::new("id", id.clone()));
            }
            if let Some(target) = mount.target.as_ref() {
                mount_options.push(InstructionOptionOption::new("target", target.clone()));
            }
            if let Some(required) = mount.required.as_ref() {
                mount_options.push(InstructionOptionOption::new(
                    "required",
                    required.to_string(),
                ));
            }
            if let Some(mode) = mount.mode.as_ref() {
                mount_options.push(InstructionOptionOption::new("mode", mode.to_string()));
            }
            if let Some(uid) = mount.uid.as_ref() {
                mount_options.push(InstructionOptionOption::new("uid", uid.to_string()));
            }
            if let Some(gid) = mount.gid.as_ref() {
                mount_options.push(InstructionOptionOption::new("gid", gid.to_string()));
            }
            options.push(InstructionOption::WithOptions(
                "mount".into(),
                mount_options,
            ));
        });

        let mut lines = vec![];

        // Shell
        if !self.shell.is_empty() {
            lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                command: DockerFileCommand::SHELL,
                content: string_vec_into(self.shell.to_vec()),
                options: vec![],
            }));
        }

        if let Some(network) = &self.network {
            options.push(InstructionOption::WithValue(
                "network".into(),
                network.to_string(),
            ));
        }

        if let Some(security) = &self.security {
            options.push(InstructionOption::WithValue(
                "security".into(),
                security.to_string(),
            ));
        }

        lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
            command: DockerFileCommand::RUN,
            content,
            options,
        }));

        Ok(lines)
    }
}

impl DockerfileGenerator for CopyResource {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        match self {
            CopyResource::Copy(copy) => copy.generate_dockerfile_lines(context),
            CopyResource::Content(content) => content.generate_dockerfile_lines(context),
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
        inst_options.push(InstructionOption::Flag("link".into()));
    }
}

impl DockerfileGenerator for Copy {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        let mut options: Vec<InstructionOption> = vec![];

        let from = match &self.from {
            FromContext::FromImage(image) => Some(image.to_string()),
            FromContext::FromBuilder(builder) => Some(builder.clone()),
            FromContext::FromContext(context) => context.clone(),
        };
        if let Some(from) = from {
            options.push(InstructionOption::WithValue("from".into(), from));
        }
        add_copy_options(&mut options, &self.options, context);

        for path in self.exclude.iter() {
            options.push(InstructionOption::WithValue("exclude".into(), path.clone()));
        }

        if self.parents.unwrap_or(false) {
            options.push(InstructionOption::Flag("parents".into()));
        }

        Ok(vec![DockerFileLine::Instruction(DockerFileInsctruction {
            command: DockerFileCommand::COPY,
            content: copy_paths_into(self.paths.to_vec(), &self.options.target),
            options,
        })])
    }
}

impl DockerfileGenerator for CopyContent {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        let mut options: Vec<InstructionOption> = vec![];

        add_copy_options(&mut options, &self.options, context);

        let mut start_delimiter = "EOF".to_string();
        if !self.substitute.clone().unwrap_or(true) {
            start_delimiter = format!("\"{start_delimiter}\"");
        }
        let target = self.options.target.clone().ok_or(Error::Custom(
            "The target file must be defined when coying content".into(),
        ))?;
        let content = format!(
            "<<{start_delimiter} {target}\n{}\nEOF",
            self.content.clone()
        );

        Ok(vec![DockerFileLine::Instruction(DockerFileInsctruction {
            command: DockerFileCommand::COPY,
            content,
            options,
        })])
    }
}

impl DockerfileGenerator for Add {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        if let Some(checksum) = &self.checksum {
            options.push(InstructionOption::WithValue(
                "checksum".into(),
                checksum.into(),
            ));
        }
        if let Some(unpack) = &self.unpack {
            options.push(InstructionOption::WithValue(
                "unpack".into(),
                unpack.to_string(),
            ));
        }
        add_copy_options(&mut options, &self.options, context);

        Ok(vec![DockerFileLine::Instruction(DockerFileInsctruction {
            command: DockerFileCommand::ADD,
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
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerFileLine>> {
        let mut options: Vec<InstructionOption> = vec![];
        add_copy_options(&mut options, &self.options, context);

        for path in self.exclude.iter() {
            options.push(InstructionOption::WithValue("exclude".into(), path.clone()));
        }
        if let Some(keep_git_dir) = &self.keep_git_dir {
            options.push(InstructionOption::WithValue(
                "keep-git-dir".into(),
                keep_git_dir.to_string(),
            ));
        }
        if let Some(checksum) = &self.checksum {
            options.push(InstructionOption::WithValue(
                "checksum".into(),
                checksum.into(),
            ));
        }

        Ok(vec![DockerFileLine::Instruction(DockerFileInsctruction {
            command: DockerFileCommand::ADD,
            content: copy_paths_into(vec![self.repo.clone()], &self.options.target),
            options,
        })])
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

fn generate_arg_command(arg: &HashMap<String, String>) -> Vec<DockerFileLine> {
    let mut lines = vec![];
    let mut keys = arg.keys().collect::<Vec<&String>>();
    keys.sort();
    keys.iter().for_each(|key| {
        let value = arg.get(*key).unwrap();
        lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
            command: DockerFileCommand::ARG,
            content: if value.is_empty() {
                key.to_string()
            } else {
                format!("{}={}", key, value)
            },
            options: vec![],
        }));
    });
    lines
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod stage {
        use std::collections::HashMap;

        use super::*;

        #[test]
        fn user_with_user() {
            let stage = Stage {
                user: Some(User::new_without_group("my-user").into()),
                ..Default::default()
            };
            let user = stage.user(&GenerationContext::default());
            assert_eq_sorted!(
                user,
                Some(User {
                    user: "my-user".into(),
                    group: None,
                })
            );
        }

        #[test]
        fn user_without_user() {
            let stage = Stage::default();
            let user = stage.user(&GenerationContext::default());
            assert_eq_sorted!(user, None);
        }

        #[test]
        fn stage_args() {
            let stage = Stage {
                arg: HashMap::from([("arg2".into(), "".into()), ("arg1".into(), "value1".into())]),
                ..Default::default()
            };

            let lines = stage.generate_dockerfile_lines(&mut GenerationContext {
                stage_name: "test".into(),
                ..Default::default()
            });

            assert_eq_sorted!(
                lines.unwrap(),
                vec![
                    DockerFileLine::Comment("test".into()),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "scratch AS test".into(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ARG,
                        content: "arg1=value1".into(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ARG,
                        content: "arg2".into(),
                        options: vec![],
                    }),
                ]
            );
        }
    }

    mod copy {
        use super::*;

        #[test]
        fn with_chmod() {
            let copy = Copy {
                paths: vec!["/path/to/file".into()],
                options: CopyOptions {
                    target: Some("/app/".into()),
                    chmod: Some("755".into()),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lines = copy
                .generate_dockerfile_lines(&mut GenerationContext::default())
                .unwrap();

            assert_eq_sorted!(
                lines,
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::COPY,
                    content: "\"/path/to/file\" \"/app/\"".into(),
                    options: vec![
                        InstructionOption::WithValue("chmod".into(), "755".into()),
                        InstructionOption::Flag("link".into())
                    ],
                })]
            );
        }

        #[test]
        fn from_content() {
            let copy = CopyContent {
                content: "echo hello".into(),
                options: CopyOptions {
                    target: Some("test.sh".into()),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lines = copy
                .generate_dockerfile_lines(&mut GenerationContext::default())
                .unwrap();

            assert_eq_sorted!(
                lines,
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::COPY,
                    content: "<<EOF test.sh\necho hello\nEOF".into(),
                    options: vec![InstructionOption::Flag("link".into())],
                })]
            );
        }
    }

    mod image_name {
        use super::*;

        #[test]
        fn user_with_user() {
            let dofigen = Dofigen {
                stage: Stage {
                    user: Some(User::new_without_group("my-user").into()),
                    from: FromContext::FromImage(ImageName {
                        path: String::from("my-image"),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let user = dofigen.stage.user(&GenerationContext {
                user: Some(User::new("1000")),
                ..Default::default()
            });
            assert_eq_sorted!(
                user,
                Some(User {
                    user: String::from("my-user"),
                    group: None,
                })
            );
        }

        #[test]
        fn user_without_user() {
            let dofigen = Dofigen {
                stage: Stage {
                    from: FromContext::FromImage(ImageName {
                        path: String::from("my-image"),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let user = dofigen.stage.user(&GenerationContext {
                user: Some(User::new("1000")),
                ..Default::default()
            });
            assert_eq_sorted!(
                user,
                Some(User {
                    user: String::from("1000"),
                    group: Some(String::from("1000")),
                })
            );
        }

        #[test]
        fn with_platform() {
            let stage = Stage {
                from: FromContext::FromImage(ImageName {
                    path: String::from("alpine"),
                    platform: Some("linux/amd64".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            assert_eq_sorted!(
                stage
                    .generate_dockerfile_lines(&mut GenerationContext {
                        stage_name: "runtime".into(),
                        ..Default::default()
                    })
                    .unwrap(),
                vec![
                    DockerFileLine::Comment("runtime".into()),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "alpine AS runtime".into(),
                        options: vec![InstructionOption::WithValue(
                            "platform".into(),
                            "linux/amd64".into()
                        )],
                    })
                ]
            );
        }
    }

    mod run {
        use super::*;

        #[test]
        fn simple() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder
                    .generate_dockerfile_lines(&mut GenerationContext::default())
                    .unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![],
                })]
            );
        }

        #[test]
        fn without_run() {
            let builder = Run {
                ..Default::default()
            };
            assert_eq_sorted!(
                builder
                    .generate_dockerfile_lines(&mut GenerationContext::default())
                    .unwrap(),
                vec![]
            );
        }

        #[test]
        fn with_empty_run() {
            let builder = Run {
                run: vec![].into(),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder
                    .generate_dockerfile_lines(&mut GenerationContext::default())
                    .unwrap(),
                vec![]
            );
        }

        #[test]
        fn with_script_and_caches_with_named_user() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                cache: vec![Cache {
                    target: "/path/to/cache".into(),
                    readonly: Some(true),
                    ..Default::default()
                }]
                .into(),
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new("test")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "cache".into()),
                            InstructionOptionOption::new("target", "/path/to/cache".into()),
                            InstructionOptionOption::new("sharing", "locked".into()),
                            InstructionOptionOption::new_flag("readonly"),
                        ],
                    )],
                })]
            );
        }

        #[test]
        fn with_script_and_caches_with_uid_user() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                cache: vec![Cache {
                    target: "/path/to/cache".into(),
                    ..Default::default()
                }],
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "cache".into()),
                            InstructionOptionOption::new("target", "/path/to/cache".into()),
                            InstructionOptionOption::new("uid", "1000".into()),
                            InstructionOptionOption::new("gid", "1000".into()),
                            InstructionOptionOption::new("sharing", "locked".into()),
                        ],
                    )],
                })]
            );
        }

        #[test]
        fn with_script_and_caches_with_uid_user_without_group() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                cache: vec![Cache {
                    target: "/path/to/cache".into(),
                    ..Default::default()
                }],
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new_without_group("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "cache".into()),
                            InstructionOptionOption::new("target", "/path/to/cache".into()),
                            InstructionOptionOption::new("uid", "1000".into()),
                            InstructionOptionOption::new("sharing", "locked".into()),
                        ],
                    )],
                })]
            );
        }

        #[test]
        fn with_tmpfs() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                tmpfs: vec![TmpFs {
                    target: "/path/to/tmpfs".into(),
                    ..Default::default()
                }],
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new_without_group("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "tmpfs".into()),
                            InstructionOptionOption::new("target", "/path/to/tmpfs".into()),
                        ],
                    )],
                })]
            );
        }

        #[test]
        fn with_secret() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                secret: vec![Secret {
                    id: Some("test".into()),
                    ..Default::default()
                }],
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new_without_group("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "secret".into()),
                            InstructionOptionOption::new("id", "test".into()),
                        ],
                    )],
                })]
            );
        }

        #[test]
        fn with_secret_empty() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                secret: vec![Secret::default()],
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new_without_group("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![InstructionOptionOption::new("type", "secret".into()),],
                    )],
                })]
            );
        }

        #[test]
        fn with_ssh() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                ssh: vec![Ssh {
                    id: Some("test".into()),
                    ..Default::default()
                }],
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new_without_group("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "ssh".into()),
                            InstructionOptionOption::new("id", "test".into()),
                        ],
                    )],
                })]
            );
        }

        #[test]
        fn with_ssh_empty() {
            let builder = Run {
                run: vec!["echo Hello".into()].into(),
                ssh: vec![Ssh::default()],
                ..Default::default()
            };
            let mut context = GenerationContext {
                user: Some(User::new_without_group("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&mut context).unwrap(),
                vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![InstructionOptionOption::new("type", "ssh".into()),],
                    )],
                })]
            );
        }
    }

    mod label {
        use std::collections::HashMap;

        use crate::{DofigenContext, lock::Lock};

        use super::*;

        #[test]
        fn with_label() {
            let stage = Stage {
                label: HashMap::from([("key".into(), "value".into())]),
                ..Default::default()
            };
            let lines = stage
                .generate_dockerfile_lines(&mut GenerationContext::default())
                .unwrap();
            assert_eq_sorted!(
                lines[2],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::LABEL,
                    content: "key=\"value\"".into(),
                    options: vec![],
                })
            );
        }

        #[test]
        fn with_many_multiline_labels() {
            let stage = Stage {
                label: HashMap::from([
                    ("key1".into(), "value1".into()),
                    ("key2".into(), "value2\nligne2".into()),
                ]),
                ..Default::default()
            };
            let lines = stage
                .generate_dockerfile_lines(&mut GenerationContext::default())
                .unwrap();
            assert_eq_sorted!(
                lines[2],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::LABEL,
                    content: "key1=\"value1\" \\\n    key2=\"value2\\\nligne2\"".into(),
                    options: vec![],
                })
            );
        }

        #[test]
        fn locked_with_many_multiline_labels() {
            let dofigen = Dofigen {
                stage: Stage {
                    label: HashMap::from([
                        ("key1".into(), "value1".into()),
                        ("key2".into(), "value2\nligne2".into()),
                    ]),
                    ..Default::default()
                },
                ..Default::default()
            };
            let dofigen = dofigen.lock(&mut DofigenContext::new()).unwrap();
            let lines = dofigen
                .generate_dockerfile_lines(&mut GenerationContext::default())
                .unwrap();
            assert_eq_sorted!(
                lines[6],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::LABEL,
                    content: "io.dofigen.version=\"0.0.0\" \\\n    key1=\"value1\" \\\n    key2=\"value2\\\nligne2\"".into(),
                    options: vec![],
                })
            );
        }
    }
}

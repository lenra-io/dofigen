use std::collections::{HashMap, HashSet};

use crate::{dockerfile_struct::*, dofigen_struct::*, Result, DOCKERFILE_VERSION};

pub const LINE_SEPARATOR: &str = " \\\n    ";
pub const DEFAULT_FROM: &str = "scratch";

#[derive(Debug, Clone, PartialEq)]
pub struct GenerationContext {
    pub(crate) user: Option<User>,
    pub(crate) stage_name: String,
    pub(crate) default_from: FromContext,
    state_stack: Vec<GenerationContextState>,
    pub(crate) lint_session: LintSession,
}

impl GenerationContext {
    pub fn get_lint_messages(&self) -> Vec<LintMessage> {
        self.lint_session.messages.clone()
    }

    pub fn push_state(&mut self, state: GenerationContextState) {
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

    pub fn pop_state(&mut self) {
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

    pub fn from(dofigen: &Dofigen) -> Self {
        Self {
            user: None,
            stage_name: String::default(),
            default_from: FromContext::default(),
            lint_session: LintSession::analyze(dofigen),
            state_stack: vec![],
        }
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
    ) -> Result<Vec<DockerfileLine>>;
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

impl ToString for CacheSharing {
    fn to_string(&self) -> String {
        match self {
            CacheSharing::Shared => "shared".into(),
            CacheSharing::Private => "private".into(),
            CacheSharing::Locked => "locked".into(),
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

impl DockerfileGenerator for CopyResource {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
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
        inst_options.push(InstructionOption::Flag("link".into()));
    }
}

impl DockerfileGenerator for Copy {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
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
        context: &mut GenerationContext,
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
        context: &mut GenerationContext,
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

impl DockerfileGenerator for Dofigen {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        context.push_state(GenerationContextState {
            default_from: Some(self.stage.from(context).clone()),
            ..Default::default()
        });
        let mut lines = vec![
            DockerfileLine::Comment(format!("syntax=docker/dockerfile:{}", DOCKERFILE_VERSION)),
            DockerfileLine::Empty,
        ];

        for name in context.lint_session.get_sorted_builders() {
            println!("Generating stage: {}", name);
            context.push_state(GenerationContextState {
                stage_name: Some(name.clone()),
                ..Default::default()
            });
            let builder = self
                .builders
                .get(&name)
                .expect(format!("The builder '{}' not found", name).as_str());

            lines.append(&mut Stage::generate_dockerfile_lines(builder, context)?);
            lines.push(DockerfileLine::Empty);
            context.pop_state();
        }

        context.push_state(GenerationContextState {
            user: Some(Some(User::new("1000"))),
            stage_name: Some("runtime".into()),
            default_from: Some(FromContext::default()),
        });
        lines.append(&mut self.stage.generate_dockerfile_lines(context)?);
        context.pop_state();

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

impl DockerfileGenerator for Stage {
    fn generate_dockerfile_lines(
        &self,
        context: &mut GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        context.push_state(GenerationContextState {
            user: Some(self.user(context)),
            ..Default::default()
        });
        let stage_name = context.stage_name.clone();

        // From
        let mut lines = vec![
            DockerfileLine::Comment(stage_name.clone()),
            DockerfileLine::Instruction(DockerfileInsctruction {
                command: "FROM".into(),
                content: format!(
                    "{image_name} AS {stage_name}",
                    image_name = self.from(context).to_string()
                ),
                options: vec![],
            }),
        ];

        // Arg
        if !self.arg.is_empty() {
            let mut keys = self.arg.keys().collect::<Vec<&String>>();
            keys.sort();
            keys.iter().for_each(|key| {
                let value = self.arg.get(*key).unwrap();
                lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "ARG".into(),
                    content: if value.is_empty() {
                        key.to_string()
                    } else {
                        format!("{}={}", key, value)
                    },
                    options: vec![],
                }));
            });
        }

        // Env
        if !self.env.is_empty() {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "ENV".into(),
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
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "WORKDIR".into(),
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
                lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "USER".into(),
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
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "USER".into(),
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
    ) -> Result<Vec<DockerfileLine>> {
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

        Ok(vec![DockerfileLine::Instruction(DockerfileInsctruction {
            command: "RUN".into(),
            content,
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

#[derive(Debug, Clone, PartialEq)]
struct StageDependency {
    stage: String,
    path: String,
    origin: Vec<String>,
}

trait StageDependencyGetter {
    fn get_dependencies(&self, origin: &Vec<String>) -> Vec<StageDependency>;
}

impl StageDependencyGetter for Stage {
    fn get_dependencies(&self, origin: &Vec<String>) -> Vec<StageDependency> {
        let mut dependencies = vec![];
        if let FromContext::FromBuilder(builder) = &self.from {
            dependencies.push(StageDependency {
                stage: builder.clone(),
                path: "/".into(),
                origin: [origin.clone(), vec!["from".into()]].concat(),
            });
        }
        for (position, copy) in self.copy.iter().enumerate() {
            dependencies.append(&mut copy.get_dependencies(
                &[origin.clone(), vec!["copy".into(), position.to_string()]].concat(),
            ));
        }
        dependencies.append(&mut self.run.get_dependencies(origin));
        if let Some(root) = &self.root {
            dependencies.append(
                &mut root.get_dependencies(&[origin.clone(), vec!["root".into()]].concat()),
            );
        }
        dependencies
    }
}

impl StageDependencyGetter for Run {
    fn get_dependencies(&self, origin: &Vec<String>) -> Vec<StageDependency> {
        let mut dependencies = vec![];
        for (position, cache) in self.cache.iter().enumerate() {
            if let FromContext::FromBuilder(builder) = &cache.from {
                dependencies.push(StageDependency {
                    stage: builder.clone(),
                    path: cache.source.clone().unwrap_or("/".into()),
                    origin: [origin.clone(), vec!["cache".into(), position.to_string()]].concat(),
                });
            }
        }
        for (position, bind) in self.bind.iter().enumerate() {
            if let FromContext::FromBuilder(builder) = &bind.from {
                dependencies.push(StageDependency {
                    stage: builder.clone(),
                    path: bind.source.clone().unwrap_or("/".into()),
                    origin: [origin.clone(), vec!["bind".into(), position.to_string()]].concat(),
                });
            }
        }
        dependencies
    }
}

impl StageDependencyGetter for CopyResource {
    fn get_dependencies(&self, origin: &Vec<String>) -> Vec<StageDependency> {
        match self {
            CopyResource::Copy(copy) => match &copy.from {
                FromContext::FromBuilder(builder) => copy
                    .paths
                    .iter()
                    .map(|path| StageDependency {
                        stage: builder.clone(),
                        path: path.clone(),
                        origin: origin.clone(),
                    })
                    .collect(),
                _ => vec![],
            },
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LintSession {
    messages: Vec<LintMessage>,
    stage_infos: HashMap<String, StageLintInfo>,
    recursive_stage_dependencies: HashMap<String, Vec<String>>,
}

impl LintSession {
    pub fn get_sorted_builders(&mut self) -> Vec<String> {
        let mut stages: Vec<(String, Vec<String>)> = self
            .stage_infos
            .clone()
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    self.get_stage_recursive_dependencies(name.clone()),
                )
            })
            .collect();

        stages.sort_by(|(a_stage, a_deps), (b_stage, b_deps)| {
            if a_deps.contains(b_stage) {
                return std::cmp::Ordering::Greater;
            }
            if b_deps.contains(a_stage) {
                return std::cmp::Ordering::Less;
            }
            a_stage.cmp(b_stage)
        });

        stages
            .into_iter()
            .map(|(stage, _)| stage)
            .filter(|name| *name != "runtime")
            .collect()
    }

    pub fn get_stage_recursive_dependencies(&mut self, stage: String) -> Vec<String> {
        self.resolve_stage_recursive_dependencies(&mut vec![stage])
    }

    fn resolve_stage_recursive_dependencies(&mut self, path: &mut Vec<String>) -> Vec<String> {
        let stage = &path.last().expect("The path is empty").clone();
        if let Some(dependencies) = self.recursive_stage_dependencies.get(stage) {
            return dependencies.clone();
        }
        let mut deps = HashSet::new();
        let dependencies = self
            .stage_infos
            .get(stage)
            .expect(format!("The stage info not found for stage '{}'", stage).as_str())
            .dependencies
            .clone();
        for dependency in dependencies {
            let dep_stage = &dependency.stage;
            if path.contains(dep_stage) {
                self.messages.push(LintMessage {
                    level: MessageLevel::Error,
                    message: format!(
                        "Circular dependency detected: {} -> {}",
                        path.join(" -> "),
                        dependency.stage
                    ),
                    path: dependency.origin.clone(),
                });
                continue;
            }
            deps.insert(dep_stage.clone());
            if self.stage_infos.contains_key(dep_stage) {
                path.push(dep_stage.clone());
                deps.extend(self.resolve_stage_recursive_dependencies(path));
                path.pop();
            } // the else is already managed in check_dependencies
        }
        let deps: Vec<String> = deps.into_iter().collect();
        self.recursive_stage_dependencies
            .insert(stage.clone(), deps.clone());
        deps
    }

    /// Checks if dependencies are using path that are in cache
    fn check_dependencies(&mut self) {
        let dependencies = self
            .stage_infos
            .iter()
            .flat_map(|(_name, info)| info.dependencies.clone())
            .collect::<Vec<_>>();

        let caches = self
            .stage_infos
            .iter()
            .map(|(name, info)| (name, info.cache_paths.clone()))
            .collect::<HashMap<_, _>>();

        for dependency in dependencies {
            if let Some(paths) = caches.get(&dependency.stage) {
                paths
                    .iter()
                    .filter(|path| dependency.path.starts_with(*path))
                    .for_each(|path| {
                        self.messages.push(LintMessage {
                            level: MessageLevel::Error,
                            message: format!(
                                "Use of the '{}' builder cache path {}",
                                dependency.stage, path
                            ),
                            path: dependency.origin.clone(),
                        });
                    });
            } else {
                self.messages.push(LintMessage {
                    level: MessageLevel::Error,
                    message: format!("The builder '{}' not found", dependency.stage),
                    path: dependency.origin.clone(),
                });
            }
        }
    }

    fn analyze_stage(&mut self, path: &Vec<String>, name: &String, stage: &Stage) {
        let dependencies = stage.get_dependencies(path);
        self.messages.append(
            &mut dependencies
                .iter()
                .filter(|dep| dep.stage == "runtime")
                .map(|dep| LintMessage {
                    level: MessageLevel::Error,
                    message: format!("The builder '{}' can't depend on the 'runtime'", name,),
                    path: dep.origin.clone(),
                })
                .collect(),
        );
        let cache_paths = self.get_stage_cache_paths(stage, path);
        self.stage_infos.insert(
            name.clone(),
            StageLintInfo {
                dependencies,
                cache_paths,
            },
        );
    }

    fn get_stage_cache_paths(&mut self, stage: &Stage, path: &Vec<String>) -> Vec<String> {
        let mut paths = vec![];
        paths.append(&mut self.get_run_cache_paths(&stage.run, path, &stage.workdir));
        if let Some(root) = &stage.root {
            paths.append(&mut self.get_run_cache_paths(
                root,
                &[path.clone(), vec!["root".into()]].concat(),
                &stage.workdir,
            ));
        }
        paths
    }

    fn get_run_cache_paths(
        &mut self,
        run: &Run,
        path: &Vec<String>,
        workdir: &Option<String>,
    ) -> Vec<String> {
        let mut cache_paths = vec![];
        for (position, cache) in run.cache.iter().enumerate() {
            let target = cache.target.clone();
            cache_paths.push(if target.starts_with("/") {
                target.clone()
            } else {
                if let Some(workdir) = workdir {
                    format!("{}/{}", workdir, target)
                }
                else {
                    self.messages.push(LintMessage {
                        level: MessageLevel::Warn,
                        message: "The cache target should be absolute or a workdir should be defined in the stage".to_string(),
                        path: [path.clone(), vec!["cache".into(), position.to_string()]].concat(),
                    });
                    target.clone()
                }
            });
        }
        cache_paths
    }

    ////////// Statics //////////

    /// Analyze the given Dofigen configuration and return a lint session
    pub fn analyze(dofigen: &Dofigen) -> Self {
        let mut session = Self::default();
        for (name, builder) in dofigen.builders.iter() {
            let base_origin = vec!["builders".into(), name.clone()];
            if name == "runtime" {
                session.messages.push(LintMessage {
                    level: MessageLevel::Error,
                    message: "The builder name 'runtime' is reserved".into(),
                    path: base_origin.clone(),
                });
            }
            session.analyze_stage(&base_origin, name, builder);
        }

        session.analyze_stage(&vec![], &"runtime".into(), &dofigen.stage);
        session.check_dependencies();

        session
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StageLintInfo {
    dependencies: Vec<StageDependency>,
    cache_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LintMessage {
    pub level: MessageLevel,
    pub path: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageLevel {
    Warn,
    Error,
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    impl Default for GenerationContext {
        fn default() -> Self {
            Self {
                user: None,
                stage_name: String::default(),
                default_from: FromContext::default(),
                lint_session: LintSession::default(),
                state_stack: vec![],
            }
        }
    }

    mod stage {
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
                    DockerfileLine::Comment("test".into()),
                    DockerfileLine::Instruction(DockerfileInsctruction {
                        command: "FROM".into(),
                        content: "scratch AS test".into(),
                        options: vec![],
                    }),
                    DockerfileLine::Instruction(DockerfileInsctruction {
                        command: "ARG".into(),
                        content: "arg1=value1".into(),
                        options: vec![],
                    }),
                    DockerfileLine::Instruction(DockerfileInsctruction {
                        command: "ARG".into(),
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
                vec![DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "COPY".into(),
                    content: "\"/path/to/file\" \"/app/\"".into(),
                    options: vec![
                        InstructionOption::WithValue("chmod".into(), "755".into()),
                        InstructionOption::Flag("link".into())
                    ],
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
                vec![DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "RUN".into(),
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
                vec![DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "RUN".into(),
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "cache".into()),
                            InstructionOptionOption::new("target", "/path/to/cache".into()),
                            InstructionOptionOption::new("sharing", "locked".into()),
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
                vec![DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "RUN".into(),
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
                vec![DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "RUN".into(),
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
    }

    mod lint_session {
        use super::*;

        #[test]
        fn builders_dependencies() {
            let dofigen = Dofigen {
                builders: HashMap::from([
                    (
                        "builder1".into(),
                        Stage {
                            copy: vec![CopyResource::Copy(Copy {
                                from: FromContext::FromBuilder("builder2".into()),
                                paths: vec!["/path/to/copy".into()],
                                options: Default::default(),
                                ..Default::default()
                            })],
                            ..Default::default()
                        },
                    ),
                    (
                        "builder2".into(),
                        Stage {
                            copy: vec![CopyResource::Copy(Copy {
                                from: FromContext::FromBuilder("builder3".into()),
                                paths: vec!["/path/to/copy".into()],
                                options: Default::default(),
                                ..Default::default()
                            })],
                            ..Default::default()
                        },
                    ),
                    (
                        "builder3".into(),
                        Stage {
                            run: Run {
                                run: vec!["echo Hello".into()].into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ),
                ]),
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut dependencies = lint_session.get_stage_recursive_dependencies("runtime".into());
            dependencies.sort();
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            dependencies = lint_session.get_stage_recursive_dependencies("builder1".into());
            dependencies.sort();
            assert_eq_sorted!(dependencies, vec!["builder2", "builder3"]);

            dependencies = lint_session.get_stage_recursive_dependencies("builder2".into());
            assert_eq_sorted!(dependencies, vec!["builder3"]);

            dependencies = lint_session.get_stage_recursive_dependencies("builder3".into());
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, vec!["builder1", "builder2", "builder3"]);

            assert_eq_sorted!(lint_session.messages, vec![]);
        }

        #[test]
        fn builders_circular_dependencies() {
            let dofigen = Dofigen {
                builders: HashMap::from([
                    (
                        "builder1".into(),
                        Stage {
                            copy: vec![CopyResource::Copy(Copy {
                                from: FromContext::FromBuilder("builder2".into()),
                                paths: vec!["/path/to/copy".into()],
                                options: Default::default(),
                                ..Default::default()
                            })],
                            ..Default::default()
                        },
                    ),
                    (
                        "builder2".into(),
                        Stage {
                            copy: vec![CopyResource::Copy(Copy {
                                from: FromContext::FromBuilder("builder3".into()),
                                paths: vec!["/path/to/copy".into()],
                                options: Default::default(),
                                ..Default::default()
                            })],
                            ..Default::default()
                        },
                    ),
                    (
                        "builder3".into(),
                        Stage {
                            copy: vec![CopyResource::Copy(Copy {
                                from: FromContext::FromBuilder("builder1".into()),
                                paths: vec!["/path/to/copy".into()],
                                options: Default::default(),
                                ..Default::default()
                            })],
                            ..Default::default()
                        },
                    ),
                ]),
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut dependencies = lint_session.get_stage_recursive_dependencies("runtime".into());
            dependencies.sort();
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            dependencies = lint_session.get_stage_recursive_dependencies("builder1".into());
            dependencies.sort();
            assert_eq_sorted!(dependencies, vec!["builder2", "builder3"]);

            dependencies = lint_session.get_stage_recursive_dependencies("builder2".into());
            assert_eq_sorted!(dependencies, vec!["builder3"]);

            dependencies = lint_session.get_stage_recursive_dependencies("builder3".into());
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, vec!["builder1", "builder2", "builder3"]);

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Error,
                    path: vec![
                        "builders".into(),
                        "builder3".into(),
                        "copy".into(),
                        "0".into(),
                    ],
                    message:
                        "Circular dependency detected: builder1 -> builder2 -> builder3 -> builder1"
                            .into(),
                },]
            );
        }

        #[test]
        fn builder_named_runtime() {
            let dofigen = Dofigen {
                builders: HashMap::from([(
                    "runtime".into(),
                    Stage {
                        run: Run {
                            run: vec!["echo Hello".into()].into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, Vec::<String>::new());

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Error,
                    path: vec!["builders".into(), "runtime".into(),],
                    message: "The builder name 'runtime' is reserved".into(),
                },]
            );
        }

        #[test]
        fn builder_not_found() {
            let dofigen = Dofigen {
                stage: Stage {
                    from: FromContext::FromBuilder("builder1".into()),
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, Vec::<String>::new());

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Error,
                    path: vec!["from".into(),],
                    message: "The builder 'builder1' not found".into(),
                },]
            );
        }

        #[test]
        fn dependency_to_runtime() {
            let dofigen = Dofigen {
                builders: HashMap::from([(
                    "builder".into(),
                    Stage {
                        copy: vec![CopyResource::Copy(Copy {
                            from: FromContext::FromBuilder("runtime".into()),
                            paths: vec!["/path/to/copy".into()],
                            ..Default::default()
                        })],
                        ..Default::default()
                    },
                )]),
                stage: Stage {
                    run: Run {
                        run: vec!["echo Hello".into()].into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, vec!["builder"]);

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Error,
                    path: vec![
                        "builders".into(),
                        "builder".into(),
                        "copy".into(),
                        "0".into()
                    ],
                    message: "The builder 'builder' can't depend on the 'runtime'".into(),
                },]
            );
        }

        #[test]
        fn dependency_to_cache_path() {
            let dofigen = Dofigen {
                builders: HashMap::from([
                    (
                        "builder1".into(),
                        Stage {
                            run: Run {
                                run: vec!["echo Hello".into()].into(),
                                cache: vec![Cache {
                                    target: "/path/to/cache".into(),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ),
                    (
                        "builder2".into(),
                        Stage {
                            copy: vec![CopyResource::Copy(Copy {
                                from: FromContext::FromBuilder("builder1".into()),
                                paths: vec!["/path/to/cache/test".into()],
                                ..Default::default()
                            })],
                            ..Default::default()
                        },
                    ),
                ]),
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, vec!["builder1", "builder2"]);

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Error,
                    path: vec![
                        "builders".into(),
                        "builder2".into(),
                        "copy".into(),
                        "0".into()
                    ],
                    message: "Use of the 'builder1' builder cache path /path/to/cache".into(),
                },]
            );
        }

        #[test]
        fn runtime_dependencies() {
            let dofigen = Dofigen {
                builders: HashMap::from([
                    (
                        "install-deps".to_string(),
                        Stage {
                            from: FromContext::FromImage(ImageName {
                                path: "php".to_string(),
                                version: Some(ImageVersion::Tag("8.3-fpm-alpine".to_string())),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    ),
                    (
                        "install-php-ext".to_string(),
                        Stage {
                            from: FromContext::FromBuilder("install-deps".to_string()),
                            ..Default::default()
                        },
                    ),
                    (
                        "get-composer".to_string(),
                        Stage {
                            from: FromContext::FromImage(ImageName {
                                path: "composer".to_string(),
                                version: Some(ImageVersion::Tag("latest".to_string())),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    ),
                ]),
                stage: Stage {
                    from: FromContext::FromBuilder("install-php-ext".to_string()),
                    copy: vec![CopyResource::Copy(Copy {
                        from: FromContext::FromBuilder("get-composer".to_string()),
                        paths: vec!["/usr/bin/composer".to_string()],
                        options: CopyOptions {
                            target: Some("/bin/".to_string()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })],
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut dependencies =
                lint_session.get_stage_recursive_dependencies("install-deps".into());
            dependencies.sort();
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            dependencies = lint_session.get_stage_recursive_dependencies("install-php-ext".into());
            assert_eq_sorted!(dependencies, vec!["install-deps"]);

            dependencies = lint_session.get_stage_recursive_dependencies("get-composer".into());
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            dependencies = lint_session.get_stage_recursive_dependencies("runtime".into());
            dependencies.sort();
            assert_eq_sorted!(
                dependencies,
                vec!["get-composer", "install-deps", "install-php-ext"]
            );

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(
                builders,
                vec!["get-composer", "install-deps", "install-php-ext"]
            );

            assert_eq_sorted!(lint_session.messages, vec![]);
        }
    }
}

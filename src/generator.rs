use std::collections::{HashMap, HashSet};

use crate::{dockerfile_struct::*, dofigen_struct::*, Error, Result, DOCKERFILE_VERSION};

pub const LINE_SEPARATOR: &str = " \\\n    ";
pub const DEFAULT_FROM: &str = "scratch";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenerationContext {
    pub user: Option<User>,
    pub wokdir: Option<String>,
    pub stage_name: String,
    pub default_from: FromContext,
}
pub trait DockerfileGenerator {
    fn generate_dockerfile_lines(&self, context: &GenerationContext)
        -> Result<Vec<DockerfileLine>>;
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

impl DockerfileGenerator for Dofigen {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let mut context: GenerationContext = GenerationContext {
            user: None,
            wokdir: None,
            stage_name: String::new(),
            default_from: self.stage.from(context).clone(),
        };
        let mut lines = vec![
            DockerfileLine::Comment(format!("syntax=docker/dockerfile:{}", DOCKERFILE_VERSION)),
            DockerfileLine::Empty,
        ];

        let stage_resolver = &mut StagesDependencyResolver::new(self);

        for name in stage_resolver.get_sorted_builders()? {
            context.stage_name = name.clone();
            let builder = self
                .builders
                .get(&name)
                .ok_or(Error::Custom(format!("The builder '{}' not found", name)))?;
            lines.append(&mut Stage::generate_dockerfile_lines(builder, &context)?);
            lines.push(DockerfileLine::Empty);
        }
        context.user = Some(User::new("1000"));
        context.stage_name = "runtime".into();
        context.default_from = FromContext::default();
        lines.append(&mut self.stage.generate_dockerfile_lines(&context)?);
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
        context: &GenerationContext,
    ) -> Result<Vec<DockerfileLine>> {
        let context = GenerationContext {
            user: self.user(context),
            wokdir: self.workdir.clone(),
            ..context.clone()
        };
        let stage_name = context.stage_name.clone();

        // From
        let mut lines = vec![
            DockerfileLine::Comment(stage_name.clone()),
            DockerfileLine::Instruction(DockerfileInsctruction {
                command: "FROM".into(),
                content: format!(
                    "{image_name} AS {stage_name}",
                    image_name = self.from(&context).to_string()
                ),
                options: vec![],
            }),
        ];

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
            lines.append(&mut copy.generate_dockerfile_lines(&context)?);
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

                let root_context = GenerationContext {
                    user: Some(root_user),
                    ..context.clone()
                };
                // Run
                lines.append(&mut root.generate_dockerfile_lines(&root_context)?);
            }
        }

        // User
        if let Some(user) = self.user(&context) {
            lines.push(DockerfileLine::Instruction(DockerfileInsctruction {
                command: "USER".into(),
                content: user.to_string(),
                options: vec![],
            }));
        }

        // Run
        lines.append(&mut self.run.generate_dockerfile_lines(&context)?);

        Ok(lines)
    }
}

impl DockerfileGenerator for Run {
    fn generate_dockerfile_lines(
        &self,
        context: &GenerationContext,
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
            let mut target = cache.target.clone();

            // Manage relative paths
            if !target.starts_with("/") {
                target = format!(
                    "{}/{}",
                    context.wokdir.clone().ok_or(Error::Custom(
                        "The cache target must be absolute or a workdir must be defined"
                            .to_string()
                    ))?,
                    target
                );
            }

            let mut cache_options = vec![
                InstructionOptionOption::new("type", "cache".into()),
                InstructionOptionOption::new("target", target),
            ];
            if let Some(id) = cache.id.as_ref() {
                cache_options.push(InstructionOptionOption::new("id", id.clone()));
            }
            if let Some(from) = cache.from.as_ref() {
                cache_options.push(InstructionOptionOption::new("from", from.clone()));
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
            if let Some(sharing) = cache.sharing.as_ref() {
                cache_options.push(InstructionOptionOption::new("sharing", sharing.to_string()));
            }
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

impl Stage {
    pub(crate) fn get_dependencies(&self) -> Vec<String> {
        let mut dependencies = vec![];
        if let FromContext::FromBuilder(builder) = &self.from {
            dependencies.push(builder.clone());
        }
        for copy in self.copy.iter() {
            dependencies.append(&mut copy.get_dependencies());
        }
        dependencies
    }
}

impl CopyResource {
    pub(crate) fn get_dependencies(&self) -> Vec<String> {
        match self {
            CopyResource::Copy(copy) => match &copy.from {
                FromContext::FromBuilder(builder) => vec![builder.clone()],
                _ => vec![],
            },
            _ => vec![],
        }
    }
}

struct StagesDependencyResolver {
    dependencies: HashMap<String, Vec<String>>,
    recursive_dependencies: HashMap<String, Vec<String>>,
}

impl StagesDependencyResolver {
    pub fn get_sorted_builders(&mut self) -> Result<Vec<String>> {
        let mut stages: Vec<(String, Vec<String>)> = self
            .dependencies
            .clone()
            .keys()
            .into_iter()
            .filter(|stage| **stage != "runtime")
            .map(|stage| Ok((stage.clone(), self.resolve_dependencies(stage.clone())?)))
            .collect::<Result<_>>()?;

        stages.sort_by(|(a_stage, a_deps), (b_stage, b_deps)| {
            if a_deps.contains(b_stage) {
                return std::cmp::Ordering::Greater;
            }
            if b_deps.contains(a_stage) {
                return std::cmp::Ordering::Less;
            }
            a_stage.cmp(b_stage)
        });

        Ok(stages.into_iter().map(|(stage, _)| stage).collect())
    }

    pub fn resolve_dependencies(&mut self, stage: String) -> Result<Vec<String>> {
        self.resolve_recursive_dependencies(&mut vec![stage])
    }

    fn resolve_recursive_dependencies(&mut self, path: &mut Vec<String>) -> Result<Vec<String>> {
        let stage = path
            .last()
            .ok_or(Error::Custom("The path is empty".to_string()))?
            .clone();
        if let Some(dependencies) = self.recursive_dependencies.get(&stage) {
            return Ok(dependencies.clone());
        }
        let mut deps = HashSet::new();
        let dependencies = self
            .dependencies
            .get(&stage)
            .ok_or(Error::Custom(format!(
                "The stage dependencies {} not found",
                stage
            )))?
            .clone();
        for dependency in dependencies {
            if path.contains(&dependency) {
                return Err(Error::Custom(format!(
                    "Circular dependency detected: {} -> {}",
                    path.join(" -> "),
                    dependency
                )));
            }
            deps.insert(dependency.clone());
            path.push(dependency.clone());
            deps.extend(self.resolve_recursive_dependencies(path)?);
            path.pop();
        }
        let deps: Vec<String> = deps.into_iter().collect();
        self.recursive_dependencies
            .insert(stage.clone(), deps.clone());
        Ok(deps)
    }

    pub fn new(dofigen: &Dofigen) -> Self {
        let mut dependencies: HashMap<String, Vec<String>> = dofigen
            .builders
            .iter()
            .map(|(name, builder)| {
                if name == "runtime" {
                    panic!("The builder name 'runtime' is reserved");
                }
                let deps = builder.get_dependencies();
                if deps.contains(&"runtime".to_string()) {
                    panic!("The builder '{}' can't depend on the 'runtime'", name);
                }
                (name.clone(), deps)
            })
            .collect();

        dependencies.insert("runtime".into(), dofigen.stage.get_dependencies());
        Self {
            dependencies,
            recursive_dependencies: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod builder {
        use super::*;

        #[test]
        fn user_with_user() {
            let builder = Stage {
                user: Some(User::new_without_group("my-user").into()),
                ..Default::default()
            };
            let user = builder.user(&GenerationContext::default());
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
            let builder = Stage::default();
            let user = builder.user(&GenerationContext::default());
            assert_eq_sorted!(user, None);
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
                    .generate_dockerfile_lines(&GenerationContext::default())
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
                    .generate_dockerfile_lines(&GenerationContext::default())
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
                    .generate_dockerfile_lines(&GenerationContext::default())
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
            let context = GenerationContext {
                user: Some(User::new("test")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&context).unwrap(),
                vec![DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "RUN".into(),
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "cache".into()),
                            InstructionOptionOption::new("target", "/path/to/cache".into()),
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
            let context = GenerationContext {
                user: Some(User::new("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&context).unwrap(),
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
            let context = GenerationContext {
                user: Some(User::new_without_group("1000")),
                ..Default::default()
            };
            assert_eq_sorted!(
                builder.generate_dockerfile_lines(&context).unwrap(),
                vec![DockerfileLine::Instruction(DockerfileInsctruction {
                    command: "RUN".into(),
                    content: "echo Hello".into(),
                    options: vec![InstructionOption::WithOptions(
                        "mount".into(),
                        vec![
                            InstructionOptionOption::new("type", "cache".into()),
                            InstructionOptionOption::new("target", "/path/to/cache".into()),
                            InstructionOptionOption::new("uid", "1000".into()),
                        ],
                    )],
                })]
            );
        }
    }

    mod stages_dependency_resolver {
        use super::*;

        #[test]
        fn resolve_builders_dependencies() {
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

            let mut resolver = StagesDependencyResolver::new(&dofigen);

            let mut dependencies = resolver.resolve_dependencies("runtime".into()).unwrap();
            dependencies.sort();
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            dependencies = resolver.resolve_dependencies("builder1".into()).unwrap();
            dependencies.sort();
            assert_eq_sorted!(dependencies, vec!["builder2", "builder3"]);

            dependencies = resolver.resolve_dependencies("builder2".into()).unwrap();
            assert_eq_sorted!(dependencies, vec!["builder3"]);

            dependencies = resolver.resolve_dependencies("builder3".into()).unwrap();
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            let mut builders = resolver.get_sorted_builders().unwrap();
            builders.sort();

            assert_eq_sorted!(builders, vec!["builder1", "builder2", "builder3"]);
        }

        #[test]
        fn resolve_runtime_dependencies() {
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

            let mut resolver = StagesDependencyResolver::new(&dofigen);

            let mut dependencies = resolver
                .resolve_dependencies("install-deps".into())
                .unwrap();
            dependencies.sort();
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            dependencies = resolver
                .resolve_dependencies("install-php-ext".into())
                .unwrap();
            assert_eq_sorted!(dependencies, vec!["install-deps"]);

            dependencies = resolver
                .resolve_dependencies("get-composer".into())
                .unwrap();
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            dependencies = resolver.resolve_dependencies("runtime".into()).unwrap();
            dependencies.sort();
            assert_eq_sorted!(
                dependencies,
                vec!["get-composer", "install-deps", "install-php-ext"]
            );

            let mut builders = resolver.get_sorted_builders().unwrap();
            builders.sort();

            assert_eq_sorted!(
                builders,
                vec!["get-composer", "install-deps", "install-php-ext"]
            );
        }
    }
}

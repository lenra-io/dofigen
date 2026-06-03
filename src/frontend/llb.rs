use std::{collections::HashMap, path::PathBuf, sync::Arc};

use buildkit_llb::{
    ops::{
        Command, FileSystem, MergeOperation, MultiOwnedLastOutput, MultiOwnedOutput,
        OperationBuilder, Platform, SingleOwnedOutput, Source,
        fs::{FileOperation, SequenceOperation},
        source::LocalSource,
    },
    pb,
    prelude::{LayerPath, Mount, OwnOutputIdx},
    utils::{OperationOutput, OutputIdx},
};
use dofigen_lib::{
    CacheSharing, CopyOptions, CopyResource, Dofigen, FromContext, LintSession, Network, Resource,
    Run, Security, Stage, User,
};
use failure::{Error, format_err};

pub struct LlbBuilder {
    dofigen: Dofigen,
    context: Arc<LocalSource>,
    lint_session: LintSession,
    platform: Option<Platform>,
}

impl LlbBuilder {
    pub fn new(dofigen: Dofigen, platform: Option<Platform>) -> Self {
        let context = Self::init_context(&dofigen);
        let lint_session = LintSession::analyze(&dofigen);
        Self {
            dofigen,
            context,
            lint_session,
            platform,
        }
    }

    fn init_context(dofigen: &Dofigen) -> Arc<LocalSource> {
        let context = Source::local("context");
        let context = dofigen.context.iter().fold(context, |ctx, pattern| {
            ctx.add_include_pattern(pattern.clone())
        });
        let context = dofigen.ignore.iter().fold(context, |ctx, pattern| {
            ctx.add_exclude_pattern(pattern.clone())
        });
        context.ref_counted()
    }

    pub fn build(&mut self) -> Result<OperationOutput<'static>, Error> {
        let mut stage_outputs: HashMap<String, OperationOutput<'static>> = HashMap::new();
        let builder_names = self.lint_session.get_sorted_builders();

        for name in builder_names {
            let builder = self
                .dofigen
                .builders
                .get(&name)
                .ok_or_else(|| format_err!("The builder '{}' was not found", name))?;
            let output = self.stage_to_llb(builder, &stage_outputs)?;
            stage_outputs.insert(name.to_string(), output);
        }

        self.stage_to_llb(&self.dofigen.stage, &stage_outputs)
    }

    fn stage_to_llb(
        &self,
        stage: &Stage,
        stage_outputs: &HashMap<String, OperationOutput<'static>>,
    ) -> Result<OperationOutput<'static>, Error> {
        let base: Option<OperationOutput<'static>> = match &stage.from {
            FromContext::FromImage(image) => {
                Some(self.image_source(image.to_string()).ref_counted().output())
            }
            FromContext::FromBuilder(name) => Some(stage_outputs[name].clone()),
            FromContext::FromContext(Some(ctx_name)) => {
                Some(Source::local(ctx_name.clone()).ref_counted().output())
            }
            FromContext::FromContext(None) => Some(self.context.output()),
        };

        let mut build = StageBuild::new(base);

        // WORKDIR → mkdir
        let workdir = if let Some(workdir) = &stage.workdir {
            let dest = self.dest_layer(
                &PathBuf::from("/"),
                &stage.workdir,
                &build.base,
                build.last_own_idx,
            );
            let idx = build.output_idx();
            build.append(FileSystem::mkdir(idx, dest).make_parents(true));
            PathBuf::from(workdir)
        } else {
            // TODO: maybe get the workdir from the base image or builder if not specified?
            PathBuf::from("/")
        };

        // COPY / ADD resources
        for copy_resource in &stage.copy {
            self.apply_copy_resource(&mut build, copy_resource, &workdir, stage_outputs)?;
        }

        let mut current: OperationOutput<'static> = if build.last_own_idx.is_some() {
            build
                .sequence
                .ref_counted()
                .last_output()
                .ok_or_else(|| format_err!("the file operation sequence produced no output"))?
        } else {
            build.base.clone().unwrap_or_else(|| self.context.output())
        };

        // Root RUN (runs as root user)
        if let Some(root) = &stage.root {
            if !root.run.is_empty() {
                current = self.apply_run(
                    current,
                    root,
                    Some("root"),
                    &stage.env,
                    &stage.workdir,
                    stage_outputs,
                )?;
            }
        }

        // Regular RUN
        if !stage.run.run.is_empty() {
            let user_str = stage.user.as_ref().map(|u| u.to_string());
            current = self.apply_run(
                current,
                &stage.run,
                user_str.as_deref(),
                &stage.env,
                &stage.workdir,
                stage_outputs,
            )?;
        }

        Ok(current)
    }

    fn apply_run(
        &self,
        base: OperationOutput<'static>,
        run: &Run,
        user: Option<&str>,
        env: &HashMap<String, String>,
        workdir: &Option<String>,
        stage_outputs: &HashMap<String, OperationOutput<'static>>,
    ) -> Result<OperationOutput<'static>, Error> {
        let script = run.run.join("\n");

        // SHELL: use the custom shell when provided, otherwise default to
        // `/bin/sh -c`. The shell is `[executable, args...]` and the script is
        // appended as the final argument.
        let (program, mut args): (&str, Vec<String>) = if run.shell.is_empty() {
            ("/bin/sh", vec!["-c".to_string()])
        } else {
            (run.shell[0].as_str(), run.shell[1..].to_vec())
        };
        args.push(script);

        let mut cmd = Command::run(program)
            .args(args.iter().map(String::as_str))
            .env_iter(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .mount(Mount::Layer(OutputIdx(0), base, "/"));

        if let Some(wd) = workdir {
            cmd = cmd.cwd(wd.as_str());
        }

        if let Some(u) = user {
            cmd = cmd.user(u);
        }

        // Cache mounts
        for cache in &run.cache {
            let opt = pb::CacheOpt {
                id: cache.id.clone().unwrap_or_else(|| cache.target.clone()),
                sharing: cache
                    .sharing
                    .as_ref()
                    .map(cache_sharing)
                    .unwrap_or(pb::CacheSharingOpt::Shared) as i32,
            };
            let readonly = cache.readonly.unwrap_or(false);

            if cache.chmod.is_some() || cache.chown.is_some() {
                // chmod/chown: BuildKit can't set these on the cache mount
                // directly, so seed it from a directory created with the desired
                // attributes and use that directory as the cache source.
                if !cache.from.is_empty() {
                    warn_unsupported(
                        "cache mount from combined with chmod/chown",
                        "the chmod/chown seed takes precedence over from",
                    );
                }
                let seed = "/cache";
                let mut mkdir =
                    FileSystem::mkdir(OutputIdx(0), LayerPath::Scratch(PathBuf::from(seed)));
                if let Some(mode) = cache.chmod.as_deref().and_then(parse_chmod) {
                    mkdir = mkdir.chmod(mode);
                }
                if let Some(chown) = &cache.chown {
                    mkdir = mkdir.chown(to_chown_opt(chown));
                }
                let seed_output = mkdir
                    .into_operation()
                    .ref_counted()
                    .last_output()
                    .ok_or_else(|| format_err!("the cache seed produced no output"))?;
                cmd = cmd.mount(Mount::CacheFrom(
                    seed_output,
                    cache.target.as_str(),
                    seed,
                    opt,
                    readonly,
                ));
            } else if !cache.from.is_empty() {
                // Seed the cache from another stage/image/context.
                let from_output = self.resolve_copy_from(&cache.from, stage_outputs);
                let selector = cache.source.as_deref().unwrap_or("");
                cmd = cmd.mount(Mount::CacheFrom(
                    from_output,
                    cache.target.as_str(),
                    selector,
                    opt,
                    readonly,
                ));
            } else {
                if cache.source.is_some() {
                    warn_unsupported(
                        "cache mount source",
                        "only meaningful together with from; ignored",
                    );
                }
                cmd = cmd.mount(Mount::Cache(cache.target.as_str(), opt, readonly));
            }
        }

        // SSH agent mounts
        for ssh in &run.ssh {
            let path = ssh.target.as_deref().unwrap_or("/run/buildkit/ssh_agent.0");
            let opt = pb::SshOpt {
                id: ssh.id.clone().unwrap_or_else(|| "default".to_string()),
                uid: ssh.uid.map(u32::from).unwrap_or(0),
                gid: ssh.gid.map(u32::from).unwrap_or(0),
                mode: ssh
                    .mode
                    .as_deref()
                    .and_then(parse_chmod)
                    .map(|m| m as u32)
                    .unwrap_or(0o600),
                optional: !ssh.required.unwrap_or(false),
            };
            cmd = cmd.mount(Mount::Ssh(path, opt));
        }

        // Bind mounts (read-only by default, read-write when requested)
        for bind in &run.bind {
            let from_output = self.resolve_copy_from(&bind.from, stage_outputs);
            let readwrite = bind.readwrite.unwrap_or(false);
            cmd = cmd.mount(match (&bind.source, readwrite) {
                (Some(src), true) => {
                    Mount::ReadWriteSelector(from_output, bind.target.as_str(), src.as_str())
                }
                (Some(src), false) => {
                    Mount::ReadOnlySelector(from_output, bind.target.as_str(), src.as_str())
                }
                (None, true) => Mount::ReadWriteLayer(from_output, bind.target.as_str()),
                (None, false) => Mount::ReadOnlyLayer(from_output, bind.target.as_str()),
            });
        }

        // tmpfs mounts (size 0 means unlimited)
        for tmpfs in &run.tmpfs {
            let size = tmpfs
                .size
                .as_deref()
                .and_then(parse_size_bytes)
                .unwrap_or(0);
            cmd = cmd.mount(Mount::Tmpfs(tmpfs.target.as_str(), pb::TmpfsOpt { size }));
        }

        // Secret mounts (file and/or environment variable)
        for secret in &run.secret {
            let id = secret.id.clone().unwrap_or_else(|| {
                secret
                    .target
                    .as_deref()
                    .and_then(|t| std::path::Path::new(t).file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .or_else(|| secret.env.clone())
                    .unwrap_or_default()
            });
            let optional = !secret.required.unwrap_or(false);

            // Mount the secret into an environment variable when requested.
            if let Some(env_name) = &secret.env {
                cmd = cmd.secret_env(pb::SecretEnv {
                    id: id.clone(),
                    name: env_name.clone(),
                    optional,
                });
            }

            // Mount the secret as a file when a target is given, or by default
            // when it isn't exposed as an environment variable.
            if secret.target.is_some() || secret.env.is_none() {
                let target = secret
                    .target
                    .clone()
                    .unwrap_or_else(|| format!("/run/secrets/{id}"));
                let opt = pb::SecretOpt {
                    id: id.clone(),
                    uid: secret.uid.map(u32::from).unwrap_or(0),
                    gid: secret.gid.map(u32::from).unwrap_or(0),
                    mode: secret
                        .mode
                        .as_deref()
                        .and_then(parse_chmod)
                        .map(|m| m as u32)
                        .unwrap_or(0o400),
                    optional,
                };
                cmd = cmd.mount(Mount::Secret(target, opt));
            }
        }

        // RUN --network
        if let Some(network) = &run.network {
            cmd = cmd.network(match network {
                Network::Default => pb::NetMode::Unset,
                Network::None => pb::NetMode::None,
                Network::Host => pb::NetMode::Host,
            });
        }

        // RUN --security
        if let Some(security) = &run.security {
            cmd = cmd.security(match security {
                Security::Sandbox => pb::SecurityMode::Sandbox,
                Security::Insecure => pb::SecurityMode::Insecure,
            });
        }

        Ok(cmd.ref_counted().output(0))
    }

    fn image_source(&self, name: String) -> buildkit_llb::ops::source::ImageSource {
        let src = Source::image(name);
        match &self.platform {
            Some(p) => src.with_platform(p.clone()),
            None => src,
        }
    }

    fn resolve_copy_from(
        &self,
        from: &FromContext,
        stage_outputs: &HashMap<String, OperationOutput<'static>>,
    ) -> OperationOutput<'static> {
        match from {
            FromContext::FromImage(image) => {
                self.image_source(image.to_string()).ref_counted().output()
            }
            FromContext::FromBuilder(name) => stage_outputs[name].clone(),
            FromContext::FromContext(Some(ctx_name)) => {
                Source::local(ctx_name.clone()).ref_counted().output()
            }
            FromContext::FromContext(None) => self.context.output(),
        }
    }

    /// Joins the optional target onto the workdir to produce a destination path.
    fn joined_dest(&self, workdir: &PathBuf, target: &Option<String>) -> PathBuf {
        let target = target.as_deref().unwrap_or("");
        if workdir.as_os_str() == "/" {
            PathBuf::from(target)
        } else {
            workdir.join(target)
        }
    }

    fn dest_layer(
        &self,
        workdir: &PathBuf,
        path: &Option<String>,
        base: &Option<OperationOutput<'static>>,
        last_own_idx: Option<u32>,
    ) -> LayerPath<'static, PathBuf> {
        let path = self.joined_dest(workdir, path);
        match last_own_idx {
            Some(i) => LayerPath::Own(OwnOutputIdx(i), path),
            None => match base {
                Some(b) => LayerPath::Other(b.clone(), path),
                None => LayerPath::Scratch(path),
            },
        }
    }

    /// Translates a single COPY/ADD resource into LLB, appending it to the
    /// running sequence or, for `--link`, building it as an independent layer
    /// that is merged on top of the current state.
    fn apply_copy_resource(
        &self,
        build: &mut StageBuild,
        copy: &CopyResource,
        workdir: &PathBuf,
        stage_outputs: &HashMap<String, OperationOutput<'static>>,
    ) -> Result<(), Error> {
        match copy {
            CopyResource::Copy(c) => {
                let parents = c.parents == Some(true);
                let from_output = self.resolve_copy_from(&c.from, stage_outputs);

                let warn_parents = |path: &str| {
                    if parents && path.contains(['*', '?', '[']) {
                        warn_unsupported(
                            "COPY --parents with wildcards",
                            "the parent structure is resolved at generation time, not per matched file",
                        );
                    }
                };
                let target_for = |path: &str| {
                    if parents {
                        parents_target(&c.options.target, path)
                    } else {
                        c.options.target.clone()
                    }
                };

                if c.options.link == Some(true) {
                    let mut layers = Vec::with_capacity(c.paths.len());
                    for path in &c.paths {
                        warn_parents(path);
                        let dest_path = self.joined_dest(workdir, &target_for(path));
                        let mut op = FileSystem::copy()
                            .from(LayerPath::Other(from_output.clone(), path.as_str()))
                            .to(OutputIdx(0), LayerPath::Scratch(dest_path))
                            .create_path(true)
                            .recursive(true);
                        op = apply_chmod_chown(op, &c.options);
                        if !c.exclude.is_empty() {
                            op = op.exclude_patterns(c.exclude.clone());
                        }
                        layers.push(link_layer(op.into_operation())?);
                    }
                    self.finalize_link(build, layers)?;
                } else {
                    for path in &c.paths {
                        warn_parents(path);
                        let dest = self.dest_layer(
                            workdir,
                            &target_for(path),
                            &build.base,
                            build.last_own_idx,
                        );
                        let mut op = FileSystem::copy()
                            .from(LayerPath::Other(from_output.clone(), path.as_str()))
                            .to(build.output_idx(), dest)
                            .create_path(true)
                            .recursive(true);
                        op = apply_chmod_chown(op, &c.options);
                        if !c.exclude.is_empty() {
                            op = op.exclude_patterns(c.exclude.clone());
                        }
                        build.append(op);
                    }
                }
            }
            CopyResource::Content(cc) => {
                let make_file = |dest: LayerPath<'static, PathBuf>, idx: OutputIdx| {
                    let mut op = FileSystem::mkfile(idx, dest).data(cc.content.as_bytes().to_vec());
                    if let Some(mode) = cc.options.chmod.as_deref().and_then(parse_chmod) {
                        op = op.chmod(mode);
                    }
                    if let Some(chown) = &cc.options.chown {
                        op = op.chown(to_chown_opt(chown));
                    }
                    op
                };
                if cc.options.link == Some(true) {
                    let dest_path = self.joined_dest(workdir, &cc.options.target);
                    let op = make_file(LayerPath::Scratch(dest_path), OutputIdx(0));
                    self.finalize_link(build, vec![link_layer(op.into_operation())?])?;
                } else {
                    let dest = self.dest_layer(
                        workdir,
                        &cc.options.target,
                        &build.base,
                        build.last_own_idx,
                    );
                    let op = make_file(dest, build.output_idx());
                    build.append(op);
                }
            }
            CopyResource::AddGitRepo(ag) => {
                let mut git = Source::git(ag.repo.clone());
                if ag.keep_git_dir == Some(true) {
                    git = git.with_keep_git_dir(true);
                }
                if let Some(checksum) = &ag.checksum {
                    git = git.with_checksum(checksum.clone());
                }
                let git_output = git.ref_counted().output();

                if ag.options.link == Some(true) {
                    let dest_path = self.joined_dest(workdir, &ag.options.target);
                    let mut op = FileSystem::copy()
                        .from(LayerPath::Other(git_output, "."))
                        .to(OutputIdx(0), LayerPath::Scratch(dest_path))
                        .create_path(true)
                        .recursive(true);
                    op = apply_chmod_chown(op, &ag.options);
                    if !ag.exclude.is_empty() {
                        op = op.exclude_patterns(ag.exclude.clone());
                    }
                    self.finalize_link(build, vec![link_layer(op.into_operation())?])?;
                } else {
                    let dest = self.dest_layer(
                        workdir,
                        &ag.options.target,
                        &build.base,
                        build.last_own_idx,
                    );
                    let mut op = FileSystem::copy()
                        .from(LayerPath::Other(git_output, "."))
                        .to(build.output_idx(), dest)
                        .create_path(true)
                        .recursive(true);
                    op = apply_chmod_chown(op, &ag.options);
                    if !ag.exclude.is_empty() {
                        op = op.exclude_patterns(ag.exclude.clone());
                    }
                    build.append(op);
                }
            }
            CopyResource::Add(a) => {
                let link = a.options.link == Some(true);
                let mut layers = Vec::new();
                for file in &a.files {
                    let file_output = match file {
                        Resource::Url(url) => {
                            let mut http = Source::http(url.as_str());
                            if let Some(checksum) = &a.checksum {
                                http = http.with_checksum(checksum.clone());
                            }
                            http.ref_counted().output()
                        }
                        // Local file: reference from the build context
                        Resource::File(_path) => self.context.output(),
                    };
                    let src_path = match file {
                        Resource::Url(_) => ".".to_string(),
                        Resource::File(path) => path.to_string_lossy().into_owned(),
                    };
                    if link {
                        let dest_path = self.joined_dest(workdir, &a.options.target);
                        let mut op = FileSystem::copy()
                            .from(LayerPath::Other(file_output, src_path.as_str()))
                            .to(OutputIdx(0), LayerPath::Scratch(dest_path))
                            .create_path(true);
                        if a.unpack == Some(true) {
                            op = op.unpack(true);
                        }
                        op = apply_chmod_chown(op, &a.options);
                        layers.push(link_layer(op.into_operation())?);
                    } else {
                        let dest = self.dest_layer(
                            workdir,
                            &a.options.target,
                            &build.base,
                            build.last_own_idx,
                        );
                        let mut op = FileSystem::copy()
                            .from(LayerPath::Other(file_output, src_path.as_str()))
                            .to(build.output_idx(), dest)
                            .create_path(true);
                        if a.unpack == Some(true) {
                            op = op.unpack(true);
                        }
                        op = apply_chmod_chown(op, &a.options);
                        build.append(op);
                    }
                }
                if link {
                    self.finalize_link(build, layers)?;
                }
            }
        }
        Ok(())
    }

    /// Materialises the current state, merges the given independent `--link`
    /// layers on top of it, and resets the running sequence to continue from
    /// the merged state.
    fn finalize_link(
        &self,
        build: &mut StageBuild,
        link_layers: Vec<OperationOutput<'static>>,
    ) -> Result<(), Error> {
        let sequence = std::mem::replace(&mut build.sequence, FileSystem::sequence());
        let current_state = if build.last_own_idx.is_some() {
            sequence
                .ref_counted()
                .last_output()
                .ok_or_else(|| format_err!("the file operation sequence produced no output"))?
        } else {
            build.base.clone().unwrap_or_else(|| self.context.output())
        };

        let mut inputs = Vec::with_capacity(link_layers.len() + 1);
        inputs.push(current_state);
        inputs.extend(link_layers);

        build.base = Some(MergeOperation::new(inputs).ref_counted().output());
        build.next_idx = 0;
        build.last_own_idx = None;
        Ok(())
    }
}

/// Mutable state accumulated while translating a stage's filesystem operations.
struct StageBuild {
    sequence: SequenceOperation<'static>,
    next_idx: u32,
    last_own_idx: Option<u32>,
    base: Option<OperationOutput<'static>>,
}

impl StageBuild {
    fn new(base: Option<OperationOutput<'static>>) -> Self {
        Self {
            sequence: FileSystem::sequence(),
            next_idx: 0,
            last_own_idx: None,
            base,
        }
    }

    /// The output index to assign to the next operation in the sequence.
    fn output_idx(&self) -> OutputIdx {
        OutputIdx(self.next_idx)
    }

    /// Append a file operation to the running sequence and advance the indices.
    fn append<T>(&mut self, op: T)
    where
        T: FileOperation + 'static,
    {
        let sequence = std::mem::replace(&mut self.sequence, FileSystem::sequence());
        self.sequence = sequence.append(op);
        self.last_own_idx = self.sequence.last_output_index();
        self.next_idx += 1;
    }
}

/// Materialises a single-operation sequence into its output (a `--link` layer).
fn link_layer(sequence: SequenceOperation<'static>) -> Result<OperationOutput<'static>, Error> {
    sequence
        .ref_counted()
        .last_output()
        .ok_or_else(|| format_err!("the link copy produced no output"))
}

/// Converts a Dofigen [`User`] into a BuildKit `ChownOpt`.
fn to_chown_opt(user: &User) -> pb::ChownOpt {
    pb::ChownOpt {
        user: Some(to_user_opt(&user.user)),
        group: user.group.as_deref().map(to_user_opt),
    }
}

/// Converts a user/group reference into a BuildKit `UserOpt`, preferring a
/// numeric ID when the value parses as one.
fn to_user_opt(value: &str) -> pb::UserOpt {
    let user = match value.parse::<u32>() {
        Ok(id) => pb::user_opt::User::ById(id),
        Err(_) => pb::user_opt::User::ByName(pb::NamedUserOpt {
            name: value.to_string(),
            input: -1,
        }),
    };
    pb::UserOpt { user: Some(user) }
}

/// Parses an octal permission string (e.g. `"755"`) into mode bits.
fn parse_chmod(chmod: &str) -> Option<i32> {
    let trimmed = chmod.trim();
    let trimmed = trimmed.strip_prefix("0o").unwrap_or(trimmed);
    i32::from_str_radix(trimmed, 8).ok()
}

/// Parses a size string (e.g. `"100m"`, `"1g"`, `"512k"`, `"1024"`) into bytes.
fn parse_size_bytes(size: &str) -> Option<i64> {
    let size = size.trim();
    let last = size.chars().last()?;
    let (number, multiplier) = match last.to_ascii_lowercase() {
        'k' => (&size[..size.len() - 1], 1024_i64),
        'm' => (&size[..size.len() - 1], 1024 * 1024),
        'g' => (&size[..size.len() - 1], 1024 * 1024 * 1024),
        c if c.is_ascii_digit() => (size, 1),
        _ => return None,
    };
    number.trim().parse::<i64>().ok().map(|n| n * multiplier)
}

/// Maps a Dofigen [`CacheSharing`] to its BuildKit equivalent.
fn cache_sharing(sharing: &CacheSharing) -> pb::CacheSharingOpt {
    match sharing {
        CacheSharing::Shared => pb::CacheSharingOpt::Shared,
        CacheSharing::Private => pb::CacheSharingOpt::Private,
        CacheSharing::Locked => pb::CacheSharingOpt::Locked,
    }
}

/// Applies the `chmod`/`chown` copy options to a copy operation.
fn apply_chmod_chown<F, T>(
    mut op: buildkit_llb::ops::fs::CopyOperation<F, T>,
    options: &CopyOptions,
) -> buildkit_llb::ops::fs::CopyOperation<F, T>
where
    F: std::fmt::Debug,
    T: std::fmt::Debug,
{
    if let Some(chmod) = &options.chmod {
        // Use the octal mode bits when the value parses, otherwise fall back to
        // the raw mode string (BuildKit's `mode_str`).
        op = match parse_chmod(chmod) {
            Some(mode) => op.chmod(mode),
            None => op.chmod_str(chmod.clone()),
        };
    }
    if let Some(chown) = &options.chown {
        op = op.chown(to_chown_opt(chown));
    }
    op
}

/// Emits a one-line warning for a Dofigen feature the frontend can't translate.
fn warn_unsupported(feature: &str, context: &str) {
    eprintln!("warning: {feature} is not supported by the Dofigen BuildKit frontend ({context})");
}

/// Computes the destination for a `COPY --parents` source by appending the
/// source's parent directories to the target, preserving its hierarchy
/// (e.g. source `src/index.js` + target `/app` → `/app/src`).
fn parents_target(target: &Option<String>, source: &str) -> Option<String> {
    let parent = std::path::Path::new(source)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty());
    match (target, parent) {
        (Some(t), Some(p)) => Some(format!("{}/{}", t.trim_end_matches('/'), p)),
        (Some(t), None) => Some(t.clone()),
        (None, parent) => parent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildkit_llb::ops::Terminal;
    use dofigen_lib::{Add, Cache, Copy, ImageName};
    use prost::Message;

    fn validate_output(output: Result<OperationOutput<'static>, Error>) {
        Terminal::with(output.unwrap()).into_definition();
    }

    /// Decodes the LLB definition produced by the builder into its individual
    /// operations so that tests can assert on their contents.
    fn definition_ops(output: Result<OperationOutput<'static>, Error>) -> Vec<pb::Op> {
        let definition = Terminal::with(output.unwrap()).into_definition();
        definition
            .def
            .iter()
            .map(|bytes| pb::Op::decode(bytes.as_slice()).expect("valid LLB op"))
            .collect()
    }

    /// Returns the first execution (`RUN`) operation found in the definition.
    fn find_exec(ops: &[pb::Op]) -> &pb::ExecOp {
        ops.iter()
            .find_map(|op| match &op.op {
                Some(pb::op::Op::Exec(exec)) => Some(exec),
                _ => None,
            })
            .expect("an exec operation")
    }

    /// Collects the destination paths of all copy file actions in the definition.
    fn copy_dests(ops: &[pb::Op]) -> Vec<String> {
        ops.iter()
            .filter_map(|op| match &op.op {
                Some(pb::op::Op::File(file)) => Some(file),
                _ => None,
            })
            .flat_map(|file| file.actions.iter())
            .filter_map(|action| match &action.action {
                Some(pb::file_action::Action::Copy(copy)) => Some(copy.dest.clone()),
                _ => None,
            })
            .collect()
    }

    /// Collects all merge operations in the definition.
    fn merge_ops(ops: &[pb::Op]) -> Vec<&pb::MergeOp> {
        ops.iter()
            .filter_map(|op| match &op.op {
                Some(pb::op::Op::Merge(merge)) => Some(merge),
                _ => None,
            })
            .collect()
    }

    /// Collects all mkdir file actions in the definition.
    fn mkdir_actions(ops: &[pb::Op]) -> Vec<pb::FileActionMkDir> {
        ops.iter()
            .filter_map(|op| match &op.op {
                Some(pb::op::Op::File(file)) => Some(file),
                _ => None,
            })
            .flat_map(|file| file.actions.iter())
            .filter_map(|action| match &action.action {
                Some(pb::file_action::Action::Mkdir(mkdir)) => Some(mkdir.clone()),
                _ => None,
            })
            .collect()
    }

    /// Collects all copy file actions in the definition.
    fn copy_actions(ops: &[pb::Op]) -> Vec<pb::FileActionCopy> {
        ops.iter()
            .filter_map(|op| match &op.op {
                Some(pb::op::Op::File(file)) => Some(file),
                _ => None,
            })
            .flat_map(|file| file.actions.iter())
            .filter_map(|action| match &action.action {
                Some(pb::file_action::Action::Copy(copy)) => Some(copy.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_init_context_patterns() {
        let dofigen = Dofigen {
            context: vec!["src/**".to_string()],
            ignore: vec!["target/".to_string()],
            ..Default::default()
        };
        LlbBuilder::init_context(&dofigen);
    }

    #[test]
    fn test_build_from_image_no_ops() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            ..Default::default()
        };
        validate_output(LlbBuilder::new(dofigen, None).build());
    }

    #[test]
    fn test_build_workdir_only() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                workdir: Some("/app".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        validate_output(LlbBuilder::new(dofigen, None).build());
    }

    #[test]
    fn test_build_with_run() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                workdir: Some("/app".to_string()),
                run: Run {
                    run: vec!["echo hello".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        let exec = find_exec(&ops);
        assert_eq!(
            exec.meta.as_ref().expect("exec meta").cwd,
            "/app",
            "the RUN command cwd must match the workdir"
        );
    }

    #[test]
    fn test_build_multi_stage_ordering() {
        let mut builders = HashMap::new();
        builders.insert(
            "b1".to_string(),
            Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                run: Run {
                    run: vec!["echo builder1".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        builders.insert(
            "b2".to_string(),
            Stage {
                from: FromContext::FromBuilder("b1".to_string()),
                run: Run {
                    run: vec!["echo builder2".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let dofigen = Dofigen {
            builders,
            stage: Stage {
                from: FromContext::FromBuilder("b2".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        validate_output(LlbBuilder::new(dofigen, None).build());
    }

    #[test]
    fn test_build_copy_from_context() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                workdir: Some("/app".to_string()),
                copy: vec![CopyResource::Copy(Copy {
                    from: FromContext::FromContext(None),
                    paths: vec!["src/".to_string()],
                    options: CopyOptions {
                        target: Some("/app/".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        };
        validate_output(LlbBuilder::new(dofigen, None).build());
    }

    #[test]
    fn test_build_copy_from_context_without_target() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                workdir: Some("/app".to_string()),
                copy: vec![CopyResource::Copy(Copy {
                    from: FromContext::FromContext(None),
                    paths: vec!["src/".to_string()],
                    options: CopyOptions {
                        ..Default::default()
                    },
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        let dests = copy_dests(&ops);
        // With a workdir set and no explicit target, files land in the workdir.
        assert!(
            dests.contains(&"/app/".to_string()),
            "expected destination '/app/' among {:?}",
            dests
        );
    }

    #[test]
    fn test_build_copy_from_builder() {
        let mut builders = HashMap::new();
        builders.insert(
            "builder".to_string(),
            Stage {
                from: ImageName {
                    path: "rust".to_string(),
                    ..Default::default()
                }
                .into(),
                run: Run {
                    run: vec!["cargo build --release".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let dofigen = Dofigen {
            builders,
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                copy: vec![CopyResource::Copy(Copy {
                    from: FromContext::FromBuilder("builder".to_string()),
                    paths: vec!["/app/target/release/myapp".to_string()],
                    options: CopyOptions {
                        target: Some("/usr/local/bin/".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        };
        validate_output(LlbBuilder::new(dofigen, None).build());
    }

    #[test]
    fn test_build_with_default_shell() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                run: Run {
                    run: vec!["echo hello".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        let exec = find_exec(&ops);
        let args = &exec.meta.as_ref().expect("exec meta").args;
        assert_eq!(
            args,
            &vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo hello".to_string()
            ],
            "the custom SHELL must be used as the RUN executable and arguments"
        );
    }

    #[test]
    fn test_build_with_custom_shell() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                run: Run {
                    run: vec!["echo hello".to_string()],
                    shell: vec!["/bin/bash".to_string(), "-c".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        let exec = find_exec(&ops);
        let args = &exec.meta.as_ref().expect("exec meta").args;
        assert_eq!(
            args,
            &vec![
                "/bin/bash".to_string(),
                "-c".to_string(),
                "echo hello".to_string()
            ],
            "the custom SHELL must be used as the RUN executable and arguments"
        );
    }

    #[test]
    fn test_copy_with_chmod_chown() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                copy: vec![CopyResource::Copy(Copy {
                    from: FromContext::FromContext(None),
                    paths: vec!["app".to_string()],
                    options: CopyOptions {
                        target: Some("/app".to_string()),
                        chmod: Some("755".to_string()),
                        chown: Some(User {
                            user: "1000".to_string(),
                            group: Some("1000".to_string()),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        let copies = copy_actions(&ops);
        let copy = copies.first().expect("a copy action");
        assert_eq!(copy.mode, 0o755, "chmod must be translated to mode bits");
        let owner = copy.owner.as_ref().expect("an owner override");
        assert_eq!(
            owner.user.as_ref().and_then(|u| u.user.clone()),
            Some(pb::user_opt::User::ById(1000)),
            "numeric chown user must map to a user ID"
        );
    }

    #[test]
    fn test_run_with_network_and_security() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                run: Run {
                    run: vec!["echo hi".to_string()],
                    network: Some(Network::Host),
                    security: Some(Security::Insecure),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        let exec = find_exec(&ops);
        assert_eq!(exec.network, pb::NetMode::Host as i32, "RUN --network=host");
        assert_eq!(
            exec.security,
            pb::SecurityMode::Insecure as i32,
            "RUN --security=insecure"
        );
    }

    #[test]
    fn test_copy_with_link() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                copy: vec![CopyResource::Copy(Copy {
                    from: FromContext::FromContext(None),
                    paths: vec!["app".to_string()],
                    options: CopyOptions {
                        target: Some("/app".to_string()),
                        link: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        let merges = merge_ops(&ops);
        assert_eq!(merges.len(), 1, "a --link copy must emit a merge op");
        assert_eq!(
            merges[0].inputs.len(),
            2,
            "the merge combines the base state and one independent link layer"
        );
        // The copy itself is still emitted (onto the independent layer).
        assert_eq!(
            copy_actions(&ops).len(),
            1,
            "the link copy action is present"
        );
    }

    #[test]
    fn test_copy_without_link_has_no_merge() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                copy: vec![CopyResource::Copy(Copy {
                    from: FromContext::FromContext(None),
                    paths: vec!["app".to_string()],
                    options: CopyOptions {
                        target: Some("/app".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        assert!(
            merge_ops(&ops).is_empty(),
            "a regular copy must not emit a merge op"
        );
    }

    #[test]
    fn test_add_with_link() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                copy: vec![CopyResource::Add(Add {
                    files: vec![Resource::File("file.txt".into())],
                    options: CopyOptions {
                        target: Some("/app".to_string()),
                        link: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        assert_eq!(
            merge_ops(&ops).len(),
            1,
            "ADD --link must also emit a merge op"
        );
    }

    #[test]
    fn test_cache_with_chmod_chown() {
        let dofigen = Dofigen {
            stage: Stage {
                from: ImageName {
                    path: "alpine".to_string(),
                    ..Default::default()
                }
                .into(),
                run: Run {
                    run: vec!["echo hi".to_string()],
                    cache: vec![Cache {
                        target: "/var/cache/apt".to_string(),
                        chmod: Some("755".to_string()),
                        chown: Some(User {
                            user: "0".to_string(),
                            group: Some("0".to_string()),
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let ops = definition_ops(LlbBuilder::new(dofigen, None).build());
        // The cache is seeded from a directory created with the requested mode.
        let mkdirs = mkdir_actions(&ops);
        assert!(
            mkdirs.iter().any(|m| m.mode == 0o755),
            "the cache seed mkdir must carry chmod 755, got {:?}",
            mkdirs.iter().map(|m| m.mode).collect::<Vec<_>>()
        );
        // The exec has a cache mount seeded from that directory.
        let exec = find_exec(&ops);
        assert!(
            exec.mounts
                .iter()
                .any(|m| m.mount_type == pb::MountType::Cache as i32 && m.selector == "/cache"),
            "a cache mount seeded from /cache must be present"
        );
    }
}

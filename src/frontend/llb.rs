use std::{collections::HashMap, path::PathBuf, sync::Arc};

use buildkit_llb::{
    ops::{
        Command, FileSystem, MultiOwnedLastOutput, MultiOwnedOutput, OperationBuilder,
        SingleOwnedOutput, Source, source::LocalSource,
    },
    prelude::{LayerPath, Mount, OwnOutputIdx},
    utils::{OperationOutput, OutputIdx},
};
use dofigen_lib::{
    CopyResource, Dofigen, FromContext, LintMessage, LintSession, Resource, Run, Stage,
};

pub struct LlbBuilder {
    dofigen: Dofigen,
    context: Arc<LocalSource>,
    lint_session: LintSession,
}

impl LlbBuilder {
    pub fn new(dofigen: Dofigen) -> Self {
        let context = Self::init_context(&dofigen);
        let lint_session = LintSession::analyze(&dofigen);
        Self {
            dofigen,
            context,
            lint_session,
        }
    }

    pub fn get_lint_messages(&self) -> Vec<LintMessage> {
        self.lint_session.messages()
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

    pub fn build(&mut self) -> OperationOutput<'static> {
        let mut stage_outputs: HashMap<String, OperationOutput<'static>> = HashMap::new();
        let builder_names = self.lint_session.get_sorted_builders();

        for name in builder_names {
            let builder = self
                .dofigen
                .builders
                .get(&name)
                .expect(format!("The builder '{}' not found", name).as_str());
            let output = self.stage_to_llb(builder, &stage_outputs);
            stage_outputs.insert(name.to_string(), output);
        }

        self.stage_to_llb(&self.dofigen.stage, &stage_outputs)
    }

    fn stage_to_llb(
        &self,
        stage: &Stage,
        stage_outputs: &HashMap<String, OperationOutput<'static>>,
    ) -> OperationOutput<'static> {
        let base: Option<OperationOutput<'static>> = match &stage.from {
            FromContext::FromImage(image) => {
                Some(Source::image(image.to_string()).ref_counted().output())
            }
            FromContext::FromBuilder(name) => Some(stage_outputs[name].clone()),
            FromContext::FromContext(Some(ctx_name)) => {
                Some(Source::local(ctx_name.clone()).ref_counted().output())
            }
            FromContext::FromContext(None) => Some(self.context.output()),
        };

        let mut sequence = FileSystem::sequence();
        let mut next_idx: u32 = 0;
        let mut last_own_idx: Option<u32> = None;

        // WORKDIR → mkdir
        let workdir = if let Some(workdir) = &stage.workdir {
            let dest = self.dest_layer(&PathBuf::from("/"), &stage.workdir, &base, last_own_idx);
            sequence =
                sequence.append(FileSystem::mkdir(OutputIdx(next_idx), dest).make_parents(true));
            last_own_idx = sequence.last_output_index();
            next_idx += 1;
            PathBuf::from(workdir)
        } else {
            // TOOD: maybe get the workdir from the base image or builder if not specified?
            PathBuf::from("/")
        };

        // COPY resources
        for copy_resource in &stage.copy {
            match copy_resource {
                CopyResource::Copy(c) => {
                    let from_output = self.resolve_copy_from(&c.from, &base, stage_outputs);
                    for path in &c.paths {
                        let dest =
                            self.dest_layer(&workdir, &c.options.target, &base, last_own_idx);
                        sequence = sequence.append(
                            FileSystem::copy()
                                .from(LayerPath::Other(from_output.clone(), path.as_str()))
                                .to(OutputIdx(next_idx), dest)
                                .create_path(true)
                                .recursive(true),
                        );
                        last_own_idx = sequence.last_output_index();
                        next_idx += 1;
                    }
                }
                CopyResource::Content(cc) => {
                    let dest = self.dest_layer(&workdir, &cc.options.target, &base, last_own_idx);
                    sequence = sequence.append(
                        FileSystem::mkfile(OutputIdx(next_idx), dest)
                            .data(cc.content.as_bytes().to_vec()),
                    );
                    last_own_idx = sequence.last_output_index();
                    next_idx += 1;
                }
                CopyResource::AddGitRepo(ag) => {
                    let git_output = Source::git(ag.repo.clone()).ref_counted().output();
                    let dest = self.dest_layer(&workdir, &ag.options.target, &base, last_own_idx);
                    sequence = sequence.append(
                        FileSystem::copy()
                            .from(LayerPath::Other(git_output, "."))
                            .to(OutputIdx(next_idx), dest)
                            .create_path(true)
                            .recursive(true),
                    );
                    last_own_idx = sequence.last_output_index();
                    next_idx += 1;
                }
                CopyResource::Add(a) => {
                    for file in &a.files {
                        let file_output = match file {
                            Resource::Url(url) => Source::http(url.as_str()).ref_counted().output(),
                            Resource::File(_path) => {
                                // Local file: reference from the build context
                                self.context.output()
                            }
                        };
                        let src_path = match file {
                            Resource::Url(_) => ".".to_string(),
                            Resource::File(path) => path.to_string_lossy().into_owned(),
                        };
                        let dest =
                            self.dest_layer(&workdir, &a.options.target, &base, last_own_idx);
                        sequence = sequence.append(
                            FileSystem::copy()
                                .from(LayerPath::Other(file_output, src_path.as_str()))
                                .to(OutputIdx(next_idx), dest)
                                .create_path(true),
                        );
                        last_own_idx = sequence.last_output_index();
                        next_idx += 1;
                    }
                }
            }
        }

        let mut current: OperationOutput<'static> = if last_own_idx.is_some() {
            sequence.ref_counted().last_output().unwrap()
        } else {
            base.clone().unwrap_or_else(|| self.context.output())
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
                );
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
            );
        }

        current
    }

    fn apply_run(
        &self,
        base: OperationOutput<'static>,
        run: &Run,
        user: Option<&str>,
        env: &HashMap<String, String>,
        workdir: &Option<String>,
        stage_outputs: &HashMap<String, OperationOutput<'static>>,
    ) -> OperationOutput<'static> {
        let script = run.run.join("\n");
        let mut scratch_idx: u32 = 1;

        let mut cmd = Command::run("/bin/sh")
            .args(["-c", script.as_str()])
            .env_iter(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .mount(Mount::Layer(OutputIdx(0), base, "/"));

        if let Some(wd) = workdir {
            cmd = cmd.cwd(wd.as_str());
        }

        if let Some(u) = user {
            cmd = cmd.user(u);
        }

        for cache in &run.cache {
            cmd = cmd.mount(Mount::SharedCache(cache.target.as_str()));
        }

        for ssh in &run.ssh {
            let path = ssh.target.as_deref().unwrap_or("/run/buildkit/ssh_agent.0");
            cmd = cmd.mount(Mount::OptionalSshAgent(path));
        }

        for bind in &run.bind {
            let from_output = self.resolve_copy_from(&bind.from, &None, stage_outputs);
            match &bind.source {
                Some(src) => {
                    cmd = cmd.mount(Mount::ReadOnlySelector(
                        from_output,
                        bind.target.as_str(),
                        src.as_str(),
                    ));
                }
                None => {
                    cmd = cmd.mount(Mount::ReadOnlyLayer(from_output, bind.target.as_str()));
                }
            }
        }

        for tmpfs in &run.tmpfs {
            cmd = cmd.mount(Mount::Scratch(
                OutputIdx(scratch_idx),
                tmpfs.target.as_str(),
            ));
            scratch_idx += 1;
        }

        cmd.ref_counted().output(0)
    }

    fn resolve_copy_from(
        &self,
        from: &FromContext,
        base: &Option<OperationOutput<'static>>,
        stage_outputs: &HashMap<String, OperationOutput<'static>>,
    ) -> OperationOutput<'static> {
        match from {
            FromContext::FromImage(image) => {
                Source::image(image.to_string()).ref_counted().output()
            }
            FromContext::FromBuilder(name) => stage_outputs[name].clone(),
            FromContext::FromContext(Some(ctx_name)) => {
                Source::local(ctx_name.clone()).ref_counted().output()
            }
            FromContext::FromContext(None) => base.clone().unwrap_or_else(|| self.context.output()),
        }
    }

    fn dest_layer(
        &self,
        workdir: &PathBuf,
        path: &Option<String>,
        base: &Option<OperationOutput<'static>>,
        last_own_idx: Option<u32>,
    ) -> LayerPath<'static, PathBuf> {
        let path = path.as_deref().unwrap_or("");
        let path = if workdir.as_os_str() == "/" {
            PathBuf::from(path)
        } else {
            workdir.join(path)
        };
        match last_own_idx {
            Some(i) => LayerPath::Own(OwnOutputIdx(i), path),
            None => match base {
                Some(b) => LayerPath::Other(b.clone(), path),
                None => LayerPath::Scratch(path),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildkit_llb::ops::Terminal;
    use dofigen_lib::{Copy, CopyOptions, Dofigen, FromContext, ImageName, Run, Stage};

    fn validate_output(output: OperationOutput<'static>) {
        Terminal::with(output).into_definition();
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
        validate_output(LlbBuilder::new(dofigen).build());
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
        validate_output(LlbBuilder::new(dofigen).build());
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
        todo!("Test the run cwd is the workdir");
        validate_output(LlbBuilder::new(dofigen).build());
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
        validate_output(LlbBuilder::new(dofigen).build());
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
        validate_output(LlbBuilder::new(dofigen).build());
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
        todo!("Test the dest path is correct with workdir and no target");
        validate_output(LlbBuilder::new(dofigen).build());
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
        validate_output(LlbBuilder::new(dofigen).build());
    }
}

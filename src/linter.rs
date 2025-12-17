use std::collections::{HashMap, HashSet};

use crate::{context, dofigen_struct::*};

#[derive(Debug, Clone, PartialEq)]
struct StageDependency {
    stage: String,
    path: String,
    origin: Vec<String>,
}

macro_rules! linter_path {
    ($session:expr_2021, $part:expr_2021, $block:block) => {
        $session.push_path_part($part);
        $block
        $session.pop_path_part();
    };
}

trait Linter {
    fn analyze(&self, session: &mut LintSession);
}

impl Linter for Dofigen {
    fn analyze(&self, session: &mut LintSession) {
        linter_path!(session, "builders".into(), {
            for (name, builder) in self.builders.iter() {
                linter_path!(session, name.clone(), {
                    if name == "runtime" {
                        session.add_message(
                            MessageLevel::Error,
                            "The builder name 'runtime' is reserved".into(),
                        );
                    }
                    builder.analyze(session);
                });
            }
        });

        self.stage.analyze(session);

        // Check root user in runtime stage
        if let Some(user) = &self.stage.user {
            if user.user == "root" || user.uid() == Some(0) {
                session.messages.push(LintMessage {
                    level: MessageLevel::Warn,
                    message: "The runtime user should not be root".into(),
                    path: vec!["user".into()],
                });
            }
        }

        session.check_dependencies();
    }
}

impl Linter for Stage {
    fn analyze(&self, session: &mut LintSession) {
        let name = session.current_path.last().cloned();

        // Check empty stage
        if let Some(name) = name.clone() {
            if self.copy.is_empty() && self.run.run.is_empty() && self.root.is_none() {
                session.add_message(
                    MessageLevel::Warn,
                    format!("The builder '{}' is empty and should be removed", name),
                );
            }
        }

        let name = name.unwrap_or("runtime".to_string());

        let dependencies = self.get_dependencies(&session.current_path);
        session.messages.append(
            &mut dependencies
                .iter()
                .filter(|dep| dep.stage == "runtime")
                .map(|dep| LintMessage {
                    level: MessageLevel::Error,
                    message: format!("The stage '{}' can't depend on the 'runtime'", &name,),
                    path: dep.origin.clone(),
                })
                .collect(),
        );
        let cache_paths = session.get_stage_cache_paths(self);
        session.stage_infos.insert(
            name,
            StageLintInfo {
                dependencies,
                cache_paths,
            },
        );

        // Check the use of fromContext
        self.from.analyze(session);

        linter_path!(session, "copy".into(), {
            for (position, copy) in self.copy.iter().enumerate() {
                linter_path!(session, position.to_string(), {
                    copy.analyze(session);
                });
            }
        });

        if let Some(root) = &self.root {
            linter_path!(session, "root".into(), {
                root.analyze(session);
            });
        }

        self.run.analyze(session);

        // Check if the user is using the username instead of the UID
        if let Some(user) = &self.user {
            if user.uid().is_none() {
                linter_path!(session, "user".into(), {
                    session.add_message(
                        MessageLevel::Warn,
                        "UID should be used instead of username".to_string(),
                    );
                });
            }
        }
    }
}

impl Linter for CopyResource {
    fn analyze(&self, session: &mut LintSession) {
        match self {
            CopyResource::Copy(copy) => copy.analyze(session),
            _ => {}
        }
    }
}

impl Linter for Copy {
    fn analyze(&self, session: &mut LintSession) {
        self.from.analyze(session);
    }
}

impl Linter for Run {
    fn analyze(&self, session: &mut LintSession) {
        if self.run.is_empty() {
            if !self.bind.is_empty() {
                linter_path!(session, "bind".into(), {
                    session.add_message(
                        MessageLevel::Warn,
                        "The run list is empty but there are bind definitions".to_string(),
                    );
                });
            }

            if !self.cache.is_empty() {
                linter_path!(session, "cache".into(), {
                    session.add_message(
                        MessageLevel::Warn,
                        "The run list is empty but there are cache definitions".to_string(),
                    );
                });
            }
        }

        linter_path!(session, "run".into(), {
            for (position, command) in self.run.iter().enumerate() {
                linter_path!(session, position.to_string(), {
                    if command.starts_with("cd ") {
                        session.add_message(
                            MessageLevel::Warn,
                            "Avoid using 'cd' in the run command".to_string(),
                        );
                    }
                });
            }
        });

        linter_path!(session, "bind".into(), {
            for (position, bind) in self.bind.iter().enumerate() {
                linter_path!(session, position.to_string(), {
                    bind.from.analyze(session);
                });
            }
        });

        linter_path!(session, "cache".into(), {
            for (position, cache) in self.cache.iter().enumerate() {
                linter_path!(session, position.to_string(), {
                    cache.from.analyze(session);
                });
            }
        });
    }
}

impl Linter for FromContext {
    fn analyze(&self, session: &mut LintSession) {
        match self {
            FromContext::FromImage(image) => {
                linter_path!(session, "fromImage".into(), {
                    image.analyze(session);
                });
            }
            FromContext::FromBuilder(builder) => {
                linter_path!(session, "fromBuilder".into(), {
                    check_from_arg_placeholders(session, builder);
                });
            }
            FromContext::FromContext(Some(context)) => {
                if contains_arg_placeholder(context) {
                    return;
                }
                let mut message =
                    "Prefer to use fromImage and fromBuilder instead of fromContext".to_string();
                // Check if it's main stage `FROM` or builder stage `FROM` (builders/<name>/from)
                let is_stage_from = session.current_path.is_empty()
                    || session.current_path[0] == "builders" && session.current_path.len() == 2;
                if !is_stage_from {
                    message.push_str(" (unless it's really from a build context: https://docs.docker.com/reference/cli/docker/buildx/build/#build-context)");
                }
                linter_path!(session, "fromContext".into(), {
                    session.add_message(MessageLevel::Warn, message);
                });
            }
            _ => {}
        }
    }
}

impl Linter for ImageName {
    fn analyze(&self, session: &mut LintSession) {
        if let Some(host) = &self.host {
            linter_path!(session, "host".into(), {
                check_from_arg_placeholders(session, host);
            });
        }
        linter_path!(session, "path".into(), {
            check_from_arg_placeholders(session, &self.path);
        });
        if let Some(version) = &self.version {
            match version {
                ImageVersion::Tag(tag) => {
                    linter_path!(session, "tag".into(), {
                        check_from_arg_placeholders(session, tag);
                    });
                }
                ImageVersion::Digest(digest) => {
                    linter_path!(session, "digest".into(), {
                        check_from_arg_placeholders(session, digest);
                    });
                }
            }
        }
    }
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
    current_path: Vec<String>,
    messages: Vec<LintMessage>,
    stage_infos: HashMap<String, StageLintInfo>,
    recursive_stage_dependencies: HashMap<String, Vec<String>>,
}

impl LintSession {
    fn push_path_part(&mut self, part: String) {
        self.current_path.push(part);
    }

    fn pop_path_part(&mut self) {
        self.current_path.pop();
    }

    fn add_message(&mut self, level: MessageLevel, message: String) {
        self.messages.push(LintMessage {
            level,
            message,
            path: self.current_path.clone(),
        });
    }

    pub fn messages(&self) -> Vec<LintMessage> {
        self.messages.clone()
    }

    pub fn get_sorted_builders(&mut self) -> Vec<String> {
        let mut stages: HashMap<String, Vec<String>> = self
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

        let mut sorted: Vec<String> = vec![];

        loop {
            let mut part: Vec<String> = stages
                .extract_if(|_name, deps| deps.is_empty())
                .map(|(name, _deps)| name)
                .collect();

            if part.is_empty() {
                // TODO: log circular dependency
                break;
            }

            part.sort();

            for name in part.iter() {
                for deps in stages.values_mut() {
                    deps.retain(|dep| dep != name);
                }
            }

            sorted.append(&mut part);

            if stages.is_empty() {
                break;
            }
        }

        sorted
            .into_iter()
            .filter(|name| name != "runtime")
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
            .values()
            .flat_map(|info| info.dependencies.clone())
            .collect::<Vec<_>>();

        let caches = self
            .stage_infos
            .iter()
            .map(|(name, info)| (name.clone(), info.cache_paths.clone()))
            .collect::<HashMap<_, _>>();

        // Check if there is unused builders
        let used_builders = dependencies
            .iter()
            .map(|dep| dep.stage.clone())
            .collect::<HashSet<_>>();

        let unused_builders = self
            .stage_infos
            .keys()
            .filter(|name| name != &"runtime")
            .map(|name| name.clone())
            .filter(|name| !used_builders.contains(name))
            .collect::<HashSet<_>>();

        linter_path!(self, "builders".into(), {
            for builder in unused_builders {
                linter_path!(self, builder.clone(), {
                    self.add_message(
                        MessageLevel::Warn,
                        format!(
                            "The builder '{}' is not used and should be removed",
                            builder
                        ),
                    );
                });
            }
        });

        for dependency in dependencies {
            if let Some(paths) = caches.get(&dependency.stage) {
                paths
                    .iter()
                    .filter(|path| dependency.path.starts_with(*path))
                    .for_each(|path| {
                        self.messages.push(LintMessage {
                            level: MessageLevel::Error,
                            message: format!(
                                "Use of the '{}' builder cache path '{}'",
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

    fn get_stage_cache_paths(&mut self, stage: &Stage) -> Vec<String> {
        let mut paths = vec![];
        paths.append(&mut self.get_run_cache_paths(
            &stage.run,
            &self.current_path.clone(),
            &stage.workdir,
        ));
        if let Some(root) = &stage.root {
            paths.append(&mut self.get_run_cache_paths(
                root,
                &[self.current_path.clone(), vec!["root".into()]].concat(),
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
        dofigen.analyze(&mut session);

        session
    }
}

pub fn check_from_arg_placeholders(session: &mut LintSession, value: &str) {
    if contains_arg_placeholder(value) {
        session.add_message(
            MessageLevel::Error,
            "Use fromContext when using global arg.".to_string(),
        );
    }
}

pub fn contains_arg_placeholder(value: &str) -> bool {
    value.contains("$")
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
    use crate::Dofigen;

    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod stage_dependencies {
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
                stage: Stage {
                    copy: vec![CopyResource::Copy(Copy {
                        from: FromContext::FromBuilder("builder1".into()),
                        paths: vec!["/path/to/copy".into()],
                        ..Default::default()
                    })],
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut dependencies = lint_session.get_stage_recursive_dependencies("runtime".into());
            dependencies.sort();
            assert_eq_sorted!(dependencies, vec!["builder1", "builder2", "builder3"]);

            dependencies = lint_session.get_stage_recursive_dependencies("builder1".into());
            dependencies.sort();
            assert_eq_sorted!(dependencies, vec!["builder2", "builder3"]);

            dependencies = lint_session.get_stage_recursive_dependencies("builder2".into());
            assert_eq_sorted!(dependencies, vec!["builder3"]);

            dependencies = lint_session.get_stage_recursive_dependencies("builder3".into());
            assert_eq_sorted!(dependencies, Vec::<String>::new());

            let builders = lint_session.get_sorted_builders();

            assert_eq_sorted!(builders, vec!["builder3", "builder2", "builder1",]);

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

            let builders = lint_session.get_sorted_builders();

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
                vec![
                    LintMessage {
                        level: MessageLevel::Error,
                        path: vec![
                            "builders".into(),
                            "builder".into(),
                            "copy".into(),
                            "0".into()
                        ],
                        message: "The stage 'builder' can't depend on the 'runtime'".into(),
                    },
                    LintMessage {
                        level: MessageLevel::Warn,
                        path: vec!["builders".into(), "builder".into(),],
                        message: "The builder 'builder' is not used and should be removed".into(),
                    }
                ]
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
                stage: Stage {
                    from: FromContext::FromBuilder("builder2".into()),
                    ..Default::default()
                },
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
                    message: "Use of the 'builder1' builder cache path '/path/to/cache'".into(),
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
                            run: Run {
                                run: vec!["echo coucou".to_string()],
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ),
                    (
                        "install-php-ext".to_string(),
                        Stage {
                            from: FromContext::FromBuilder("install-deps".to_string()),
                            run: Run {
                                run: vec!["echo coucou".to_string()],
                                ..Default::default()
                            },
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
                            run: Run {
                                run: vec!["echo coucou".to_string()],
                                ..Default::default()
                            },
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

    mod builder {
        use super::*;

        #[test]
        fn empty() {
            let dofigen = Dofigen {
                builders: HashMap::from([(
                    "builder".into(),
                    Stage {
                        from: FromContext::FromImage(ImageName {
                            path: "php".into(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                )]),
                stage: Stage {
                    from: FromContext::FromBuilder("builder".into()),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Warn,
                    path: vec!["builders".into(), "builder".into()],
                    message: "The builder 'builder' is empty and should be removed".into(),
                },]
            );
        }

        #[test]
        fn unused() {
            let dofigen = Dofigen {
                builders: HashMap::from([(
                    "builder".into(),
                    Stage {
                        from: FromContext::FromImage(ImageName {
                            ..Default::default()
                        }),
                        run: Run {
                            run: vec!["echo Hello".into()],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Warn,
                    path: vec!["builders".into(), "builder".into()],
                    message: "The builder 'builder' is not used and should be removed".into(),
                },]
            );
        }
    }

    mod user {
        use super::*;

        #[test]
        fn uid() {
            let dofigen = Dofigen {
                stage: Stage {
                    user: Some(User::new("1000")),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(lint_session.messages, vec![]);
        }

        #[test]
        fn username() {
            let dofigen = Dofigen {
                stage: Stage {
                    user: Some(User::new("test")),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Warn,
                    path: vec!["user".into()],
                    message: "UID should be used instead of username".into(),
                },]
            );
        }
    }

    mod from_context {
        use super::*;

        #[test]
        fn stage_and_copy() {
            let dofigen = Dofigen {
                stage: Stage {
                    from: FromContext::FromContext(Some("php:8.3-fpm-alpine".into())),
                    copy: vec![CopyResource::Copy(Copy {
                        from: FromContext::FromContext(Some("composer:latest".into())),
                        paths: vec!["/usr/bin/composer".into()],
                        ..Default::default()
                    })],
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(lint_session.messages, vec![
                LintMessage {
                    level: MessageLevel::Warn,
                    path: vec!["fromContext".into()],
                    message: "Prefer to use fromImage and fromBuilder instead of fromContext".into(),   
                },
                LintMessage {
                    level: MessageLevel::Warn,
                    path: vec!["copy".into(), "0".into(), "fromContext".into()],
                    message: "Prefer to use fromImage and fromBuilder instead of fromContext (unless it's really from a build context: https://docs.docker.com/reference/cli/docker/buildx/build/#build-context)".into(),
                }
            ]);
        }

        #[test]
        fn root_bind() {
            let dofigen = Dofigen {
                builders: HashMap::from([(
                    "builder".into(),
                    Stage {
                        root: Some(Run {
                            bind: vec![Bind {
                                from: FromContext::FromContext(Some("builder".into())),
                                source: Some("/path/to/bind".into()),
                                target: "/path/to/target".into(),
                                ..Default::default()
                            }],
                            run: vec!["echo Hello".into()],
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                )]),
                stage: Stage {
                    from: FromContext::FromBuilder("builder".into()),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(lint_session.messages, vec![
                LintMessage {
                    level: MessageLevel::Warn,
                    path: vec![
                        "builders".into(),
                        "builder".into(),
                        "root".into(),
                        "bind".into(),
                        "0".into(),
                        "fromContext".into(),
                    ],
                    message: "Prefer to use fromImage and fromBuilder instead of fromContext (unless it's really from a build context: https://docs.docker.com/reference/cli/docker/buildx/build/#build-context)".into(),
                }
            ]);
        }

        #[test]
        fn with_global_arg() {
            let dofigen = Dofigen {
                global_arg: HashMap::from([("VERSION".into(), "21".into())]),
                stage: Stage {
                    from: FromContext::FromContext(Some("eclipse-temurin:${VERSION}".into())),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(lint_session.messages, vec![]);
        }

        #[test]
        fn from_image_with_global_arg() {
            let dofigen = Dofigen {
                global_arg: HashMap::from([("VERSION".into(), "21".into())]),
                stage: Stage {
                    from: FromContext::FromImage(ImageName {
                        path: "eclipse-temurin".into(),
                        version: Some(ImageVersion::Tag("${VERSION}".into())),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(
                lint_session.messages,
                vec![LintMessage {
                    level: MessageLevel::Error,
                    path: vec!["fromImage".into(), "tag".into(),],
                    message: "Use fromContext when using global arg.".into(),
                }]
            );
        }
    }

    mod run {
        use super::*;

        #[test]
        fn empty_run() {
            let dofigen = Dofigen {
                stage: Stage {
                    run: Run {
                        bind: vec![Bind {
                            source: Some("/path/to/bind".into()),
                            target: "/path/to/target".into(),
                            ..Default::default()
                        }],
                        cache: vec![Cache {
                            source: Some("/path/to/cache".into()),
                            target: "/path/to/target".into(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            };

            let lint_session = LintSession::analyze(&dofigen);

            assert_eq_sorted!(
                lint_session.messages,
                vec![
                    LintMessage {
                        level: MessageLevel::Warn,
                        message: "The run list is empty but there are bind definitions".into(),
                        path: vec!["bind".into()],
                    },
                    LintMessage {
                        level: MessageLevel::Warn,
                        message: "The run list is empty but there are cache definitions".into(),
                        path: vec!["cache".into()],
                    },
                ]
            );
        }
    }
}

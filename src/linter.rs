use std::collections::{HashMap, HashSet};

use crate::dofigen_struct::*;

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
    pub fn messages(&self) -> Vec<LintMessage> {
        self.messages.clone()
    }

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

        // Check if the user is using the username instead of the UID
        if let Some(user) = &stage.user {
            if user.uid().is_none() {
                self.messages.push(LintMessage {
                    level: MessageLevel::Warn,
                    message: "UID should be used instead of username".to_string(),
                    path: [path.clone(), vec!["user".into()]].concat(),
                });
            }
        }
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

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, Vec::<String>::new());

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

            let mut lint_session = LintSession::analyze(&dofigen);

            let mut builders = lint_session.get_sorted_builders();
            builders.sort();

            assert_eq_sorted!(builders, Vec::<String>::new());

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
}

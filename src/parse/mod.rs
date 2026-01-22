mod context;
mod insctruction;

use colored::{Color, Colorize};
use struct_patch::Patch;

use crate::{
    DockerFile, DockerFileLine, DockerIgnore, DockerIgnoreLine, Dofigen, Error, FromContext,
    MessageLevel, Result, parse::context::ParseContext,
};

impl Dofigen {
    pub fn from_dockerfile(
        dockerfile: DockerFile,
        dockerignore: Option<DockerIgnore>,
    ) -> Result<Self> {
        let mut context = ParseContext::default();

        if let Some(dockerignore) = dockerignore {
            // TODO: If there is a negate pattern with **, then manage context field
            let ignores: Vec<String> = dockerignore
                .lines
                .iter()
                .filter(|line| {
                    matches!(line, DockerIgnoreLine::Pattern(_))
                        || matches!(line, DockerIgnoreLine::NegatePattern(_))
                })
                .map(|line| match line {
                    DockerIgnoreLine::Pattern(pattern) => pattern.clone(),
                    DockerIgnoreLine::NegatePattern(pattern) => format!("!{pattern}"),
                    _ => unreachable!(),
                })
                .collect();
            context.dofigen.ignore = ignores;
        }

        let instructions: Vec<_> = dockerfile
            .lines
            .iter()
            .filter(|line| matches!(line, DockerFileLine::Instruction(_)))
            .collect();

        for line in instructions {
            context.apply(line)?;
        }

        // Get runtime informations
        let runtime_stage = context.current_stage(None)?.clone();
        let runtime_name = context.current_stage_name(None)?;

        // Get base instructions in from builders
        let mut dofigen_patches = context
            .builder_dofigen_patches
            .remove(&runtime_name)
            .into_iter()
            .collect::<Vec<_>>();
        let mut searching_stage = runtime_stage.clone();
        while let FromContext::FromBuilder(builder_name) = searching_stage.from.clone() {
            if let Some(builder_dofigen_patch) =
                context.builder_dofigen_patches.remove(&builder_name)
            {
                dofigen_patches.insert(0, builder_dofigen_patch);
            }
            searching_stage = context
                .builders
                .get(&builder_name)
                .ok_or(Error::Custom(format!(
                    "Builder '{}' not found",
                    builder_name
                )))?
                .clone();
        }

        // Apply merged patches
        if !dofigen_patches.is_empty() {
            dofigen_patches.iter().for_each(|dofigen_patch| {
                context.dofigen.apply(dofigen_patch.clone());
            });
        }

        context.apply_root()?;
        context.dofigen.stage = runtime_stage;

        // Set builders
        if !context.builders.is_empty() {
            context.dofigen.builders = context.builders;
        }

        // Handle lint messages
        context.messages.iter().for_each(|message| {
            eprintln!(
                "{}[path={}]: {}",
                match message.level {
                    MessageLevel::Error => "error".color(Color::Red).bold(),
                    MessageLevel::Warn => "warning".color(Color::Yellow).bold(),
                },
                message.path.join(".").color(Color::Blue).bold(),
                message.message
            );
        });

        Ok(context.dofigen.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Copy, CopyOptions, DockerFileCommand, DockerFileInsctruction, FromContext, ImageName, Run,
        Stage, User,
    };
    use pretty_assertions_sorted::assert_eq_sorted;
    use std::collections::HashMap;

    mod dockerignore {

        use super::*;

        #[test]
        fn simple() {
            let dockerignore = DockerIgnore {
                lines: vec![
                    DockerIgnoreLine::Pattern("*.tmp".to_string()),
                    DockerIgnoreLine::Pattern("/test/".to_string()),
                ],
            };

            let result = Dofigen::from_dockerfile(
                DockerFile {
                    lines: vec![DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    })],
                },
                Some(dockerignore),
            );

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 2);
            assert_eq_sorted!(dofigen.ignore, vec!["*.tmp", "/test/"]);
        }

        #[test]
        fn with_negate_patterns() {
            let dockerignore = DockerIgnore {
                lines: vec![
                    DockerIgnoreLine::Pattern("*.tmp".to_string()),
                    DockerIgnoreLine::NegatePattern("test.tmp".to_string()),
                    DockerIgnoreLine::Pattern("/test/".to_string()),
                    DockerIgnoreLine::NegatePattern("/test/lib.ts".to_string()),
                ],
            };

            let result = Dofigen::from_dockerfile(
                DockerFile {
                    lines: vec![DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    })],
                },
                Some(dockerignore),
            );

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 4);
            assert_eq_sorted!(
                dofigen.ignore,
                vec!["*.tmp", "!test.tmp", "/test/", "!/test/lib.ts"]
            );
        }
    }

    mod from {

        use super::*;

        #[test]
        fn image_ubuntu() {
            let dockerfile = DockerFile {
                lines: vec![DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::FROM,
                    content: "ubuntu:25.04".to_string(),
                    options: vec![],
                })],
            };

            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.from,
                FromContext::FromImage(ImageName {
                    path: "ubuntu".to_string(),
                    version: Some(crate::ImageVersion::Tag("25.04".to_string())),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn build_stage_and_main_stage_from_scratch() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as builder".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "scratch AS runtime".to_string(),
                        options: vec![],
                    }),
                ],
            };

            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 1);
            assert!(dofigen.builders.contains_key("builder"));
            assert_eq_sorted!(
                dofigen.builders["builder"].from,
                FromContext::FromImage(ImageName {
                    path: "ubuntu".to_string(),
                    version: Some(crate::ImageVersion::Tag("25.04".to_string())),
                    ..Default::default()
                })
            );
            assert_eq_sorted!(dofigen.stage.from, FromContext::FromContext(None));
        }

        #[test]
        fn from_builder() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as test".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "test".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.builders,
                HashMap::from([(
                    "test".to_string(),
                    Stage {
                        from: FromContext::FromImage(ImageName {
                            path: "ubuntu".to_string(),
                            version: Some(crate::ImageVersion::Tag("25.04".to_string())),
                            ..Default::default()
                        })
                        .into(),
                        ..Default::default()
                    }
                )])
            );

            assert_eq_sorted!(
                dofigen.stage.from,
                FromContext::FromBuilder("test".to_string())
            );
        }

        #[test]
        fn without_from() {
            let dockerfile = DockerFile { lines: vec![] };

            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            assert!(result.is_err());
            let error = result.unwrap_err();

            assert_eq_sorted!(error.to_string(), "No FROM instruction found");
        }
    }

    mod arg {
        use super::*;

        #[test]
        fn simple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ARG,
                        content: "FOO=bar".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.arg,
                HashMap::from([("FOO".to_string(), "bar".to_string())])
            );
        }

        #[test]
        fn multiline() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ARG,
                        content: "FOO=bar baz \\\n\t test=OK".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.arg,
                HashMap::from([
                    ("FOO".to_string(), "bar".to_string()),
                    ("baz".to_string(), String::new()),
                    ("test".to_string(), "OK".to_string())
                ])
            );
        }

        #[test]
        fn with_space() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ARG,
                        content: "FOO=\"bar baz\"".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.arg,
                HashMap::from([("FOO".to_string(), "bar baz".to_string())])
            );
        }
    }

    mod label {
        use super::*;

        #[test]

        fn all_formats() {
            // See: https://docs.docker.com/reference/dockerfile/#label
            // LABEL "com.example.vendor"="ACME Incorporated"
            // LABEL com.example.label-with-value="foo"
            // LABEL version="1.0"
            // LABEL description="This text illustrates \
            // that label-values can span multiple lines."

            let dockerfile = DockerFile {
						lines: vec![
							DockerFileLine::Instruction(DockerFileInsctruction {
								command: DockerFileCommand::FROM,
								content: "ubuntu:25.04".to_string(),
								options: vec![],
							}),
							DockerFileLine::Instruction(DockerFileInsctruction {
								command: DockerFileCommand::LABEL,
								content: "\"com.example.vendor\"=\"ACME Incorporated\"".to_string(),
								options: vec![],
							}),
							DockerFileLine::Instruction(DockerFileInsctruction {
								command: DockerFileCommand::LABEL,
								content: "com.example.label-with-value=\"foo\"".to_string(),
								options: vec![],
							}),
							DockerFileLine::Instruction(DockerFileInsctruction {
								command: DockerFileCommand::LABEL,
								content: "version=\"1.0\"".to_string(),
								options: vec![],
							}),
							DockerFileLine::Instruction(DockerFileInsctruction {
								command: DockerFileCommand::LABEL,
								content: "description=\"This text illustrates \\\nthat label-values can span multiple lines.\"".to_string(),
								options: vec![],
							}),
						],
					};

            let result = Dofigen::from_dockerfile(dockerfile, None);

            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.label,
                HashMap::from([
                    (
                        "com.example.vendor".to_string(),
                        "ACME Incorporated".to_string()
                    ),
                    (
                        "com.example.label-with-value".to_string(),
                        "foo".to_string()
                    ),
                    ("version".to_string(), "1.0".to_string()),
                    (
                        "description".to_string(),
                        "This text illustrates \nthat label-values can span multiple lines."
                            .to_string()
                    )
                ])
            );
        }

        #[test]
        fn from_maintainer() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::MAINTAINER,
                        content: "taorepoara".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let result = Dofigen::from_dockerfile(dockerfile, None);
            let dofigen = result.unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.label,
                HashMap::from([(
                    "org.opencontainers.image.authors".to_string(),
                    "taorepoara".to_string()
                )])
            );
        }
    }

    mod expose {
        use crate::Port;

        use super::*;

        #[test]
        fn simple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::EXPOSE,
                        content: "80".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.expose,
                vec![Port {
                    port: 80,
                    protocol: None,
                }]
            );
        }

        #[test]
        fn multiple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::EXPOSE,
                        content: "80".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::EXPOSE,
                        content: "443".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.expose,
                vec![
                    Port {
                        port: 80,
                        protocol: None,
                    },
                    Port {
                        port: 443,
                        protocol: None,
                    }
                ]
            );
        }

        #[test]
        fn from_builder() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as test".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::EXPOSE,
                        content: "80".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "test".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.expose,
                vec![Port {
                    port: 80,
                    protocol: None,
                }]
            );
        }
    }

    mod volume {
        use super::*;

        #[test]
        fn simple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::VOLUME,
                        content: "/data".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.volume, vec!["/data".to_string()]);
        }

        #[test]
        fn multiple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::VOLUME,
                        content: "/data".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::VOLUME,
                        content: "/app".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.volume,
                vec!["/data".to_string(), "/app".to_string()]
            );
        }

        #[test]
        fn json_array() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::VOLUME,
                        content: r#"["/data", "/app"]"#.to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.volume,
                vec!["/data".to_string(), "/app".to_string()]
            );
        }

        #[test]
        fn from_builder() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as test".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::VOLUME,
                        content: "/data".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "test".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.volume, vec!["/data".to_string()]);
        }
    }

    mod shell {
        use super::*;

        #[test]
        fn simple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::SHELL,
                        content: r#"["/bin/sh", "-c"]"#.to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::RUN,
                        content: "echo coucou".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.stage.run,
                Run {
                    shell: vec!["/bin/sh".to_string(), "-c".to_string()],
                    run: vec!["echo coucou".to_string()],
                    ..Default::default()
                }
            );
        }
    }

    mod healthcheck {
        use crate::Healthcheck;

        use super::*;

        #[test]
        fn simple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::HEALTHCHECK,
                        content: "/check.sh".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.healthcheck,
                Some(Healthcheck {
                    cmd: "/check.sh".to_string(),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn multiple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::HEALTHCHECK,
                        content: "/check.sh".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::HEALTHCHECK,
                        content: "/new_check.sh".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.healthcheck,
                Some(Healthcheck {
                    cmd: "/new_check.sh".to_string(),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn json_array() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::HEALTHCHECK,
                        content: r#"["/check.sh", "--test"]"#.to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.healthcheck,
                Some(Healthcheck {
                    cmd: "/check.sh --test".to_string(),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn from_builder() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as test".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::HEALTHCHECK,
                        content: "/check.sh".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "test".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.healthcheck,
                Some(Healthcheck {
                    cmd: "/check.sh".to_string(),
                    ..Default::default()
                })
            );
        }
    }

    mod cmd {
        use super::*;

        #[test]
        fn simple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::CMD,
                        content: "--help".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.cmd, vec!["--help".to_string()]);
        }

        #[test]
        fn multiple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::CMD,
                        content: "--help".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::CMD,
                        content: "gen".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.cmd, vec!["gen".to_string()]);
        }

        #[test]
        fn json_array() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::CMD,
                        content: r#"["gen", "--help"]"#.to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.cmd, vec!["gen".to_string(), "--help".to_string()]);
        }

        #[test]
        fn from_builder() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as test".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::CMD,
                        content: "--help".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "test".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.cmd, vec!["--help".to_string()]);
        }
    }

    mod entrypoint {
        use super::*;

        #[test]
        fn simple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ENTRYPOINT,
                        content: "/entrypoint.sh".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.entrypoint, vec!["/entrypoint.sh".to_string()]);
        }

        #[test]
        fn multiple() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ENTRYPOINT,
                        content: "/entrypoint.sh".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ENTRYPOINT,
                        content: "/new_entrypoint.sh".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.entrypoint, vec!["/new_entrypoint.sh".to_string()]);
        }

        #[test]
        fn json_array() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ENTRYPOINT,
                        content: r#"["/entrypoint.sh", "-c"]"#.to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(
                dofigen.entrypoint,
                vec!["/entrypoint.sh".to_string(), "-c".to_string()]
            );
        }

        #[test]
        fn from_builder() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as test".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ENTRYPOINT,
                        content: "/entrypoint.sh".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "test".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            let dofigen = result.unwrap();

            assert_eq_sorted!(dofigen.entrypoint, vec!["/entrypoint.sh".to_string()]);
        }
    }

    mod from_string {

        use crate::{AddGitRepo, CopyResource, DofigenContext, GenerationContext, ImageVersion};

        use super::*;

        #[test]
        // #[ignore = "Not managed yet by serde because of multilevel flatten: https://serde.rs/field-attrs.html#flatten"]
        fn php_dockerfile() {
            let dockerfile_content = r#"# syntax=docker/dockerfile:1.19.0
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# get-composer
FROM composer:latest AS get-composer

# install-deps
FROM php:8.3-fpm-alpine AS install-deps
USER 0:0
RUN <<EOF
apt-get update
apk add --no-cache --update ca-certificates dcron curl git supervisor tar unzip nginx libpng-dev libxml2-dev libzip-dev icu-dev mysql-client
EOF

# install-php-ext
FROM install-deps AS install-php-ext
USER 0:0
RUN <<EOF
docker-php-ext-configure zip
docker-php-ext-install bcmath gd intl pdo_mysql zip
EOF

# runtime
FROM install-php-ext AS runtime
WORKDIR /
COPY \
    --from=get-composer \
    --chown=www-data \
    --link \
    "/usr/bin/composer" "/bin/"
ADD \
    --chown=www-data \
    --link \
    "https://github.com/pelican-dev/panel.git" "/tmp/pelican"
USER www-data
RUN <<EOF
cd /tmp/pelican
cp .env.example .env
mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache
chmod 777 -R bootstrap storage
composer install --no-dev --optimize-autoloader
rm -rf .env bootstrap/cache/*.php
mkdir -p /app/storage/logs/
chown -R nginx:nginx .
rm /usr/local/etc/php-fpm.conf
echo "* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1" >> /var/spool/cron/crontabs/root
mkdir -p /var/run/php /var/run/nginx
mv .github/docker/default.conf /etc/nginx/http.d/default.conf
mv .github/docker/supervisord.conf /etc/supervisord.conf
EOF
"#;

            let yaml = r#"builders:
  install-deps:
    fromImage: php:8.3-fpm-alpine
    root:
      run:
      - apt-get update
      - >-
        apk add --no-cache --update
        ca-certificates
        dcron
        curl
        git
        supervisor
        tar
        unzip
        nginx
        libpng-dev
        libxml2-dev
        libzip-dev
        icu-dev
        mysql-client
  install-php-ext:
    fromBuilder: install-deps
    root:
      run:
      # - docker-php-ext-configure gd --with-freetype --with-jpeg
      # - docker-php-ext-install -j$(nproc) gd zip intl curl mbstring mysqli
        - docker-php-ext-configure zip
        - docker-php-ext-install bcmath gd intl pdo_mysql zip
  get-composer:
    name: composer
    fromImage: composer:latest
fromBuilder: install-php-ext
workdir: /
user:
  user: www-data
copy:
- fromBuilder: get-composer
  paths: "/usr/bin/composer"
  target: "/bin/"
  chown:
    user: www-data
  link: true
- repo: 'https://github.com/pelican-dev/panel.git'
  target: '/tmp/pelican'
  chown:
    user: www-data
  link: true
run:
  - cd /tmp/pelican
  - cp .env.example .env
  - mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache
  - chmod 777 -R bootstrap storage
  - composer install --no-dev --optimize-autoloader
  - rm -rf .env bootstrap/cache/*.php
  - mkdir -p /app/storage/logs/
  - chown -R nginx:nginx .
  - rm /usr/local/etc/php-fpm.conf
  - echo "* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1" >> /var/spool/cron/crontabs/root
  - mkdir -p /var/run/php /var/run/nginx
  - mv .github/docker/default.conf /etc/nginx/http.d/default.conf
  - mv .github/docker/supervisord.conf /etc/supervisord.conf
"#;

            let dockerfile: DockerFile = dockerfile_content.parse().unwrap();

            let result = Dofigen::from_dockerfile(dockerfile, None);

            let dofigen_from_dockerfile = result.unwrap();

            assert_eq_sorted!(dofigen_from_dockerfile, Dofigen {
                builders: HashMap::from([
                    ("get-composer".to_string(), Stage {
                        from: FromContext::FromImage(
                            ImageName {
                                path: "composer".to_string(),
                                version: Some(
                                    ImageVersion::Tag(
                                        "latest".to_string(),
                                    ),
                                ),
                ..Default::default()
                            },
                        ),
                ..Default::default()
                    }),
                    ("install-deps".to_string(), Stage {
                        from: FromContext::FromImage(
                            ImageName {
                                path: "php".to_string(),
                                version: Some(
                                    ImageVersion::Tag(
                                        "8.3-fpm-alpine".to_string(),
                                    ),
                                ),
                ..Default::default()
                            },
                        ),
                        root: Some(
                            Run {
                                run: vec![
                                    "apt-get update".to_string(),
                                    "apk add --no-cache --update ca-certificates dcron curl git supervisor tar unzip nginx libpng-dev libxml2-dev libzip-dev icu-dev mysql-client".to_string(),
                                ],
                ..Default::default()
                            },
                        ),
                ..Default::default()
                    }),
                    ("install-php-ext".to_string(), Stage {
                        from: FromContext::FromBuilder(
                            "install-deps".to_string(),
                        ),
                        root: Some(
                            Run {
                                run: vec![
                                    "docker-php-ext-configure zip".to_string(),
                                    "docker-php-ext-install bcmath gd intl pdo_mysql zip".to_string(),
                                ],
                ..Default::default()
                            },
                        ),
                ..Default::default()
                    })
                    ]),
                stage: Stage {
                    from: FromContext::FromBuilder(
                        "install-php-ext".to_string(),
                    ),
                    user: Some(
                        User {
                            user: "www-data".to_string(),
                            group: None,
                        },
                    ),
                    workdir: Some(
                        "/".to_string(),
                    ),
                    copy: vec![
                        CopyResource::Copy(
                            Copy {
                                from: FromContext::FromBuilder(
                                    "get-composer".to_string(),
                                ),
                                paths: vec![
                                    "/usr/bin/composer".to_string(),
                                ],
                                options: CopyOptions {
                                   target: Some(
                                       "/bin/".to_string(),
                                   ),
                                   chown: Some(
                                       User {
                                           user: "www-data".to_string(),
                                           group: None,
                                       },
                                   ),
                                   link: Some(
                                       true,
                                   ),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ),
                        CopyResource::AddGitRepo(
                            AddGitRepo {
                                repo: "https://github.com/pelican-dev/panel.git".to_string(),
                                options: CopyOptions {
                                   target: Some(
                                       "/tmp/pelican".to_string(),
                                   ),
                                   chown: Some(
                                       User {
                                           user: "www-data".to_string(),
                                           group: None,
                                       },
                                   ),
                                   link: Some(
                                       true,
                                   ),
                ..Default::default()
                                },
                ..Default::default()
                            },
                        ),
                    ],
                    run: Run {
                        run: vec![
                            "cd /tmp/pelican".to_string(),
                            "cp .env.example .env".to_string(),
                            "mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache".to_string(),
                            "chmod 777 -R bootstrap storage".to_string(),
                            "composer install --no-dev --optimize-autoloader".to_string(),
                            "rm -rf .env bootstrap/cache/*.php".to_string(),
                            "mkdir -p /app/storage/logs/".to_string(),
                            "chown -R nginx:nginx .".to_string(),
                            "rm /usr/local/etc/php-fpm.conf".to_string(),
                            "echo \"* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1\" >> /var/spool/cron/crontabs/root".to_string(),
                            "mkdir -p /var/run/php /var/run/nginx".to_string(),
                            "mv .github/docker/default.conf /etc/nginx/http.d/default.conf".to_string(),
                            "mv .github/docker/supervisord.conf /etc/supervisord.conf".to_string(),
                        ],
                ..Default::default()
                    },
                                ..Default::default()
                },
                ..Default::default()
            });

            let dofigen_from_string: Dofigen = DofigenContext::new()
                .parse_from_string(yaml)
                .map_err(Error::from)
                .unwrap();

            assert_eq_sorted!(dofigen_from_dockerfile, dofigen_from_string);

            let mut context = GenerationContext::from(dofigen_from_string.clone());

            let generated_dockerfile = context.generate_dockerfile().unwrap();

            assert_eq_sorted!(dockerfile_content.to_string(), generated_dockerfile);
        }
    }
}

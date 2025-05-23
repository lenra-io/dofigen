use colored::{Color, Colorize};
use regex::Regex;

use crate::{
    DockerFile, DockerFileCommand, DockerFileInsctruction, DockerFileLine, DockerIgnore,
    DockerIgnoreLine, Dofigen, Error, FromContextPatch, LintMessage, MessageLevel, Result, Run,
    Stage, User,
};
use std::collections::HashMap;

impl Dofigen {
    pub fn from_dockerfile(
        dockerfile: DockerFile,
        dockerignore: Option<DockerIgnore>,
    ) -> Result<Self> {
        let mut dofigen = Self::default();
        let mut messages = vec![];

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
            dofigen.ignore = ignores;
        }

        let instructions: Vec<_> = dockerfile
            .lines
            .iter()
            .filter(|line| matches!(line, DockerFileLine::Instruction(_)))
            .collect();

        let mut current_stage_name: Option<String> = None;
        let mut current_stage: Option<Stage> = None;
        let mut current_root: Option<Run> = None;
        let mut last_inscruction: Option<DockerFileInsctruction> = None;
        let mut builders: HashMap<String, Stage> = HashMap::new();

        for line in instructions {
            if let DockerFileLine::Instruction(instruction) = line.clone() {
                let stage_name = current_stage_name
                    .clone()
                    .unwrap_or("Unnamed stage".to_string());
                match instruction.command {
                    DockerFileCommand::FROM => {
                        add_root(&mut current_stage, &mut current_root)?;
                        current_root = None;

                        if let Some(previous_stage) = current_stage {
                            builders.insert(
                                current_stage_name.unwrap_or(format!("builder-{}", builders.len())),
                                previous_stage,
                            );
                        }

                        let from = instruction.content.clone();
                        let pos = from.to_lowercase().find(" as ");
                        let (from_name, name) = if let Some(pos) = pos {
                            let (from, name) = from.split_at(pos);
                            let name = name[4..].trim();
                            (from.to_string(), name.to_string())
                        } else {
                            (from, "runtime".to_string())
                        };
                        current_stage_name = Some(name);
                        let from: FromContextPatch = if from_name == "scratch" {
                            FromContextPatch::FromContext(None)
                        } else if builders.contains_key(&from_name) {
                            FromContextPatch::FromBuilder(from_name)
                        } else {
                            FromContextPatch::FromImage(from_name.parse()?)
                        };

                        current_stage = Some(Stage {
                            from: from.into(),
                            ..Default::default()
                        });
                    }
                    DockerFileCommand::ARG => {
                        if let Some(inscruction) = last_inscruction {
                            if !matches!(
                                inscruction.command,
                                DockerFileCommand::FROM | DockerFileCommand::ARG
                            ) {
                                messages.push(LintMessage {
                                    level: MessageLevel::Warn,
                                    path: vec![stage_name.clone(), "ARG".to_string()],
                                    message: "The ARG instruction is not the first of the stage. It could be used in a previous instruction before the declaration".to_string(),
                                });
                            }
                        } else {
                            return Err(Error::Custom(format!(
                                "Global ARG instruction is not managed yet: {line:?}"
                            )));
                        }
                        let stage = get_stage(&mut current_stage, &instruction)?;
                        messages.extend(add_entries(
                            &mut stage.arg,
                            vec![stage_name.clone(), "ARG".to_string()],
                            instruction.content.clone(),
                        )?);
                    }
                    DockerFileCommand::LABEL => {
                        let stage = get_stage(&mut current_stage, &instruction)?;
                        messages.extend(add_entries(
                            &mut stage.label,
                            vec![stage_name.clone(), "LABEL".to_string()],
                            instruction.content.clone(),
                        )?);
                    }
                    DockerFileCommand::MAINTAINER => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::RUN => {
                        let run = if let Some(run) = current_root.as_mut() {
                            run
                        } else {
                            let stage = get_stage(&mut current_stage, &instruction)?;
                            &mut stage.run
                        };
                        if !run.is_empty() {
                            todo!("Many RUN instructions are not managed yet");
                        } else {
                            if !instruction.options.is_empty() {
                                todo!("RUN options are not managed yet");
                            }
                            if instruction.content.starts_with("<<EOF") {
                                let mut lines = instruction
                                    .content
                                    .lines()
                                    .map(str::to_string)
                                    .collect::<Vec<_>>();
                                lines.remove(0);
                                lines.remove(lines.len() - 1);
                                run.run.append(&mut lines);
                            } else {
                                todo!(
                                    "Not heredoc RUN is not managed yet: {:?}",
                                    instruction.content
                                );
                            }
                        }
                    }
                    DockerFileCommand::COPY => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::ADD => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::WORKDIR => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::ENV => {
                        let stage = get_stage(&mut current_stage, &instruction)?;
                        messages.extend(add_entries(
                            &mut stage.env,
                            vec![stage_name.clone(), "ENV".to_string()],
                            instruction.content.clone(),
                        )?);
                    }
                    DockerFileCommand::EXPOSE => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::USER => {
                        let stage = get_stage(&mut current_stage, &instruction)?;
                        let user = if let Some((user, group)) = instruction.content.split_once(":")
                        {
                            User {
                                user: user.to_string(),
                                group: Some(group.to_string()),
                            }
                        } else {
                            User {
                                user: instruction.content.clone(),
                                group: None,
                            }
                        };
                        if user.user == "0" {
                            current_root = Some(Run::default());
                        } else {
                            stage.user = Some(user);
                            add_root(&mut current_stage, &mut current_root)?;
                            current_root = None;
                        }
                    }
                    DockerFileCommand::VOLUME => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::SHELL => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::HEALTHCHECK => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::CMD => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::ENTRYPOINT => {
                        todo!("{:?} instruction is not managed yet", instruction.command)
                    }
                    DockerFileCommand::Unknown(command) => {
                        todo!("Unknown instruction {:?} is not managed yet", command)
                    }
                }
                last_inscruction = Some(instruction);
            }
        }

        if !builders.is_empty() {
            dofigen.builders = builders;
        }

        add_root(&mut current_stage, &mut current_root)?;

        dofigen.stage =
            current_stage.ok_or(Error::Custom("No FROM instruction found".to_string()))?;

        messages.iter().for_each(|message| {
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

        Ok(dofigen.into())
    }
}

fn add_root<'a>(stage: &'a mut Option<Stage>, root: &'a mut Option<Run>) -> Result<()> {
    if let Some(stage) = stage {
        if let Some(root) = root {
            stage.root = Some(root.clone());
        }
    }
    Ok(())
}

fn get_stage<'a>(
    stage: &'a mut Option<Stage>,
    instruction: &'a DockerFileInsctruction,
) -> Result<&'a mut Stage> {
    stage.as_mut().ok_or(Error::Custom(format!(
        "No FROM instruction found before line: {:?}",
        instruction
    )))
}

fn add_entries(
    entries: &mut HashMap<String, String>,
    path: Vec<String>,
    content: String,
) -> Result<Vec<LintMessage>> {
    let mut messages: Vec<LintMessage> = vec![];
    let (new_entries, parse_messages) = parse_key_value_entries(content)?;
    messages.extend(parse_messages.into_iter().map(|m| {
        let mut path = path.clone();
        path.extend(m.path);
        LintMessage { path, ..m }
    }));
    for (k, v) in new_entries {
        if entries.contains_key(&k) {
            let mut path = path.clone();
            path.push(k.clone());
            messages.push(LintMessage {
                message: format!("Duplicate key '{k}' found. The last one will be used."),
                level: MessageLevel::Warn,
                path,
            });
        }
        entries.insert(k, v);
    }
    Ok(messages)
}

fn parse_key_value_entries(content: String) -> Result<(HashMap<String, String>, Vec<LintMessage>)> {
    let mut entries = HashMap::new();
    let mut messages = Vec::new();
    let clean_content = content.replace("\\\n", "\n");
    let regex =
        Regex::new("(?<key>(?:[^=\\s\"]+|\"[^=\"]+\"))(?:=(?<value>(?:[^=\\s\"]+|\"[^=\"]+\")))?")?;
    for m in regex.find_iter(clean_content.as_str()) {
        let m = m.as_str();
        let captures = regex.captures(m).unwrap();
        let mut key = captures.name("key").unwrap().as_str().to_string();
        if key.starts_with('"') && key.ends_with('"') {
            key = key[1..key.len() - 1].to_string();
        }
        let mut value = captures
            .name("value")
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if value.starts_with('"') && value.ends_with('"') {
            value = value[1..value.len() - 1].to_string();
        }

        if entries.contains_key(&key) {
            messages.push(LintMessage {
                message: format!("Duplicate key '{key}' found. The last one will be used."),
                path: vec![key.clone()],
                level: MessageLevel::Warn,
            });
        }

        entries.insert(key, value);
    }
    Ok((entries, messages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DockerFileInsctruction, FromContext, ImageName};
    use pretty_assertions_sorted::assert_eq_sorted;

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
    }

    //     mod from_string {
    //         use crate::DofigenContext;

    //         use super::*;

    //         #[test]
    //         fn php_dockerfile() {
    //             let dockerfile_content = r#"# syntax=docker/dockerfile:1.11
    // # This file is generated by Dofigen v0.0.0
    // # See https://github.com/lenra-io/dofigen

    // # get-composer
    // FROM composer:latest AS get-composer

    // # install-deps
    // FROM php:8.3-fpm-alpine AS install-deps
    // USER 0:0
    // RUN <<EOF
    // apt-get update
    // apk add --no-cache --update ca-certificates dcron curl git supervisor tar unzip nginx libpng-dev libxml2-dev libzip-dev icu-dev mysql-client
    // EOF

    // # install-php-ext
    // FROM install-deps AS install-php-ext
    // USER 0:0
    // RUN <<EOF
    // docker-php-ext-configure zip
    // docker-php-ext-install bcmath gd intl pdo_mysql zip
    // EOF

    // # runtime
    // FROM install-php-ext AS runtime
    // WORKDIR /
    // COPY \
    //     --from=get-composer \
    //     --chown=www-data \
    //     --link \
    //     "/usr/bin/composer" "/bin/"
    // ADD \
    //     --chown=www-data \
    //     --link \
    //     "https://github.com/pelican-dev/panel.git" "/tmp/pelican"
    // USER www-data
    // RUN <<EOF
    // cd /tmp/pelican
    // cp .env.example .env
    // mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache
    // chmod 777 -R bootstrap storage
    // composer install --no-dev --optimize-autoloader
    // rm -rf .env bootstrap/cache/*.php
    // mkdir -p /app/storage/logs/
    // chown -R nginx:nginx .
    // rm /usr/local/etc/php-fpm.conf
    // echo "* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1" >> /var/spool/cron/crontabs/root
    // mkdir -p /var/run/php /var/run/nginx
    // mv .github/docker/default.conf /etc/nginx/http.d/default.conf
    // mv .github/docker/supervisord.conf /etc/supervisord.conf
    // EOF
    // "#;

    //             let yaml = r#"builders:
    //   install-deps:
    //     fromImage: php:8.3-fpm-alpine
    //     root:
    //       run:
    //       - apt-get update
    //       - >-
    //         apk add --no-cache --update
    //         ca-certificates
    //         dcron
    //         curl
    //         git
    //         supervisor
    //         tar
    //         unzip
    //         nginx
    //         libpng-dev
    //         libxml2-dev
    //         libzip-dev
    //         icu-dev
    //         mysql-client
    //   install-php-ext:
    //     fromBuilder: install-deps
    //     root:
    //       run:
    //       # - docker-php-ext-configure gd --with-freetype --with-jpeg
    //       # - docker-php-ext-install -j$(nproc) gd zip intl curl mbstring mysqli
    //         - docker-php-ext-configure zip
    //         - docker-php-ext-install bcmath gd intl pdo_mysql zip
    //   get-composer:
    //     name: composer
    //     fromImage: composer:latest
    // fromBuilder: install-php-ext
    // workdir: /
    // user: www-data
    // copy:
    // - fromBuilder: get-composer
    //   paths: "/usr/bin/composer"
    //   target: "/bin/"
    // - repo: 'https://github.com/pelican-dev/panel.git'
    //   target: '/tmp/pelican'
    // run:
    //   - cd /tmp/pelican
    //   - cp .env.example .env
    //   - mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache
    //   - chmod 777 -R bootstrap storage
    //   - composer install --no-dev --optimize-autoloader
    //   - rm -rf .env bootstrap/cache/*.php
    //   - mkdir -p /app/storage/logs/
    //   - chown -R nginx:nginx .
    //   - rm /usr/local/etc/php-fpm.conf
    //   - echo "* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1" >> /var/spool/cron/crontabs/root
    //   - mkdir -p /var/run/php /var/run/nginx
    //   - mv .github/docker/default.conf /etc/nginx/http.d/default.conf
    //   - mv .github/docker/supervisord.conf /etc/supervisord.conf
    // "#;

    //             let dockerfile: DockerFile = dockerfile_content.parse().unwrap();

    //             let result = Dofigen::from_dockerfile(dockerfile, None);

    //             let dofigen_from_dockerfile = result.unwrap();

    //             let dofigen_from_string: Dofigen = DofigenContext::new()
    //                 .parse_from_string(yaml)
    //                 .map_err(Error::from)
    //                 .unwrap();

    //             assert_eq_sorted!(dofigen_from_dockerfile, dofigen_from_string);
    //         }
    //     }
}

use std::collections::HashMap;

use regex::Regex;
use struct_patch::{Merge, Patch};

use crate::{
    AddPatch, Copy, CopyOptions, CopyOptionsPatch, CopyResourcePatch, DockerFileCommand,
    DockerFileLine, Error, FromContext, FromContextPatch, HealthcheckPatch, ImageNamePatch,
    LintMessage, MessageLevel, Port, PortPatch, Result, Run, Stage, User, UserPatch, VecDeepPatch,
    VecDeepPatchCommand, VecPatch, VecPatchCommand,
    parse::{context::ParseContext, split_from},
};

impl ParseContext {
    fn builder_exists(&self, name: &str) -> bool {
        self.dofigen.builders.contains_key(name)
    }

    pub(crate) fn apply(&mut self, line: &DockerFileLine) -> Result<()> {
        if let DockerFileLine::Instruction(instruction) = line {
            log::debug!("Applying instruction: {:?}", instruction);
            match &instruction.command {
                DockerFileCommand::FROM => {
                    self.apply_root()?;
                    self.current_shell = None;

                    if self.current_stage.is_some() {
                        self.add_current_stage_as_builder(
                            self.current_stage_name
                                .clone()
                                .expect("Stage name must be set"),
                        )?;
                    }

                    let (from_name, name) = split_from(&instruction.content);
                    self.current_stage_name = name.map(|n| n.to_string());
                    let from: FromContextPatch = if from_name == "scratch" {
                        FromContextPatch::FromContext(None)
                    } else if self.dofigen.builders.contains_key(from_name) {
                        FromContextPatch::FromBuilder(from_name.to_string())
                    } else {
                        FromContextPatch::FromImage(from_name.parse()?)
                    };

                    self.current_stage = Some(Stage {
                        from: from.into(),
                        ..Default::default()
                    });
                }
                DockerFileCommand::ARG => {
                    if let Some(inscruction) = &self.last_inscruction {
                        if !matches!(
                            inscruction.command,
                            DockerFileCommand::FROM | DockerFileCommand::ARG
                        ) {
                            self.messages.push(LintMessage {
                                    level: MessageLevel::Warn,
                                    path: self.get_current_message_path(line),
                                    message: "The ARG instruction is not the first of the stage. It could be used in a previous instruction before the declaration".to_string(),
                                });
                        }
                    } else {
                        return Err(Error::Custom(format!(
                            "Global ARG instruction is not managed yet: {line:?}"
                        )));
                    }
                    let path = self.get_current_message_path(line);
                    let stage = self.current_stage(&instruction)?;
                    let new_messages =
                        add_entries(&mut stage.arg, path, instruction.content.clone())?;
                    self.messages.extend(new_messages);
                }
                DockerFileCommand::LABEL => {
                    let path = self.get_current_message_path(line);
                    let stage = self.current_stage(&instruction)?;
                    let new_messages =
                        add_entries(&mut stage.label, path, instruction.content.clone())?;
                    self.messages.extend(new_messages);
                }
                DockerFileCommand::MAINTAINER => {
                    // Transform to LABEL org.opencontainers.image.authors
                    let path = self.get_current_message_path(line);
                    let stage = self.current_stage(&instruction)?;
                    add_entries(
                        &mut stage.label,
                        path,
                        format!(
                            "{}={}",
                            "org.opencontainers.image.authors", instruction.content,
                        ),
                    )?;
                }
                DockerFileCommand::RUN => {
                    let current_shell = self.current_shell.clone();
                    let run = if let Some(run) = self.current_root.as_mut() {
                        run
                    } else {
                        &mut self.current_stage(&instruction)?.run
                    };
                    if !run.is_empty() {
                        todo!("Many RUN instructions are not managed yet");
                    } else {
                        if let Some(shell) = current_shell {
                            run.shell = shell.clone();
                        }
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
                            let mut commands = instruction
                                .content
                                .split("&&")
                                .map(str::trim)
                                .map(str::to_string)
                                .collect::<Vec<_>>();
                            run.run.append(&mut commands);
                        }
                    }
                }
                DockerFileCommand::COPY => {
                    // TODO: manege heredocs
                    let copy = instruction.content.parse::<CopyResourcePatch>()?;
                    let mut copy: Copy = if let CopyResourcePatch::Copy(copy) = copy {
                        copy.into()
                    } else {
                        return Err(Error::Custom(
                            "COPY instruction content must be a CopyResourcePatch".to_string(),
                        ));
                    };
                    let target = copy.options.target.ok_or(Error::Custom(
                        "COPY instruction must have at least one source and a target".to_string(),
                    ))?;
                    let (options, exclude, not_managed_options) =
                        parse_copy_options(&instruction.options)?;

                    let mut options: CopyOptions = options.into();

                    options.target = Some(target);
                    copy.exclude = exclude;

                    for option in not_managed_options.iter() {
                        match option {
                            crate::InstructionOption::Flag(name) => match name.as_str() {
                                "parents" => copy.parents = Some(true),
                                _ => unreachable!("Unknown COPY flag option: {name}"),
                            },
                            crate::InstructionOption::WithValue(name, value) => match name.as_str()
                            {
                                "parents" => copy.parents = Some(true),
                                "from" => {
                                    if self.builder_exists(value) {
                                        copy.from = FromContext::FromBuilder(value.clone());
                                    } else if value == "scratch" {
                                        copy.from = FromContext::FromContext(None);
                                    } else {
                                        if let Ok(image) = value.parse::<ImageNamePatch>() {
                                            copy.from = FromContext::FromImage(image.into());
                                        } else {
                                            copy.from =
                                                FromContext::FromContext(Some(value.clone()));
                                        }
                                    }
                                }
                                _ => unreachable!("Unknown COPY option: {name}"),
                            },
                            crate::InstructionOption::WithOptions(
                                name,
                                instruction_option_options,
                            ) => {
                                todo!(
                                    "Unknown COPY option {name} with sub options: {instruction_option_options:?}"
                                )
                            }
                        }
                    }
                    copy.options = options;
                    self.add_copy(&instruction, crate::CopyResource::Copy(copy))?;
                }
                DockerFileCommand::ADD => {
                    let (options, exclude, not_managed_options) =
                        parse_copy_options(&instruction.options)?;
                    let add_options = CopyResourcePatch::Unknown(crate::UnknownPatch {
                        options: Some(options),
                        exclude: Some(exclude.into_patch()),
                    });

                    let copy_resource = instruction.content.parse::<CopyResourcePatch>()?;
                    let copy_resource = if let CopyResourcePatch::AddGitRepo(add_git) =
                        &copy_resource
                    {
                        let mut add_git = add_git.clone();
                        println!("add_git: {add_git:?}");
                        for option in not_managed_options.iter() {
                            match option {
                                crate::InstructionOption::Flag(name) => match name.as_str() {
                                    _ => unreachable!("Unknown ADD flag option: {name}"),
                                },
                                crate::InstructionOption::WithValue(name, value) => {
                                    match name.as_str() {
                                        "keep-git-dir" => {
                                            add_git.keep_git_dir =
                                                Some(Some(value.parse().map_err(Error::from)?));
                                        }
                                        _ => unreachable!("Unknown ADD option: {name}"),
                                    }
                                }
                                crate::InstructionOption::WithOptions(
                                    name,
                                    instruction_option_options,
                                ) => todo!(
                                    "Unknown ADD option {name} with sub options: {instruction_option_options:?}"
                                ),
                            }
                        }
                        copy_resource
                    } else {
                        let mut add = instruction.content.parse::<AddPatch>()?;

                        for option in not_managed_options.iter() {
                            match option {
                                crate::InstructionOption::Flag(name) => match name.as_str() {
                                    _ => unreachable!("Unknown ADD flag option: {name}"),
                                },
                                crate::InstructionOption::WithValue(name, value) => {
                                    match name.as_str() {
                                        "checksum" => {
                                            add.checksum = Some(Some(value.clone()));
                                        }
                                        _ => unreachable!("Unknown ADD option: {name}"),
                                    }
                                }
                                crate::InstructionOption::WithOptions(
                                    name,
                                    instruction_option_options,
                                ) => todo!(
                                    "Unknown ADD option {name} with sub options: {instruction_option_options:?}"
                                ),
                            }
                        }
                        CopyResourcePatch::Add(add)
                    };

                    let copy_resource = copy_resource.merge(add_options);
                    self.add_copy(&instruction, copy_resource.into())?;
                }
                DockerFileCommand::WORKDIR => {
                    let stage = self.current_stage(&instruction)?;
                    if stage.workdir.is_none() {
                        stage.workdir = Some(instruction.content.clone());
                    } else {
                        todo!("Many WORKDIR instructions in the same stage are not managed yet");
                    }
                }
                DockerFileCommand::ENV => {
                    let path = self.get_current_message_path(line);
                    let stage = self.current_stage(&instruction)?;
                    let new_messages =
                        add_entries(&mut stage.env, path, instruction.content.clone())?;
                    self.messages.extend(new_messages);
                }
                DockerFileCommand::EXPOSE => {
                    let dofigen_patch = self.current_dofigen_patch(instruction)?;
                    let ports = instruction
                        .content
                        .clone()
                        .split_whitespace()
                        .map(|port_str| {
                            port_str
                                .parse()
                                .map_err(Error::from)
                                .map(|port_patch: PortPatch| {
                                    let mut port = Port::default();
                                    port.apply(port_patch);
                                    port
                                })
                        })
                        .collect::<Result<Vec<Port>>>()?;
                    let expose = if let Some(expose) = dofigen_patch.expose.as_mut() {
                        expose
                    } else {
                        dofigen_patch.expose = Some(VecDeepPatch::default());
                        &mut dofigen_patch.expose.as_mut().unwrap()
                    };
                    expose.commands.push(VecDeepPatchCommand::Append(ports));
                }
                DockerFileCommand::USER => {
                    let has_not_applied_root = self.current_root.is_some();
                    let stage = self.current_stage.as_ref();
                    let has_user = stage.and_then(|s| s.user.as_ref()).is_some();
                    let has_run = stage.map(|s| !s.run.is_empty()).unwrap_or(false);
                    let user = if let Some((user, group)) = instruction.content.split_once(":") {
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
                    if has_user || has_run {
                        self.split_current_stage()?;
                    }
                    if user.user == "0" || user.user.to_lowercase() == "root" {
                        if has_not_applied_root {
                            todo!("Many ROOT USER instructions are not managed yet");
                        }
                        self.current_root = Some(Run::default());
                    } else {
                        let stage = self.current_stage(&instruction)?;
                        stage.user = Some(user);
                        self.apply_root()?;
                    }
                }
                DockerFileCommand::VOLUME => {
                    let dofigen_patch = self.current_dofigen_patch(instruction)?;
                    let volumes = parse_json_array(&instruction.content)?;
                    let volume = if let Some(volume) = dofigen_patch.volume.as_mut() {
                        volume
                    } else {
                        dofigen_patch.volume = Some(VecPatch::default());
                        &mut dofigen_patch.volume.as_mut().unwrap()
                    };
                    volume.commands.push(VecPatchCommand::Append(volumes));
                }
                DockerFileCommand::SHELL => {
                    self.current_shell = Some(parse_json_array(&instruction.content)?);
                }
                DockerFileCommand::HEALTHCHECK => {
                    let path = self.get_current_message_path(line);
                    let mut healthcheck = HealthcheckPatch {
                        cmd: Some(parse_json_array(&instruction.content)?.join(" ")),
                        interval: Some(None),
                        retries: Some(None),
                        start: Some(None),
                        timeout: Some(None),
                    };
                    instruction.options.iter().for_each(|option| {
                            match option {
                                crate::InstructionOption::WithValue(name, value) => {
                                    match name.as_str() {
                                        "interval" => {
                                            healthcheck.interval = Some(Some(value.clone()));
                                            return;
                                        }
                                        "retries" => {
                                            if let Ok(parsed) = value.parse() {
                                                healthcheck.retries = Some(Some(parsed));
                                            } else {
                                                self.messages.push(LintMessage {
                                    level: MessageLevel::Error,
                                    message: format!("Could not parse healthcheck {} option for value: '{}'", name, value),
                                    path: path.clone(),
                                });
                                            }
                                            return;
                                        }
                                        "start-period" => {
                                            healthcheck.start = Some(Some(value.clone()));
                                            return;
                                        }
                                        "timeout" => {
                                            healthcheck.timeout = Some(Some(value.clone()));
                                            return;
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                            self.messages.push(LintMessage {
                                level: MessageLevel::Warn,
                                message: format!(
                                    "HEALTHCHECK option '{option:?}' is not managed yet"
                                ),
                                path: self.get_current_message_path(line),
                            });
                        });
                    let dofigen_patch = self.current_dofigen_patch(instruction)?;
                    dofigen_patch.healthcheck = Some(Some(healthcheck));
                }
                DockerFileCommand::CMD => {
                    let dofigen_patch = self.current_dofigen_patch(instruction)?;
                    dofigen_patch.cmd = Some(VecPatch {
                        commands: vec![VecPatchCommand::ReplaceAll(parse_json_array(
                            &instruction.content,
                        )?)],
                    });
                }
                DockerFileCommand::ENTRYPOINT => {
                    let dofigen_patch = self.current_dofigen_patch(instruction)?;
                    dofigen_patch.entrypoint = Some(VecPatch {
                        commands: vec![VecPatchCommand::ReplaceAll(parse_json_array(
                            &instruction.content,
                        )?)],
                    });
                }
                DockerFileCommand::Unknown(command) => {
                    todo!("Unknown instruction {:?} is not managed yet", command)
                }
            }
            self.last_inscruction = Some(instruction.clone());
        }
        Ok(())
    }
}

fn parse_copy_options(
    options: &[crate::InstructionOption],
) -> Result<(CopyOptionsPatch, Vec<String>, Vec<crate::InstructionOption>)> {
    let mut copy_options = CopyOptionsPatch::default();
    let mut exclude = vec![];
    let mut not_managed_options = vec![];
    for option in options {
        match option {
            crate::InstructionOption::Flag(name) => match name.as_str() {
                "link" => copy_options.link = Some(Some(true)),
                _ => not_managed_options.push(option.clone()),
            },
            crate::InstructionOption::WithValue(name, value) => match name.as_str() {
                "link" => copy_options.link = Some(Some(value.parse().map_err(Error::from)?)),
                "chown" => copy_options.chown = Some(Some(value.parse::<UserPatch>()?.into())),
                "chmod" => copy_options.chmod = Some(Some(value.clone())),
                "exclude" => exclude.push(value.clone()),
                _ => not_managed_options.push(option.clone()),
            },
            crate::InstructionOption::WithOptions(_, _) => not_managed_options.push(option.clone()),
        }
    }
    Ok((copy_options, exclude, not_managed_options))
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

fn parse_json_array(content: &str) -> Result<Vec<String>> {
    return if content.starts_with('[') && content.ends_with(']') {
        serde_json::from_str(content).map_err(Error::from)
    } else {
        Ok(vec![content.to_string()])
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dockerfile_struct::*;
    use crate::dofigen_struct::*;
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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.from,
                FromContext::FromImage(ImageName {
                    path: "ubuntu".to_string(),
                    version: Some(ImageVersion::Tag("25.04".to_string())),
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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 1);
            assert!(dofigen.builders.contains_key("builder"));
            assert_eq_sorted!(
                dofigen.builders["builder"].from,
                FromContext::FromImage(ImageName {
                    path: "ubuntu".to_string(),
                    version: Some(ImageVersion::Tag("25.04".to_string())),
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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

            assert_eq_sorted!(
                dofigen.builders,
                HashMap::from([(
                    "test".to_string(),
                    Stage {
                        from: FromContext::FromImage(ImageName {
                            path: "ubuntu".to_string(),
                            version: Some(ImageVersion::Tag("25.04".to_string())),
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

            assert_eq_sorted!(error.to_string(), "No FROM instruction found in Dockerfile");
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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

    mod copy {
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
                        command: DockerFileCommand::COPY,
                        content: "file.txt /app/".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.copy,
                vec![CopyResource::Copy(Copy {
                    paths: vec!["file.txt".to_string()],
                    options: CopyOptions {
                        target: Some("/app/".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                })]
            );
        }

        #[test]
        fn many() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::COPY,
                        content: "file1.txt /app/".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::COPY,
                        content: "file2.txt /app/".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.copy,
                vec![
                    CopyResource::Copy(Copy {
                        paths: vec!["file1.txt".to_string()],
                        options: CopyOptions {
                            target: Some("/app/".to_string()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    CopyResource::Copy(Copy {
                        paths: vec!["file2.txt".to_string()],
                        options: CopyOptions {
                            target: Some("/app/".to_string()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                ]
            );
        }

        #[test]
        fn copy_and_add() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::COPY,
                        content: "file1.txt /app/".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ADD,
                        content: "file2.txt /app/".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dockerignore = None;

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 0);
            assert_eq_sorted!(
                dofigen.stage.copy,
                vec![
                    CopyResource::Copy(Copy {
                        paths: vec!["file1.txt".to_string()],
                        options: CopyOptions {
                            target: Some("/app/".to_string()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    CopyResource::Add(Add {
                        files: vec![Resource::File("file2.txt".into()),],
                        options: CopyOptions {
                            target: Some("/app/".to_string()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                ]
            );
        }

        #[test]
        fn with_chown_chmod_link_flag() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::COPY,
                        content: "file.txt /app/".to_string(),
                        options: vec![
                            crate::InstructionOption::Flag("link".to_string()),
                            crate::InstructionOption::WithValue(
                                "chown".to_string(),
                                "user:group".to_string(),
                            ),
                            crate::InstructionOption::WithValue(
                                "chmod".to_string(),
                                "0755".to_string(),
                            ),
                        ],
                    }),
                ],
            };
            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq_sorted!(
                dofigen.stage.copy,
                vec![CopyResource::Copy(Copy {
                    paths: vec!["file.txt".to_string()],
                    options: CopyOptions {
                        target: Some("/app/".to_string()),
                        chown: Some(User {
                            user: "user".to_string(),
                            group: Some("group".to_string())
                        }),
                        chmod: Some("0755".to_string()),
                        link: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                })]
            );
        }

        #[test]
        fn with_exclude_parents_and_from_builder() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04 as builder".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::COPY,
                        content: "file.txt /app/".to_string(),
                        options: vec![
                            crate::InstructionOption::WithValue(
                                "exclude".to_string(),
                                "*.log".to_string(),
                            ),
                            crate::InstructionOption::WithValue(
                                "from".to_string(),
                                "builder".to_string(),
                            ),
                            crate::InstructionOption::Flag("parents".to_string()),
                        ],
                    }),
                ],
            };
            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq_sorted!(
                dofigen.stage.copy,
                vec![CopyResource::Copy(Copy {
                    paths: vec!["file.txt".to_string()],
                    options: CopyOptions {
                        target: Some("/app/".to_string()),
                        ..Default::default()
                    },
                    exclude: vec!["*.log".to_string()],
                    parents: Some(true),
                    from: FromContext::FromBuilder("builder".to_string()),
                    ..Default::default()
                })]
            );
        }

        #[test]
        fn add_with_checksum_option() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::ADD,
                        content: "file2.txt /app/".to_string(),
                        options: vec![crate::InstructionOption::WithValue(
                            "checksum".to_string(),
                            "sha256:abcd".to_string(),
                        )],
                    }),
                ],
            };
            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq_sorted!(
                dofigen.stage.copy,
                vec![CopyResource::Add(Add {
                    files: vec![Resource::File("file2.txt".into())],
                    options: CopyOptions {
                        target: Some("/app/".to_string()),
                        ..Default::default()
                    },
                    checksum: Some("sha256:abcd".to_string()),
                    ..Default::default()
                })]
            );
        }
    }

    mod run {
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
                        command: DockerFileCommand::RUN,
                        content: "echo Hello World".to_string(),
                        options: vec![],
                    }),
                ],
            };
            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();
            assert_eq_sorted!(
                dofigen.stage.run,
                Run {
                    run: vec!["echo Hello World".to_string()],
                    ..Default::default()
                }
            );
        }

        #[test]
        fn run_heredoc() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::RUN,
                        content: "<<EOF\nline1\nline2\nEOF".to_string(),
                        options: vec![],
                    }),
                ],
            };

            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq_sorted!(
                dofigen.stage.run,
                Run {
                    run: vec!["line1".to_string(), "line2".to_string()],
                    ..Default::default()
                }
            );
        }

        #[test]
        fn run_split_with_and_and() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::RUN,
                        content: "echo one &&   echo two".to_string(),
                        options: vec![],
                    }),
                ],
            };

            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq_sorted!(
                dofigen.stage.run,
                Run {
                    run: vec!["echo one".to_string(), "echo two".to_string()],
                    ..Default::default()
                }
            );
        }

        #[test]
        fn before_copy() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::RUN,
                        content: "echo Coucou".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::COPY,
                        content: "file.txt /app/".to_string(),
                        options: vec![],
                    }),
                ],
            };

            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 1);
            assert_eq_sorted!(
                dofigen,
                Dofigen {
                    builders: HashMap::from([(
                        "runtime-builder-1".to_string(),
                        Stage {
                            from: FromContext::FromImage(ImageName {
                                path: "ubuntu".to_string(),
                                version: Some(ImageVersion::Tag("25.04".to_string(),),),
                                ..Default::default()
                            }),
                            run: Run {
                                run: vec!["echo Coucou".to_string()],
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    )]),
                    stage: Stage {
                        from: FromContext::FromBuilder("runtime-builder-1".to_string()),
                        copy: vec![CopyResource::Copy(Copy {
                            paths: vec!["file.txt".to_string()],
                            options: CopyOptions {
                                target: Some("/app/".to_string()),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),],
                        ..Default::default()
                    },
                    ..Default::default()
                }
            );
        }

        #[test]
        fn before_root() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::RUN,
                        content: "echo before root".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::USER,
                        content: "0".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::RUN,
                        content: "echo after root".to_string(),
                        options: vec![],
                    }),
                ],
            };

            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 1);
            assert_eq_sorted!(
                dofigen,
                Dofigen {
                    builders: HashMap::from([(
                        "runtime-builder-1".to_string(),
                        Stage {
                            from: FromContext::FromImage(ImageName {
                                path: "ubuntu".to_string(),
                                version: Some(ImageVersion::Tag("25.04".to_string(),),),
                                ..Default::default()
                            }),
                            run: Run {
                                run: vec!["echo before root".to_string()],
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    )]),
                    stage: Stage {
                        from: FromContext::FromBuilder("runtime-builder-1".to_string()),
                        root: Some(Run {
                            run: vec!["echo after root".to_string()],
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            );
        }

        #[test]
        fn before_user() {
            let dockerfile = DockerFile {
                lines: vec![
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::RUN,
                        content: "echo before user".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::USER,
                        content: "1000".to_string(),
                        options: vec![],
                    }),
                ],
            };

            let dofigen = Dofigen::from_dockerfile(dockerfile, None).unwrap();

            assert_eq!(dofigen.ignore.len(), 0);
            assert_eq!(dofigen.builders.len(), 1);
            assert_eq_sorted!(
                dofigen,
                Dofigen {
                    builders: HashMap::from([(
                        "runtime-builder-1".to_string(),
                        Stage {
                            from: FromContext::FromImage(ImageName {
                                path: "ubuntu".to_string(),
                                version: Some(ImageVersion::Tag("25.04".to_string(),),),
                                ..Default::default()
                            }),
                            run: Run {
                                run: vec!["echo before user".to_string()],
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    )]),
                    stage: Stage {
                        from: FromContext::FromBuilder("runtime-builder-1".to_string()),
                        user: Some(User {
                            user: "1000".to_string(),
                            group: None
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            );
        }
    }

    mod expose {
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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

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

            let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore).unwrap();

            assert_eq_sorted!(dofigen.entrypoint, vec!["/entrypoint.sh".to_string()]);
        }
    }
}

use std::collections::HashMap;

use regex::Regex;
use struct_patch::{Merge, Patch};

use crate::{
    AddPatch, Copy, CopyOptions, CopyOptionsPatch, CopyResourcePatch, DockerFileCommand,
    DockerFileInsctruction, DockerFileLine, DofigenPatch, Error, FromContext, FromContextPatch,
    HealthcheckPatch, ImageNamePatch, LintMessage, MessageLevel, Port, PortPatch, Result, Run,
    Stage, User, UserPatch, VecDeepPatch, VecDeepPatchCommand, VecPatch, VecPatchCommand,
    parse::context::ParseContext,
};

impl ParseContext {
    fn builder_exists(&self, name: &str) -> bool {
        self.builders.contains_key(name)
    }

    pub(crate) fn apply(&mut self, line: &DockerFileLine) -> Result<()> {
        if let DockerFileLine::Instruction(instruction) = line {
            let stage_name = self
                .current_stage_name
                .clone()
                .unwrap_or("Unnamed stage".to_string());
            match &instruction.command {
                DockerFileCommand::FROM => {
                    self.apply_root()?;
                    self.current_shell = None;

                    if let Some(previous_stage) = &self.current_stage {
                        self.builders.insert(
                            self.current_stage_name
                                .clone()
                                .unwrap_or(format!("builder-{}", self.builders.len())),
                            previous_stage.clone(),
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
                    self.current_stage_name = Some(name);
                    let from: FromContextPatch = if from_name == "scratch" {
                        FromContextPatch::FromContext(None)
                    } else if self.builders.contains_key(&from_name) {
                        FromContextPatch::FromBuilder(from_name)
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
                                    path: vec![stage_name.clone(), "ARG".to_string()],
                                    message: "The ARG instruction is not the first of the stage. It could be used in a previous instruction before the declaration".to_string(),
                                });
                        }
                    } else {
                        return Err(Error::Custom(format!(
                            "Global ARG instruction is not managed yet: {line:?}"
                        )));
                    }
                    let stage = self.current_stage(Some(&instruction))?;
                    let new_messages = add_entries(
                        &mut stage.arg,
                        vec![stage_name.clone(), "ARG".to_string()],
                        instruction.content.clone(),
                    )?;
                    self.messages.extend(new_messages);
                }
                DockerFileCommand::LABEL => {
                    let stage = self.current_stage(Some(&instruction))?;
                    let new_messages = add_entries(
                        &mut stage.label,
                        vec![stage_name.clone(), "LABEL".to_string()],
                        instruction.content.clone(),
                    )?;
                    self.messages.extend(new_messages);
                }
                DockerFileCommand::MAINTAINER => {
                    // Transform to LABEL org.opencontainers.image.authors
                    let stage = self.current_stage(Some(&instruction))?;
                    add_entries(
                        &mut stage.label,
                        vec![stage_name.clone(), "MAINTAINER".to_string()],
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
                        &mut self.current_stage(Some(&instruction))?.run
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
                    let stage = self.current_stage(Some(&instruction))?;
                    stage.copy.push(crate::CopyResource::Copy(copy));
                }
                DockerFileCommand::ADD => {
                    let stage = self.current_stage(Some(&instruction))?;
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

                    stage.copy.push(copy_resource.into());
                }
                DockerFileCommand::WORKDIR => {
                    let stage = self.current_stage(Some(&instruction))?;
                    if stage.workdir.is_none() {
                        stage.workdir = Some(instruction.content.clone());
                    } else {
                        todo!("Many WORKDIR instructions in the same stage are not managed yet");
                    }
                }
                DockerFileCommand::ENV => {
                    let stage = self.current_stage(Some(&instruction))?;
                    let new_messages = add_entries(
                        &mut stage.env,
                        vec![stage_name.clone(), "ENV".to_string()],
                        instruction.content.clone(),
                    )?;
                    self.messages.extend(new_messages);
                }
                DockerFileCommand::EXPOSE => {
                    let dofigen_patch = get_dofigen_patch(
                        &self.current_stage_name,
                        &mut self.builder_dofigen_patches,
                        &instruction,
                    )?;
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
                    let stage = self.current_stage(Some(&instruction))?;
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
                    if user.user == "0" {
                        self.current_root = Some(Run::default());
                    } else {
                        stage.user = Some(user);
                        self.apply_root()?;
                    }
                }
                DockerFileCommand::VOLUME => {
                    let dofigen_patch = get_dofigen_patch(
                        &self.current_stage_name,
                        &mut self.builder_dofigen_patches,
                        &instruction,
                    )?;
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
                    let dofigen_patch = get_dofigen_patch(
                        &self.current_stage_name,
                        &mut self.builder_dofigen_patches,
                        &instruction,
                    )?;
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
                                    path: vec![stage_name.clone(), "HEALTHCHECK".to_string()],
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
                                path: vec![stage_name.clone(), "HEALTHCHECK".to_string()],
                            });
                        });
                    dofigen_patch.healthcheck = Some(Some(healthcheck));
                }
                DockerFileCommand::CMD => {
                    let dofigen_patch = get_dofigen_patch(
                        &self.current_stage_name,
                        &mut self.builder_dofigen_patches,
                        &instruction,
                    )?;
                    dofigen_patch.cmd = Some(VecPatch {
                        commands: vec![VecPatchCommand::ReplaceAll(parse_json_array(
                            &instruction.content,
                        )?)],
                    });
                }
                DockerFileCommand::ENTRYPOINT => {
                    let dofigen_patch = get_dofigen_patch(
                        &self.current_stage_name,
                        &mut self.builder_dofigen_patches,
                        &instruction,
                    )?;
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

fn get_dofigen_patch<'a>(
    stage_name: &'a Option<String>,
    stage_dofigen_patches: &'a mut HashMap<String, DofigenPatch>,
    instruction: &'a DockerFileInsctruction,
) -> Result<&'a mut DofigenPatch> {
    Ok(stage_dofigen_patches
        .entry(stage_name.clone().ok_or(Error::Custom(format!(
            "No FROM instruction found before line: {:?}",
            instruction
        )))?)
        .or_insert_with(DofigenPatch::default))
}

// fn get_stage<'a>(
//     stage: &'a mut Option<Stage>,
//     instruction: &'a DockerFileInsctruction,
// ) -> Result<&'a mut Stage> {
//     stage.as_mut().ok_or(Error::Custom(format!(
//         "No FROM instruction found before line: {:?}",
//         instruction
//     )))
// }

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

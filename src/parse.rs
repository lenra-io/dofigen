use colored::{Color, Colorize};
use regex::Regex;
use struct_patch::{Merge, Patch};

use crate::{
    AddPatch, Copy, CopyOptions, CopyOptionsPatch, CopyResourcePatch, DockerFile,
    DockerFileCommand, DockerFileInsctruction, DockerFileLine, DockerIgnore, DockerIgnoreLine,
    Dofigen, DofigenPatch, Error, FromContext, FromContextPatch, HealthcheckPatch, ImageNamePatch,
    LintMessage, MessageLevel, Port, PortPatch, Result, Run, Stage, User, UserPatch, VecDeepPatch,
    VecDeepPatchCommand, VecPatch, VecPatchCommand,
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
        let mut current_shell: Option<Vec<String>> = None;
        let mut builders: HashMap<String, Stage> = HashMap::new();
        let mut builder_dofigen_patches: HashMap<String, DofigenPatch> = HashMap::new();

        for line in instructions {
            if let DockerFileLine::Instruction(instruction) = line.clone() {
                let stage_name = current_stage_name
                    .clone()
                    .unwrap_or("Unnamed stage".to_string());
                match instruction.command {
                    DockerFileCommand::FROM => {
                        add_root(&mut current_stage, &mut current_root)?;
                        current_root = None;
                        current_shell = None;

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
                        // Transform to LABEL org.opencontainers.image.authors
                        let stage = get_stage(&mut current_stage, &instruction)?;
                        messages.extend(add_entries(
                            &mut stage.label,
                            vec![stage_name.clone(), "MAINTAINER".to_string()],
                            format!(
                                "{}={}",
                                "org.opencontainers.image.authors", instruction.content,
                            ),
                        )?);
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
                            if let Some(shell) = &current_shell {
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
                        let stage = get_stage(&mut current_stage, &instruction)?;
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
                            "COPY instruction must have at least one source and a target"
                                .to_string(),
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
                                crate::InstructionOption::WithValue(name, value) => {
                                    match name.as_str() {
                                        "parents" => copy.parents = Some(true),
                                        "from" => {
                                            if builders.contains_key(value) {
                                                copy.from = FromContext::FromBuilder(value.clone());
                                            } else if value == "scratch" {
                                                copy.from = FromContext::FromContext(None);
                                            } else {
                                                if let Ok(image) = value.parse::<ImageNamePatch>() {
                                                    copy.from =
                                                        FromContext::FromImage(image.into());
                                                } else {
                                                    copy.from = FromContext::FromContext(Some(
                                                        value.clone(),
                                                    ));
                                                }
                                            }
                                        }
                                        _ => unreachable!("Unknown COPY option: {name}"),
                                    }
                                }
                                crate::InstructionOption::WithOptions(
                                    name,
                                    instruction_option_options,
                                ) => todo!(
                                    "Unknown COPY option {name} with sub options: {instruction_option_options:?}"
                                ),
                            }
                        }
                        copy.options = options;
                        stage.copy.push(crate::CopyResource::Copy(copy));
                    }
                    DockerFileCommand::ADD => {
                        let stage = get_stage(&mut current_stage, &instruction)?;
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
                        let stage = get_stage(&mut current_stage, &instruction)?;
                        if stage.workdir.is_none() {
                            stage.workdir = Some(instruction.content.clone());
                        } else {
                            todo!(
                                "Many WORKDIR instructions in the same stage are not managed yet"
                            );
                        }
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
                        let dofigen_patch = get_dofigen_patch(
                            &current_stage_name,
                            &mut builder_dofigen_patches,
                            &instruction,
                        )?;
                        let ports = instruction
                            .content
                            .clone()
                            .split_whitespace()
                            .map(|port_str| {
                                port_str.parse().map_err(Error::from).map(
                                    |port_patch: PortPatch| {
                                        let mut port = Port::default();
                                        port.apply(port_patch);
                                        port
                                    },
                                )
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
                        let dofigen_patch = get_dofigen_patch(
                            &current_stage_name,
                            &mut builder_dofigen_patches,
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
                        current_shell = Some(parse_json_array(&instruction.content)?);
                    }
                    DockerFileCommand::HEALTHCHECK => {
                        let dofigen_patch = get_dofigen_patch(
                            &current_stage_name,
                            &mut builder_dofigen_patches,
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
                                                messages.push(LintMessage {
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
                            messages.push(LintMessage {
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
                            &current_stage_name,
                            &mut builder_dofigen_patches,
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
                            &current_stage_name,
                            &mut builder_dofigen_patches,
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
                last_inscruction = Some(instruction);
            }
        }

        // Get runtime informations
        let runtime_stage = current_stage
            .clone()
            .ok_or(Error::Custom("No FROM instruction found".to_string()))?;
        let runtime_name =
            current_stage_name.ok_or(Error::Custom("No FROM instruction found".to_string()))?;

        // Get base instructions in from builders
        let mut dofigen_patches = builder_dofigen_patches
            .remove(&runtime_name)
            .into_iter()
            .collect::<Vec<_>>();
        let mut searching_stage = runtime_stage.clone();
        while let FromContext::FromBuilder(builder_name) = searching_stage.from.clone() {
            if let Some(builder_dofigen_patch) = builder_dofigen_patches.remove(&builder_name) {
                dofigen_patches.insert(0, builder_dofigen_patch);
            }
            searching_stage = builders
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
                dofigen.apply(dofigen_patch.clone());
            });
        }

        // Set builders
        if !builders.is_empty() {
            dofigen.builders = builders;
        }

        add_root(&mut current_stage, &mut current_root)?;

        dofigen.stage = runtime_stage;

        // Handle lint messages
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

fn add_root<'a>(stage: &'a mut Option<Stage>, root: &'a mut Option<Run>) -> Result<()> {
    if let Some(stage) = stage {
        if let Some(root) = root {
            stage.root = Some(root.clone());
        }
    }
    Ok(())
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

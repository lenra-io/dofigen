use crate::{
    DockerFile, DockerFileLine, DockerIgnore, DockerIgnoreLine, Dofigen, Error, FromContextPatch,
    Result, Stage,
};
use std::collections::HashMap;

impl Dofigen {
    pub fn from_dockerfile(
        dockerfile: DockerFile,
        dockerignore: Option<DockerIgnore>,
    ) -> Result<Self> {
        let mut dofigen = Self::default();

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

        let mut stage_name: Option<String> = None;
        let mut stage: Option<Stage> = None;
        let mut builders: HashMap<String, Stage> = HashMap::new();

        for line in instructions {
            if let DockerFileLine::Instruction(instruction) = line {
                let command = instruction.command.to_uppercase();
                match command.as_str() {
                    "FROM" => {
                        if let Some(previous_stage) = stage {
                            builders.insert(
                                stage_name.unwrap_or(format!("builder-{}", builders.len())),
                                previous_stage,
                            );
                        }
                        let mut parts = instruction.content.split(" as ");
                        let from_name = parts
                            .next()
                            .ok_or(Error::Custom(format!(
                                "No content FROM found in line {:?}",
                                line
                            )))?
                            .to_string();
                        let from: FromContextPatch = if from_name == "scratch" {
                            FromContextPatch::FromContext(None)
                        } else if builders.contains_key(&from_name) {
                            FromContextPatch::FromBuilder(from_name)
                        } else {
                            FromContextPatch::FromImage(from_name.parse()?)
                        };
                        stage_name = instruction
                            .content
                            .split(" as ")
                            .last()
                            .map(|s| s.to_string());

                        stage = Some(Stage {
                            from: from.into(),
                            ..Default::default()
                        });
                    }
                    c => {
                        return Err(Error::Custom(format!(
                            "Unsupported instruction: {c} in line: {line:?}"
                        )));
                    }
                }
            }
        }

        if !builders.is_empty() {
            dofigen.builders = builders;
        }

        dofigen.stage = stage.ok_or(Error::Custom("No FROM instruction found".to_string()))?;

        Ok(dofigen.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DockerfileInsctruction, FromContext, ImageName};
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
                    lines: vec![DockerFileLine::Instruction(DockerfileInsctruction {
                        command: "FROM".to_string(),
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    })],
                },
                Some(dockerignore),
            );

            assert!(result.is_ok());
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
                    lines: vec![DockerFileLine::Instruction(DockerfileInsctruction {
                        command: "FROM".to_string(),
                        content: "ubuntu:25.04".to_string(),
                        options: vec![],
                    })],
                },
                Some(dockerignore),
            );

            assert!(result.is_ok());
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
                lines: vec![DockerFileLine::Instruction(DockerfileInsctruction {
                    command: "FROM".to_string(),
                    content: "ubuntu:25.04".to_string(),
                    options: vec![],
                })],
            };

            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            assert!(result.is_ok());
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
                    DockerFileLine::Instruction(DockerfileInsctruction {
                        command: "FROM".to_string(),
                        content: "ubuntu:25.04 as builder".to_string(),
                        options: vec![],
                    }),
                    DockerFileLine::Instruction(DockerfileInsctruction {
                        command: "FROM".to_string(),
                        content: "scratch".to_string(),
                        options: vec![],
                    }),
                ],
            };

            let dockerignore = None;

            let result = Dofigen::from_dockerfile(dockerfile, dockerignore);

            assert!(result.is_ok());
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
}

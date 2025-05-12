use std::{str::FromStr, vec};

use regex::Regex;
use serde::Deserialize;

use crate::{generator::*, Error};

macro_rules! escaped_newline {
    () => {
        r"\\\r?\n"
    };
}

macro_rules! whitespace_or_escaped_newline {
    () => {
        concat!(r"[^\S\r\n]|", escaped_newline!())
    };
}
macro_rules! whitespace_regex {
    () => {
        concat!(r"(?:", whitespace_or_escaped_newline!(), r")+")
    };
}
macro_rules! option_regex {
    () => {
        concat!(
            r"(?<option>--(?<name>\w+)(?:=(?<value>[^\s\\]+))?",
            whitespace_regex!(),
            r")"
        )
    };
}
const DOCKERFILE_LINE_REGEX: &str = concat!(
    r"(?:",
    // Instruction
    r"[^\S\r\n]*",
    r"(?<command>[A-Z]+)",
    whitespace_regex!(),
    r"(?<options>",
    option_regex!(),
    r"*)",
    r"(?:",
    // TODO: manage more complex heredocs: https://docs.docker.com/reference/dockerfile/#here-documents
    r#"<<-?(?:EOF|"EOF")\r?\n(?<eof_content>(?:.|\r?\n)+)(?:EOF|"EOF")"#,
    r"|(?<content>(?:",
    escaped_newline!(),
    r"|.)+)",
    r")",
    // Blank lines
    r"|[^\S\r\n]*",
    // Comments
    r"|[^\S\r\n]*# ?(?<comment>.+)",
    // Empty lines
    r")(?:\r?\n|$)",
);

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum DockerFileCommand {
    FROM,
    ARG,
    LABEL,
    // Deprecated
    MAINTAINER,
    RUN,
    COPY,
    ADD,
    WORKDIR,
    ENV,
    EXPOSE,
    USER,
    VOLUME,
    HEALTHCHECK,
    CMD,
    ENTRYPOINT,
    Unknown(String),
}

impl ToString for DockerFileCommand {
    fn to_string(&self) -> String {
        match self {
            DockerFileCommand::FROM => "FROM".to_string(),
            DockerFileCommand::LABEL => "LABEL".to_string(),
            DockerFileCommand::MAINTAINER => "MAINTAINER".to_string(),
            DockerFileCommand::ARG => "ARG".to_string(),
            DockerFileCommand::RUN => "RUN".to_string(),
            DockerFileCommand::COPY => "COPY".to_string(),
            DockerFileCommand::ADD => "ADD".to_string(),
            DockerFileCommand::WORKDIR => "WORKDIR".to_string(),
            DockerFileCommand::ENV => "ENV".to_string(),
            DockerFileCommand::EXPOSE => "EXPOSE".to_string(),
            DockerFileCommand::USER => "USER".to_string(),
            DockerFileCommand::VOLUME => "VOLUME".to_string(),
            DockerFileCommand::HEALTHCHECK => "HEALTHCHECK".to_string(),
            DockerFileCommand::CMD => "CMD".to_string(),
            DockerFileCommand::ENTRYPOINT => "ENTRYPOINT".to_string(),
            DockerFileCommand::Unknown(command) => command.clone(),
        }
    }
}

impl FromStr for DockerFileCommand {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_uppercase().as_str() {
            "FROM" => Ok(DockerFileCommand::FROM),
            "ARG" => Ok(DockerFileCommand::ARG),
            "LABEL" => Ok(DockerFileCommand::LABEL),
            "MAINTAINER" => Ok(DockerFileCommand::MAINTAINER),
            "RUN" => Ok(DockerFileCommand::RUN),
            "COPY" => Ok(DockerFileCommand::COPY),
            "ADD" => Ok(DockerFileCommand::ADD),
            "WORKDIR" => Ok(DockerFileCommand::WORKDIR),
            "ENV" => Ok(DockerFileCommand::ENV),
            "EXPOSE" => Ok(DockerFileCommand::EXPOSE),
            "USER" => Ok(DockerFileCommand::USER),
            "VOLUME" => Ok(DockerFileCommand::VOLUME),
            "HEALTHCHECK" => Ok(DockerFileCommand::HEALTHCHECK),
            "CMD" => Ok(DockerFileCommand::CMD),
            "ENTRYPOINT" => Ok(DockerFileCommand::ENTRYPOINT),
            _ => Ok(DockerFileCommand::Unknown(string.into())),
        }
    }
}

pub struct DockerFile {
    pub lines: Vec<DockerFileLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DockerFileLine {
    Instruction(DockerFileInsctruction),
    Comment(String),
    Empty,
}

pub struct DockerIgnore {
    pub lines: Vec<DockerIgnoreLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DockerIgnoreLine {
    Pattern(String),
    NegatePattern(String),
    Comment(String),
    Empty,
}

pub trait DockerfileContent {
    fn generate_content(&self) -> String;
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockerFileInsctruction {
    pub command: DockerFileCommand,
    pub content: String,
    pub options: Vec<InstructionOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionOption {
    Flag(String),
    WithValue(String, String),
    WithOptions(String, Vec<InstructionOptionOption>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstructionOptionOption {
    pub name: String,
    pub value: Option<String>,
}

impl InstructionOptionOption {
    pub fn new(name: &str, value: String) -> Self {
        Self {
            name: name.into(),
            value: Some(value.into()),
        }
    }

    pub fn new_flag(name: &str) -> Self {
        Self {
            name: name.into(),
            value: None,
        }
    }
}

impl DockerfileContent for DockerFileLine {
    fn generate_content(&self) -> String {
        match self {
            DockerFileLine::Instruction(instruction) => instruction.generate_content(),
            DockerFileLine::Comment(comment) => comment
                .lines()
                .map(|l| format!("# {}", l))
                .collect::<Vec<String>>()
                .join("\n"),
            DockerFileLine::Empty => String::new(),
        }
    }
}

impl DockerfileContent for DockerFileInsctruction {
    fn generate_content(&self) -> String {
        let separator = if !self.options.is_empty() || self.content.contains("\\\n") {
            LINE_SEPARATOR
        } else {
            " "
        };
        let mut content = vec![self.command.to_string()];
        for option in &self.options {
            content.push(option.generate_content());
        }
        content.push(self.content.clone());
        content.join(separator)
    }
}

impl DockerfileContent for InstructionOption {
    fn generate_content(&self) -> String {
        match self {
            InstructionOption::Flag(name) => format!("--{}", name),
            InstructionOption::WithValue(name, value) => format!("--{}={}", name, value),
            InstructionOption::WithOptions(name, options) => format!(
                "--{}={}",
                name,
                options
                    .iter()
                    .map(|o| o.generate_content())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
        }
    }
}

impl DockerfileContent for InstructionOptionOption {
    fn generate_content(&self) -> String {
        if let Some(value) = &self.value {
            if value.contains(" ") || value.contains(",") || value.contains("=") {
                format!("{}='{}'", self.name, value)
            } else {
                format!("{}={}", self.name, value)
            }
        } else {
            self.name.clone()
        }
    }
}

impl FromStr for DockerFile {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut lines = vec![];
        let regex = Regex::new(DOCKERFILE_LINE_REGEX).expect("Failed to compile regex");
        let option_content_regex = Regex::new(option_regex!()).expect("Failed to compile regex");
        println!("Regex: /{}/", regex);

        for m in regex.find_iter(string) {
            let m = m.as_str();
            let captures = regex.captures(m).unwrap();
            if let Some(command) = captures.name("command") {
                let command = command.as_str();
                let content = captures
                    .name("content")
                    .or_else(|| captures.name("eof_content"))
                    .map(|c| c.as_str())
                    .unwrap_or_default()
                    .to_string();
                let options = captures
                    .name("options")
                    .map(|o| {
                        option_content_regex
                            .find_iter(o.as_str())
                            .map(|option| {
                                let option = option.as_str();
                                let captures = option_content_regex.captures(option).unwrap();
                                let name = captures.name("name").unwrap().as_str();
                                let value =
                                    captures.name("value").map(|v| v.as_str()).unwrap_or("");
                                if value.is_empty() {
                                    InstructionOption::Flag(name.to_string())
                                } else {
                                    InstructionOption::WithValue(
                                        name.to_string(),
                                        value.to_string(),
                                    )
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                lines.push(DockerFileLine::Instruction(DockerFileInsctruction {
                    command: command.parse()?,
                    content,
                    options,
                }));
            } else if m.trim().is_empty() {
                lines.push(DockerFileLine::Empty);
            } else if let Some(comment) = captures.name("comment") {
                lines.push(DockerFileLine::Comment(comment.as_str().to_string()));
            }
        }
        Ok(Self { lines })
    }
}

impl ToString for DockerFile {
    fn to_string(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.generate_content())
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl FromStr for DockerIgnore {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            lines: string
                .lines()
                .map(|line| {
                    let line = line.trim();
                    if line.is_empty() {
                        DockerIgnoreLine::Empty
                    } else if line.starts_with('#') {
                        DockerIgnoreLine::Comment(line[1..].trim().to_string())
                    } else if line.starts_with('!') {
                        DockerIgnoreLine::NegatePattern(line[1..].trim().to_string())
                    } else {
                        DockerIgnoreLine::Pattern(line.to_string())
                    }
                })
                .collect(),
        })
    }
}

impl ToString for DockerIgnore {
    fn to_string(&self) -> String {
        self.lines
            .iter()
            .map(|line| match line {
                DockerIgnoreLine::Pattern(pattern) => pattern.clone(),
                DockerIgnoreLine::NegatePattern(pattern) => format!("!{}", pattern),
                DockerIgnoreLine::Comment(comment) => format!("# {}", comment),
                DockerIgnoreLine::Empty => String::new(),
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions_sorted::assert_eq_sorted;

    use super::*;

    mod generate {
        use super::*;

        #[test]
        fn instruction() {
            let instruction = DockerFileInsctruction {
                command: DockerFileCommand::RUN,
                content: "echo 'Hello, World!'".into(),
                options: vec![
                    InstructionOption::Flag("arg1".into()),
                    InstructionOption::WithValue("arg2".into(), "value2".into()),
                ],
            };
            assert_eq_sorted!(
                instruction.generate_content(),
                "RUN \\\n    --arg1 \\\n    --arg2=value2 \\\n    echo 'Hello, World!'"
            );
        }

        #[test]
        fn comment() {
            let comment = DockerFileLine::Comment("This is a comment".into());
            assert_eq_sorted!(comment.generate_content(), "# This is a comment");
        }

        #[test]
        fn empty() {
            let empty = DockerFileLine::Empty;
            assert_eq_sorted!(empty.generate_content(), "");
        }

        #[test]
        fn name_only_option() {
            let option = InstructionOption::Flag("arg1".into());
            assert_eq_sorted!(option.generate_content(), "--arg1");
        }

        #[test]
        fn with_value_option() {
            let option = InstructionOption::WithValue("arg1".into(), "value1".into());
            assert_eq_sorted!(option.generate_content(), "--arg1=value1");
        }

        #[test]
        fn with_options_option() {
            let sub_option1 = InstructionOptionOption::new("sub_arg1", "sub_value1".into());
            let sub_option2 = InstructionOptionOption::new("sub_arg2", "sub_value2".into());
            let options = vec![sub_option1, sub_option2];
            let option = InstructionOption::WithOptions("arg1".into(), options);
            let expected = "--arg1=sub_arg1=sub_value1,sub_arg2=sub_value2";
            assert_eq_sorted!(option.generate_content(), expected);
        }

        #[test]
        fn instruction_option_option() {
            let option = InstructionOptionOption::new("arg1", "value1".into());
            let expected = "arg1=value1";
            assert_eq_sorted!(option.generate_content(), expected);
        }
    }

    mod parse {
        use super::*;

        #[test]
        fn simple() {
            let dockerfile: DockerFile = r#"FROM alpine:3.11 as builder
RUN echo "hello world" > /hello-world
# This is a comment

FROM scratch
COPY --from=builder /hello-world /hello-world
"#
            .parse()
            .unwrap();

            let lines = dockerfile.lines;
            assert_eq!(lines.len(), 6);

            assert_eq_sorted!(
                lines[0],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::FROM,
                    content: "alpine:3.11 as builder".to_string(),
                    options: vec![]
                })
            );
            assert_eq_sorted!(
                lines[1],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::RUN,
                    content: "echo \"hello world\" > /hello-world".to_string(),
                    options: vec![]
                })
            );
            assert_eq_sorted!(
                lines[2],
                DockerFileLine::Comment("This is a comment".to_string())
            );
            assert_eq!(lines[3], DockerFileLine::Empty);
            assert_eq_sorted!(
                lines[4],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::FROM,
                    content: "scratch".to_string(),
                    options: vec![]
                })
            );
            assert_eq_sorted!(
                lines[5],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::COPY,
                    content: "/hello-world /hello-world".to_string(),
                    options: vec![InstructionOption::WithValue(
                        "from".to_string(),
                        "builder".to_string()
                    )]
                })
            );
        }

        #[test]
        fn args() {
            let dockerfile: DockerFile = r#"FROM alpine:3.11 as builder
ARG arg1 \
    arg2=value2
ARG arg3=3
"#
            .parse()
            .unwrap();

            let lines = dockerfile.lines;
            assert_eq!(lines.len(), 3);

            assert_eq_sorted!(
                lines[0],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::FROM,
                    content: "alpine:3.11 as builder".to_string(),
                    options: vec![]
                })
            );
            assert_eq_sorted!(
                lines[1],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::ARG,
                    content: "arg1 \\\n    arg2=value2".to_string(),
                    options: vec![]
                })
            );
            assert_eq_sorted!(
                lines[2],
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::ARG,
                    content: "arg3=3".to_string(),
                    options: vec![]
                })
            );
        }
    }
}

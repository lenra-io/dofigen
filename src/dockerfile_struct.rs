use std::vec;

use crate::generator::LINE_SEPARATOR;

pub trait DockerfileContent {
    fn generate_content(&self) -> String;
}

#[derive(Debug, Clone, PartialEq)]
pub enum DockerfileLine {
    Instruction(DockerfileInsctruction),
    Comment(String),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockerfileInsctruction {
    pub command: String,
    pub content: String,
    pub options: Vec<InstructionOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionOption {
    NameOnly(String),
    WithValue(String, String),
    WithOptions(String, Vec<InstructionOptionOption>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstructionOptionOption {
    pub name: String,
    pub value: String,
}

impl InstructionOptionOption {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

impl DockerfileContent for DockerfileLine {
    fn generate_content(&self) -> String {
        match self {
            DockerfileLine::Instruction(instruction) => instruction.generate_content(),
            DockerfileLine::Comment(comment) => comment
                .lines()
                .map(|l| format!("# {}", l))
                .collect::<Vec<String>>()
                .join("\n"),
            DockerfileLine::Empty => "".into(),
        }
    }
}

impl DockerfileContent for DockerfileInsctruction {
    fn generate_content(&self) -> String {
        let separator = if !self.options.is_empty() || self.content.contains("\\\n") {
            LINE_SEPARATOR
        } else {
            " "
        };
        let mut content = vec![self.command.clone()];
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
            InstructionOption::NameOnly(name) => format!("--{}", name),
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
        if self.value.contains(" ") {
            format!("{}='{}'", self.name, self.value)
        } else {
            format!("{}={}", self.name, self.value)
        }
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions_sorted::assert_eq_sorted;

    use super::*;

    #[test]
    fn test_generate_content_instruction() {
        let instruction = DockerfileInsctruction {
            command: "RUN".into(),
            content: "echo 'Hello, World!'".into(),
            options: vec![
                InstructionOption::NameOnly("arg1".into()),
                InstructionOption::WithValue("arg2".into(), "value2".into()),
            ],
        };
        assert_eq_sorted!(
            instruction.generate_content(),
            "RUN \\\n    --arg1 \\\n    --arg2=value2 \\\n    echo 'Hello, World!'"
        );
    }

    #[test]
    fn test_generate_content_comment() {
        let comment = DockerfileLine::Comment("This is a comment".into());
        assert_eq_sorted!(comment.generate_content(), "# This is a comment");
    }

    #[test]
    fn test_generate_content_empty() {
        let empty = DockerfileLine::Empty;
        assert_eq_sorted!(empty.generate_content(), "");
    }

    #[test]
    fn test_generate_content_name_only_option() {
        let option = InstructionOption::NameOnly("arg1".into());
        assert_eq_sorted!(option.generate_content(), "--arg1");
    }

    #[test]
    fn test_generate_content_with_value_option() {
        let option = InstructionOption::WithValue("arg1".into(), "value1".into());
        assert_eq_sorted!(option.generate_content(), "--arg1=value1");
    }

    #[test]
    fn test_generate_content_with_options_option() {
        let sub_option1 = InstructionOptionOption::new("sub_arg1", "sub_value1");
        let sub_option2 = InstructionOptionOption::new("sub_arg2", "sub_value2");
        let options = vec![sub_option1, sub_option2];
        let option = InstructionOption::WithOptions("arg1".into(), options);
        let expected = "--arg1=sub_arg1=sub_value1,sub_arg2=sub_value2";
        assert_eq_sorted!(option.generate_content(), expected);
    }

    #[test]
    fn test_generate_content_instruction_option_option() {
        let option = InstructionOptionOption::new("arg1", "value1");
        let expected = "arg1=value1";
        assert_eq_sorted!(option.generate_content(), expected);
    }
}

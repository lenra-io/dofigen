use std::vec;

use crate::generator::LINE_SEPARATOR;

pub trait DockerfileContent {
    fn generate_content(&self) -> String;
}

pub enum DockerfileLine {
    Instruction(DockerfileInsctruction),
    Comment(String),
    Empty,
}

pub struct DockerfileInsctruction {
    pub command: String,
    pub content: String,
    pub options: Vec<InstructionOption>,
}

pub enum InstructionOption {
    NameOnly(String),
    WithValue(String, String),
    WithOptions(String, Vec<InstructionOptionOption>),
}

pub struct InstructionOptionOption {
    pub name: String,
    pub value: String,
}

impl InstructionOptionOption {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
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
            DockerfileLine::Empty => "".to_string(),
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
        format!("{}={}", self.name, self.value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_content_instruction() {
        let instruction = DockerfileInsctruction {
            command: "RUN".to_string(),
            content: "echo 'Hello, World!'".to_string(),
            options: vec![
                InstructionOption::NameOnly("arg1".to_string()),
                InstructionOption::WithValue("arg2".to_string(), "value2".to_string()),
            ],
        };
        assert_eq!(
            instruction.generate_content(),
            "RUN \\\n    --arg1 \\\n    --arg2=value2 \\\n    echo 'Hello, World!'"
        );
    }

    #[test]
    fn test_generate_content_comment() {
        let comment = DockerfileLine::Comment("This is a comment".to_string());
        assert_eq!(comment.generate_content(), "# This is a comment");
    }

    #[test]
    fn test_generate_content_empty() {
        let empty = DockerfileLine::Empty;
        assert_eq!(empty.generate_content(), "");
    }

    #[test]
    fn test_generate_content_name_only_option() {
        let option = InstructionOption::NameOnly("arg1".to_string());
        assert_eq!(option.generate_content(), "--arg1");
    }

    #[test]
    fn test_generate_content_with_value_option() {
        let option = InstructionOption::WithValue("arg1".to_string(), "value1".to_string());
        assert_eq!(option.generate_content(), "--arg1=value1");
    }

    #[test]
    fn test_generate_content_with_options_option() {
        let sub_option1 = InstructionOptionOption::new("sub_arg1", "sub_value1");
        let sub_option2 = InstructionOptionOption::new("sub_arg2", "sub_value2");
        let options = vec![sub_option1, sub_option2];
        let option = InstructionOption::WithOptions("arg1".to_string(), options);
        let expected = "--arg1=sub_arg1=sub_value1,sub_arg2=sub_value2";
        assert_eq!(option.generate_content(), expected);
    }

    #[test]
    fn test_generate_content_instruction_option_option() {
        let option = InstructionOptionOption::new("arg1", "value1");
        let expected = "arg1=value1";
        assert_eq!(option.generate_content(), expected);
    }
}

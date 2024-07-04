use std::vec;

pub trait DockerfileContent {
    fn generate_content(&self) -> String;
}

pub enum DockerfileLine {
    Instruction(DockerfileInsctruction),
    Comment(String),
    EmptyLine,
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
            DockerfileLine::EmptyLine => "".to_string(),
        }
    }
}

impl DockerfileContent for DockerfileInsctruction {
    fn generate_content(&self) -> String {
        let mut content = vec![self.command.clone()];
        for option in &self.options {
            content.push(option.generate_content());
        }
        content.push(self.content.clone());
        let separator = if self.options.is_empty() {
            " "
        } else {
            " \\\n    "
        };
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

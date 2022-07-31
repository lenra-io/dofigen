pub mod impls;
pub mod structs;
use impls::Stage;
use std::io::Read;
use structs::Image;

pub fn from_yaml(input: String) -> Image {
    serde_yaml::from_str(&input).unwrap()
}

pub fn from_yaml_reader<R: Read>(reader: R) -> Image {
    serde_yaml::from_reader(reader).unwrap()
}

pub fn from_json(input: String) -> Image {
    serde_json::from_str(&input).unwrap()
}

pub fn from_json_reader<R: Read>(reader: R) -> Image {
    serde_json::from_reader(reader).unwrap()
}

pub fn generate_dockerfile(image: Image) -> String {
    let mut buffer: String = String::new();

    buffer.push_str("# syntax=docker/dockerfile:1.4\n");
    let mut previous_builders = Vec::new();
    if let Some(ref builders) = image.builders {
        builders.iter().for_each(|builder| {
            builder
                .generate(&mut buffer, &mut previous_builders)
                .unwrap()
        });
    }
    image.generate(&mut buffer, &mut previous_builders).unwrap();
    buffer
}

pub fn generate_dockerignore(image: Image) -> String {
    if let Some(ignore) = image.ignores {
        ignore.join("\n")
    }
    else {
        String::from("")
    }
}

#[cfg(test)]
mod tests {

    use crate::structs::{Artifact, Builder};

    use super::*;

    #[test]
    fn parse_basic_yaml() {
        let yaml = "
        image: scratch
        ";
        let image: Image = from_yaml(yaml.to_string());
        assert_eq!(
            image,
            Image {
                image: String::from("scratch"),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parse_yaml_with_builders() {
        let yaml = "
        builders:
          - name: builder
            image: ekidd/rust-musl-builder
            adds:
              - \"*\"
            script:
              - cargo build --release
        image: scratch
        artifacts:
          - builder: builder
            source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
            destination: /app
        ";
        let image: Image = from_yaml(yaml.to_string());
        assert_eq!(
            image,
            Image {
                builders: Some(Vec::from([Builder {
                    name: Some(String::from("builder")),
                    image: String::from("ekidd/rust-musl-builder"),
                    adds: Some(Vec::from([String::from("*")])),
                    script: Some(Vec::from([String::from("cargo build --release")])),
                    ..Default::default()
                }])),
                image: String::from("scratch"),
                artifacts: Some(Vec::from([Artifact {
                    builder: String::from("builder"),
                    source: String::from(
                        "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
                    ),
                    destination: String::from("/app"),
                    ..Default::default()
                }])),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parse_basic_json() {
        let json = r#"
        {
            "image": "scratch"
        }"#;
        let image: Image = from_json(json.to_string());
        assert_eq!(
            image,
            Image {
                image: String::from("scratch"),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parse_json_with_builders() {
        let json = r#"
        {
            "builders": [
                {
                    "name": "builder",
                    "image": "ekidd/rust-musl-builder",
                    "adds": [
                        "*"
                    ],
                    "script": [
                        "cargo build --release"
                    ]
                }
            ],
            "image": "scratch",
            "artifacts": [
                {
                    "builder": "builder",
                    "source": "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust",
                    "destination": "/app"
                }
            ]
        }"#;
        let image: Image = from_json(json.to_string());
        assert_eq!(
            image,
            Image {
                builders: Some(Vec::from([Builder {
                    name: Some(String::from("builder")),
                    image: String::from("ekidd/rust-musl-builder"),
                    adds: Some(Vec::from([String::from("*")])),
                    script: Some(Vec::from([String::from("cargo build --release")])),
                    ..Default::default()
                }])),
                image: String::from("scratch"),
                artifacts: Some(Vec::from([Artifact {
                    builder: String::from("builder"),
                    source: String::from(
                        "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
                    ),
                    destination: String::from("/app"),
                    ..Default::default()
                }])),
                ..Default::default()
            }
        );
    }
}

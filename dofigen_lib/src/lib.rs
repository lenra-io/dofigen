use crate::structs::Stage;

mod structs;
use structs::Image;

pub fn parse_from_json(input: String) -> Image {
    serde_json::from_str(&input).unwrap()
}

pub fn generate(image: Image) -> String {
    let mut buffer: String = String::new();

    buffer.push_str("# syntax=docker/dockerfile:1.4\n");
    let mut previous_builders = Vec::new();
    if let Some(ref builders) = image.builders() {
        builders
            .iter()
            .for_each(|builder| builder.generate(&mut buffer, &mut previous_builders));
    }
    image.generate(&mut buffer, &mut previous_builders);
    buffer
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parse_basic_json() {
        let json = r#"
        {
            "image": "scratch"
        }"#;
        let image: Image = parse_from_json(json.to_string());
        assert_eq!(image, Image {
            image: String::from("scratch"),
            envs: None,
            root_script: None,
            script: None,
            workdir: None,
            add: None,
            artifacts: None,
            builders: None,
            ignore: None,
        });
    }

    #[ignore]
    #[test]
    fn parse_json_with_builders() {
        let json = r#"
        {
            "builders": [
                {
                    "name": "builder",
                    "image": "ekidd/rust-musl-builder",
                    "add": [
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
        let image: Image = parse_from_json(json.to_string());
        assert_eq!(image, Image {
            image: String::from("scratch"),
            envs: None,
            root_script: None,
            script: None,
            workdir: None,
            add: None,
            artifacts: None,
            builders: None,
            ignore: None,
        });
    }
}

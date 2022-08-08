//! # dofigen_lib
//!
//! `dofigen_lib` help creating Dockerfile with a simplified structure and made to cache the build with Buildkit.
//! You also can parse the structure from YAML or JSON.

pub mod impls;
pub mod structs;
use impls::Stage;
use std::io::Read;
use structs::Image;

/// Parse an Image from a YAML string.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_yaml;
/// use dofigen_lib::structs::Image;
///
/// let yaml = "
/// image: scratch
/// ";
/// let image: Image = from_yaml(yaml.to_string());
/// assert_eq!(
///     image,
///     Image {
///         image: String::from("scratch"),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_yaml;
/// use dofigen_lib::structs::{Image, Builder, Artifact};
///
/// let yaml = r#"
/// builders:
///   - name: builder
///     image: ekidd/rust-musl-builder
///     adds:
///       - "*"
///     script:
///       - cargo build --release
/// image: scratch
/// artifacts:
///   - builder: builder
///     source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
///     destination: /app
/// "#;
/// let image: Image = from_yaml(yaml.to_string());
/// assert_eq!(
///     image,
///     Image {
///         builders: Some(Vec::from([Builder {
///             name: Some(String::from("builder")),
///             image: String::from("ekidd/rust-musl-builder"),
///             adds: Some(Vec::from([String::from("*")])),
///             script: Some(Vec::from([String::from("cargo build --release")])),
///             ..Default::default()
///         }])),
///         image: String::from("scratch"),
///         artifacts: Some(Vec::from([Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             destination: String::from("/app"),
///             ..Default::default()
///         }])),
///         ..Default::default()
///     }
/// );
/// ```
pub fn from_yaml(input: String) -> Image {
    serde_yaml::from_str(&input).unwrap()
}

/// Parse an Image from a reader of YAML content.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_yaml_reader;
/// use dofigen_lib::structs::Image;
///
/// let yaml = "
/// image: scratch
/// ";
/// let image: Image = from_yaml_reader(yaml.as_bytes());
/// assert_eq!(
///     image,
///     Image {
///         image: String::from("scratch"),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_yaml_reader;
/// use dofigen_lib::structs::{Image, Builder, Artifact};
///
/// let yaml = r#"
/// builders:
///   - name: builder
///     image: ekidd/rust-musl-builder
///     adds:
///       - "*"
///     script:
///       - cargo build --release
/// image: scratch
/// artifacts:
///   - builder: builder
///     source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
///     destination: /app
/// "#;
/// let image: Image = from_yaml_reader(yaml.as_bytes());
/// assert_eq!(
///     image,
///     Image {
///         builders: Some(Vec::from([Builder {
///             name: Some(String::from("builder")),
///             image: String::from("ekidd/rust-musl-builder"),
///             adds: Some(Vec::from([String::from("*")])),
///             script: Some(Vec::from([String::from("cargo build --release")])),
///             ..Default::default()
///         }])),
///         image: String::from("scratch"),
///         artifacts: Some(Vec::from([Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             destination: String::from("/app"),
///             ..Default::default()
///         }])),
///         ..Default::default()
///     }
/// );
/// ```
pub fn from_yaml_reader<R: Read>(reader: R) -> Image {
    serde_yaml::from_reader(reader).unwrap()
}

/// Parse an Image from a JSON string.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_json;
/// use dofigen_lib::structs::Image;
///
/// let json = r#"{ "image": "scratch" }"#;
/// let image: Image = from_json(json.to_string());
/// assert_eq!(
///     image,
///     Image {
///         image: String::from("scratch"),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_json;
/// use dofigen_lib::structs::{Image, Builder, Artifact};
///
/// let json = r#"
/// {
///     "builders": [
///         {
///             "name": "builder",
///             "image": "ekidd/rust-musl-builder",
///             "adds": [
///                 "*"
///             ],
///             "script": [
///                 "cargo build --release"
///             ]
///         }
///     ],
///     "image": "scratch",
///     "artifacts": [
///         {
///             "builder": "builder",
///             "source": "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust",
///             "destination": "/app"
///         }
///     ]
/// }"#;
///
/// let image: Image = from_json(json.to_string());
/// assert_eq!(
///     image,
///     Image {
///         builders: Some(Vec::from([Builder {
///             name: Some(String::from("builder")),
///             image: String::from("ekidd/rust-musl-builder"),
///             adds: Some(Vec::from([String::from("*")])),
///             script: Some(Vec::from([String::from("cargo build --release")])),
///             ..Default::default()
///         }])),
///         image: String::from("scratch"),
///         artifacts: Some(Vec::from([Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             destination: String::from("/app"),
///             ..Default::default()
///         }])),
///         ..Default::default()
///     }
/// );
/// ```
pub fn from_json(input: String) -> Image {
    serde_json::from_str(&input).unwrap()
}

/// Parse an Image from a reader of YAML content.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_json_reader;
/// use dofigen_lib::structs::Image;
///
/// let json = r#"{ "image": "scratch" }"#;
/// let image: Image = from_json_reader(json.as_bytes());
/// assert_eq!(
///     image,
///     Image {
///         image: String::from("scratch"),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::from_json_reader;
/// use dofigen_lib::structs::{Image, Builder, Artifact};
///
/// let json = r#"
/// {
///     "builders": [
///         {
///             "name": "builder",
///             "image": "ekidd/rust-musl-builder",
///             "adds": [
///                 "*"
///             ],
///             "script": [
///                 "cargo build --release"
///             ]
///         }
///     ],
///     "image": "scratch",
///     "artifacts": [
///         {
///             "builder": "builder",
///             "source": "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust",
///             "destination": "/app"
///         }
///     ]
/// }"#;
///
/// let image: Image = from_json_reader(json.as_bytes());
/// assert_eq!(
///     image,
///     Image {
///         builders: Some(Vec::from([Builder {
///             name: Some(String::from("builder")),
///             image: String::from("ekidd/rust-musl-builder"),
///             adds: Some(Vec::from([String::from("*")])),
///             script: Some(Vec::from([String::from("cargo build --release")])),
///             ..Default::default()
///         }])),
///         image: String::from("scratch"),
///         artifacts: Some(Vec::from([Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             destination: String::from("/app"),
///             ..Default::default()
///         }])),
///         ..Default::default()
///     }
/// );
/// ```
pub fn from_json_reader<R: Read>(reader: R) -> Image {
    serde_json::from_reader(reader).unwrap()
}

/// Generates the Dockerfile content from an Image.
///
/// # Examples
///
/// ```
/// use dofigen_lib::generate_dockerfile;
/// use dofigen_lib::structs::Image;
///
/// let image = Image {
///     image: String::from("scratch"),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerfile(&image);
/// assert_eq!(
///     dockerfile,
///     "# syntax=docker/dockerfile:1.4\n\n# runtime\nFROM scratch as runtime\n"
/// );
/// ```
pub fn generate_dockerfile(image: &Image) -> String {
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

/// Generates the .dockerignore file content from an Image.
///
/// # Examples
///
/// ```
/// use dofigen_lib::generate_dockerignore;
/// use dofigen_lib::structs::Image;
///
/// let image = Image {
///     image: String::from("scratch"),
///     ignores: Some(Vec::from([String::from("target")])),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq!(
///     dockerfile,
///     "target\n"
/// );
/// ```
pub fn generate_dockerignore(image: &Image) -> String {
    let mut content = if let Some(ignore) = image.ignores() {
        ignore.join("\n")
    } else {
        String::from("")
    };
    content.push_str("\n");
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_to_dockerfile() {
        let yaml = r#"
        image: scratch
        builders:
        - name: builder
          image: ekidd/rust-musl-builder
          user: rust
          adds: 
          - "."
          script:
          - cargo build --release
          cache:
          - /usr/local/cargo/registry
        - name: watchdog
          image: ghcr.io/openfaas/of-watchdog:0.9.6
        envs:
          fprocess: /app
        artifacts:
        - builder: builder
          source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
          destination: /app
        - builder: builder
          source: /fwatchdog
          destination: /fwatchdog
        ports:
        - 8080
        healthcheck:
          interval: 3s
          cmd: "[ -e /tmp/.lock ] || exit 1"
        cmd: ["/fwatchdog"]
        ignores:
        - target
        - test
        "#;

        let image: Image = from_yaml(yaml.to_string());
        let dockerfile: String = generate_dockerfile(&image);

        assert_eq!(
            dockerfile,
            r#"# syntax=docker/dockerfile:1.4

# builder
FROM ekidd/rust-musl-builder as builder
ADD --link . ./
USER rust
RUN \
    cargo build --release

# watchdog
FROM ghcr.io/openfaas/of-watchdog:0.9.6 as watchdog

# runtime
FROM scratch as runtime
ENV \
    fprocess="/app"
COPY --link --chown=1000:1000 --from=builder "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust" "/app"
COPY --link --chown=1000:1000 --from=builder "/fwatchdog" "/fwatchdog"
EXPOSE 8080
HEALTHCHECK --interval=3s CMD [ -e /tmp/.lock ] || exit 1
CMD ["/fwatchdog"]
"#
        );

        let dockerignore: String = generate_dockerignore(&image);

        assert_eq!(dockerignore, "target\ntest\n");
    }
}

//! # dofigen_lib
//!
//! `dofigen_lib` help creating Dockerfile with a simplified structure and made to cache the build with Buildkit.
//! You also can parse the structure from YAML or JSON.

mod errors;
mod runners;
mod stages;
mod structs;
pub use errors::*;
pub use stages::*;
use std::{fs, io::Read};
pub use structs::*;

/// Parse an Image from a string.
///
/// # Examples
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::{from, Image};
///
/// let yaml = "
/// image: scratch
/// ";
/// let image: Image = from(yaml.to_string()).unwrap();
/// assert_eq!(
///     image,
///     Image {
///         image: String::from("scratch"),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::{from, Image, Builder, Artifact};
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
/// let image: Image = from(yaml.to_string()).unwrap();
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
pub fn from(input: String) -> Result<Image> {
    serde_yaml::from_str(&input).map_err(|err| Error::DeserializeYaml(err))
}

/// Parse an Image from a reader.
///
/// # Examples
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::{from_reader, Image};
///
/// let yaml = "
/// image: scratch
/// ";
/// let image: Image = from_reader(yaml.as_bytes()).unwrap();
/// assert_eq!(
///     image,
///     Image {
///         image: String::from("scratch"),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::{from_reader, Image, Builder, Artifact};
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
/// let image: Image = from_reader(yaml.as_bytes()).unwrap();
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
pub fn from_reader<R: Read>(reader: R) -> Result<Image> {
    serde_yaml::from_reader(reader).map_err(|err| Error::DeserializeYaml(err))
}

/// Parse an Image from a YAML string.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::{from_yaml, Image};
///
/// let yaml = "
/// image: scratch
/// ";
/// let image: Image = from_yaml(yaml.to_string()).unwrap();
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
/// use dofigen_lib::{from_yaml, Image, Builder, Artifact};
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
/// let image: Image = from_yaml(yaml.to_string()).unwrap();
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
#[deprecated(
    since = "1.2.0",
    note = "The YAML reader can read both JSON and YAML. Should use 'from'"
)]
pub fn from_yaml(input: String) -> Result<Image> {
    serde_yaml::from_str(&input).map_err(|err| Error::DeserializeYaml(err))
}

/// Parse an Image from a reader of YAML content.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::{from_yaml_reader, Image};
///
/// let yaml = "
/// image: scratch
/// ";
/// let image: Image = from_yaml_reader(yaml.as_bytes()).unwrap();
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
/// use dofigen_lib::{from_yaml_reader, Image, Builder, Artifact};
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
/// let image: Image = from_yaml_reader(yaml.as_bytes()).unwrap();
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
#[deprecated(
    since = "1.2.0",
    note = "The YAML reader can read both JSON and YAML. Should use 'from_reader'"
)]
pub fn from_yaml_reader<R: Read>(reader: R) -> Result<Image> {
    from_reader(reader)
}

/// Parse an Image from a JSON string.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::{from_json, Image};
///
/// let json = r#"{ "image": "scratch" }"#;
/// let image: Image = from_json(json.to_string()).unwrap();
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
/// use dofigen_lib::{from_json, Image, Builder, Artifact};
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
/// let image: Image = from_json(json.to_string()).unwrap();
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
#[deprecated(
    since = "1.2.0",
    note = "The YAML reader can read both JSON and YAML. Should use 'from'"
)]
pub fn from_json(input: String) -> Result<Image> {
    serde_json::from_str(&input).map_err(|err| Error::DeserializeJson(err))
}

/// Parse an Image from a reader of YAML content.
///
/// # Examples
///
/// Basic YAML parsing
///
/// ```
/// use dofigen_lib::{from_json_reader, Image};
///
/// let json = r#"{ "image": "scratch" }"#;
/// let image: Image = from_json_reader(json.as_bytes()).unwrap();
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
/// use dofigen_lib::{from_json_reader, Image, Builder, Artifact};
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
/// let image: Image = from_json_reader(json.as_bytes()).unwrap();
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
#[deprecated(
    since = "1.2.0",
    note = "The YAML reader can read both JSON and YAML. Should use 'from_reader'"
)]
pub fn from_json_reader<R: Read>(reader: R) -> Result<Image> {
    serde_json::from_reader(reader).map_err(|err| Error::DeserializeJson(err))
}

/// Parse an Image from a YAML or JSON file path.
pub fn from_file_path(path: &std::path::PathBuf) -> Result<Image> {
    let file = fs::File::open(path).unwrap();
    match path.extension() {
        Some(os_str) => match os_str.to_str() {
            Some("yml" | "yaml" | "json") => {
                serde_yaml::from_reader(file).map_err(|err| Error::DeserializeYaml(err))
            }
            Some(ext) => Err(Error::Custom(format!(
                "Not managed Dofigen file extension {}",
                ext
            ))),
            None => Err(Error::Custom("The Dofigen file has no extension".into())),
        },
        None => Err(Error::Custom("The Dofigen file has no extension".into())),
    }
}

/// Generates the Dockerfile content from an Image.
///
/// # Examples
///
/// ```
/// use dofigen_lib::{generate_dockerfile, Image};
///
/// let image = Image {
///     image: String::from("scratch"),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerfile(&image);
/// assert_eq!(
///     dockerfile,
///     "# syntax=docker/dockerfile:1.4\n\n# runtime\nFROM scratch AS runtime\n"
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
/// use dofigen_lib::{generate_dockerignore, Image};
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
          caches:
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

        let image: Image = from(yaml.to_string()).unwrap();
        let dockerfile: String = generate_dockerfile(&image);

        assert_eq!(
            dockerfile,
            r#"# syntax=docker/dockerfile:1.4

# builder
FROM ekidd/rust-musl-builder AS builder
ADD --link . ./
USER rust
RUN \
    --mount=type=cache,sharing=locked,uid=1000,gid=1000,target=/usr/local/cargo/registry\
    cargo build --release

# watchdog
FROM ghcr.io/openfaas/of-watchdog:0.9.6 AS watchdog

# runtime
FROM scratch AS runtime
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

    #[test]
    fn using_dockerfile_overlap_aliases() {
        let yaml = r#"
builders:
- name: builder
  from: ekidd/rust-musl-builder
  adds:
  - "*"
  run:
  - cargo build --release
from: scratch
artifacts:
- builder: builder
  source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
  destination: /app
"#;
        let image: Image = from(yaml.to_string()).unwrap();
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
    fn combine_field_and_aliases() {
        let yaml = r#"
image: scratch
from: alpine
"#;
        let result = from(yaml.to_string());
        assert!(
            result.is_err(),
            "The parsing must fail since from and image are not compatible"
        );
    }

    #[test]
    fn fail_on_unknow_field() {
        let yaml = r#"
from: alpine
test: Fake value
"#;
        let result = from(yaml.to_string());
        assert!(
            result.is_err(),
            "The parsing must fail since 'test' is not a valid field"
        );

        // Check the error message
        let error = result.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Error while deserializing the YAML document: unknown field `test`, expected one of `from`, `image`, `user`, `workdir`, `env`, `envs`, `artifacts`, `add`, `adds`, `root`, `run`, `script`, `cache`, `caches`, `builders`, `ignore`, `ignores`, `entrypoint`, `cmd`, `ports`, `healthcheck` at line 3 column 1"
        );
    }

    #[test]
    fn manage_singular_aliases() -> Result<()> {
        let yaml = r#"
image: scratch
builders:
- name: builder
  image: ekidd/rust-musl-builder
  user: rust
  add: 
  - "."
  script:
  - cargo build --release
  cache:
  - /usr/local/cargo/registry
- name: watchdog
  image: ghcr.io/openfaas/of-watchdog:0.9.6
env:
  fprocess: /app
artifacts:
- builder: builder
  source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
  destination: /app
- builder: builder
  source: /fwatchdog
  target: /fwatchdog
ports:
- 8080
ignore:
- target
- test
"#;

        from(yaml.to_string())?;
        Ok(())
    }
}

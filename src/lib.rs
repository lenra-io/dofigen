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

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = env!("CARGO_PKG_REPOSITORY");

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
///         from: String::from("scratch"),
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
///             from: String::from("ekidd/rust-musl-builder"),
///             add: Some(Vec::from([String::from("*")])),
///             run: Some(Vec::from([String::from("cargo build --release")])),
///             ..Default::default()
///         }])),
///         from: String::from("scratch"),
///         artifacts: Some(Vec::from([Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             target: String::from("/app"),
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
///         from: String::from("scratch"),
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
///             from: String::from("ekidd/rust-musl-builder"),
///             add: Some(Vec::from([String::from("*")])),
///             run: Some(Vec::from([String::from("cargo build --release")])),
///             ..Default::default()
///         }])),
///         from: String::from("scratch"),
///         artifacts: Some(Vec::from([Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             target: String::from("/app"),
///             ..Default::default()
///         }])),
///         ..Default::default()
///     }
/// );
/// ```
pub fn from_reader<R: Read>(reader: R) -> Result<Image> {
    serde_yaml::from_reader(reader).map_err(|err| Error::DeserializeYaml(err))
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
///     from: String::from("scratch"),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerfile(&image);
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# https://github.com/lenra-io/dofigen\n\n# syntax=docker/dockerfile:1.4\n\n# runtime\nFROM scratch AS runtime\n"
/// );
/// ```
pub fn generate_dockerfile(image: &Image) -> String {
    let mut buffer: String = String::new();

    buffer.push_str("# This file is generated by Dofigen v");
    buffer.push_str(VERSION);
    buffer.push_str("\n");
    buffer.push_str("# ");
    buffer.push_str(REPO);
    buffer.push_str("\n\n");

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
/// ## Define the build context
///
/// ```
/// use dofigen_lib::{generate_dockerignore, Image};
///
/// let image = Image {
///     from: String::from("scratch"),
///     context: Some(Vec::from([String::from("/src")])),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# https://github.com/lenra-io/dofigen\n\n**\n!/src\n"
/// );
/// ```
///
/// ## Ignore a path
///
/// ```
/// use dofigen_lib::{generate_dockerignore, Image};
///
/// let image = Image {
///     from: String::from("scratch"),
///     ignore: Some(Vec::from([String::from("target")])),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# https://github.com/lenra-io/dofigen\n\ntarget\n"
/// );
/// ```
///
/// ## Define context ignoring a specific files
///
/// ```
/// use dofigen_lib::{generate_dockerignore, Image};
///
/// let image = Image {
///     from: String::from("scratch"),
///     context: Some(Vec::from([String::from("/src")])),
///     ignore: Some(Vec::from([String::from("/src/*.test.rs")])),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# https://github.com/lenra-io/dofigen\n\n**\n!/src\n/src/*.test.rs\n"
/// );
/// ```
pub fn generate_dockerignore(image: &Image) -> String {
    let mut content = String::new();

    content.push_str("# This file is generated by Dofigen v");
    content.push_str(VERSION);
    content.push_str("\n");
    content.push_str("# ");
    content.push_str(REPO);
    content.push_str("\n\n");

    if let Some(context) = image.context.clone() {
        content.push_str("**\n");
        context.iter().for_each(|path| {
            content.push_str("!");
            content.push_str(path);
            content.push_str("\n");
        });
    }
    if let Some(ignore) = image.ignore.clone() {
        ignore.iter().for_each(|path| {
            content.push_str(path);
            content.push_str("\n");
        });
    }
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
          - ls -al
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
            r#"# This file is generated by Dofigen v0.0.0
# https://github.com/lenra-io/dofigen

# syntax=docker/dockerfile:1.4

# builder
FROM ekidd/rust-musl-builder AS builder
ADD --link . ./
USER rust
RUN \
    --mount=type=cache,sharing=locked,uid=1000,gid=1000,target=/usr/local/cargo/registry \
    ls -al && \
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

        assert_eq!(dockerignore, "# This file is generated by Dofigen v0.0.0\n# https://github.com/lenra-io/dofigen\n\ntarget\ntest\n");
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
                    from: String::from("ekidd/rust-musl-builder"),
                    add: Some(Vec::from([String::from("*")])),
                    run: Some(Vec::from([String::from("cargo build --release")])),
                    ..Default::default()
                }])),
                from: String::from("scratch"),
                artifacts: Some(Vec::from([Artifact {
                    builder: String::from("builder"),
                    source: String::from(
                        "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
                    ),
                    target: String::from("/app"),
                    ..Default::default()
                }])),
                ..Default::default()
            }
        );
    }

    #[test]
    fn multiline_run_field() {
        let yaml = r#"
from: scratch
run:
  - |
    if [ "test" = "test" ]; then
      echo "Test"
    fi
"#;
        let image: Image = from(yaml.to_string()).unwrap();
        let dockerfile: String = generate_dockerfile(&image);

        assert_eq!(
            dockerfile,
            r#"# This file is generated by Dofigen v0.0.0
# https://github.com/lenra-io/dofigen

# syntax=docker/dockerfile:1.4

# runtime
FROM scratch AS runtime
RUN \
    if [ "test" = "test" ]; then \
      echo "Test" \
    fi
"#
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
        assert!(error.to_string().starts_with(
            "Error while deserializing the YAML document: unknown field `test`, expected one of "
        ),"Wrong error message");
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

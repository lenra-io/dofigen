//! # dofigen_lib
//!
//! `dofigen_lib` help creating Dockerfile with a simplified structure and made to cache the build with Buildkit.
//! You also can parse the structure from YAML or JSON.

mod deserialize_struct;
mod dockerfile_struct;
mod dofigen_struct;
mod errors;
#[cfg(feature = "permissive")]
mod from_str;
mod generator;
// mod merge;
mod script_runner;
#[cfg(feature = "permissive")]
mod serde_permissive;
mod stage;
use dockerfile_struct::{DockerfileContent, DockerfileLine};
pub use dofigen_struct::*;
pub use errors::*;
use generator::{DockerfileGenerator, GenerationContext};
#[cfg(feature = "json_schema")]
use schemars::schema_for;
pub use stage::*;
// pub use merge::*;
use std::{fs, io::Read};
// #[macro_use]
// extern crate pretty_assertions_sorted;
// use pretty_assertions_sorted::*;

pub const DOCKERFILE_VERSION: &str = "1.7";

const FILE_HEADER_LINES: [&str; 3] = [
    concat!(
        "# This file is generated by Dofigen v",
        env!("CARGO_PKG_VERSION")
    ),
    concat!("# See ", env!("CARGO_PKG_REPOSITORY")),
    "",
];

/// Parse an Image from a string.
///
/// # Examples
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::*;
///
/// let yaml = "
/// image:
///   path: ubuntu
/// ";
/// let image: Image = from(yaml.into()).unwrap();
/// assert_eq!(
///     image,
///     Image {
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::*;
///
/// let yaml = r#"
/// builders:
///   - name: builder
///     from:
///       path: ekidd/rust-musl-builder
///     add:
///       - paths: ["*"]
///     run:
///       - cargo build --release
/// from:
///   path: ubuntu
/// artifacts:
///   - builder: builder
///     source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
///     target: /app
/// "#;
/// let image: Image = from(yaml.into()).unwrap();
/// assert_eq!(
///     image,
///     Image {
///         builders: vec![Builder {
///             name: Some(String::from("builder")),
///             from: ImageName { path: "ekidd/rust-musl-builder".into(), ..Default::default() }.into(),
///             copy: vec![CopyResource::Copy(Copy{paths: vec!["*".into()].into(), ..Default::default()}).into()].into(),
///             run: vec!["cargo build --release".parse().unwrap()].into(),
///             ..Default::default()
///         }].into(),
///         from: Some(ImageName {
///             path: "ubuntu".into(),
///             ..Default::default()
///         }.into()),
///         artifacts: vec![Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             target: String::from("/app"),
///             ..Default::default()
///         }].into(),
///         ..Default::default()
///     }
/// );
/// ```
pub fn from(input: String) -> Result<Image> {
    merge_extended_image(serde_yaml::from_str(&input).map_err(|err| Error::Deserialize(err))?)
}

/// Parse an Image from a reader.
///
/// # Examples
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::*;
///
/// let yaml = "
/// image:
///   path: ubuntu
/// ";
/// let image: Image = from_reader(yaml.as_bytes()).unwrap();
/// assert_eq!(
///     image,
///     Image {
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::*;
///
/// let yaml = r#"
/// builders:
///   - name: builder
///     from:
///       path: ekidd/rust-musl-builder
///     add:
///       - paths: ["*"]
///     run:
///       - cargo build --release
/// from:
///     path: ubuntu
/// artifacts:
///   - builder: builder
///     source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
///     target: /app
/// "#;
/// let image: Image = from_reader(yaml.as_bytes()).unwrap();
/// assert_eq!(
///     image,
///     Image {
///         builders: vec![Builder {
///             name: Some(String::from("builder")),
///             from: ImageName{path: "ekidd/rust-musl-builder".into(), ..Default::default()}.into(),
///             copy: vec![CopyResource::Copy(Copy{paths: vec!["*".into()].into(), ..Default::default()}).into()].into(),
///             run: vec!["cargo build --release".parse().unwrap()].into(),
///             ..Default::default()
///         }].into(),
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         artifacts: vec![Artifact {
///             builder: String::from("builder"),
///             source: String::from(
///                 "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///             ),
///             target: String::from("/app"),
///             ..Default::default()
///         }].into(),
///         ..Default::default()
///     }
/// );
/// ```
pub fn from_reader<R: Read>(reader: R) -> Result<Image> {
    merge_extended_image(serde_yaml::from_reader(reader).map_err(|err| Error::Deserialize(err))?)
}

/// Parse an Image from a YAML or JSON file path.
pub fn from_file_path(path: &std::path::PathBuf) -> Result<Image> {
    let file = fs::File::open(path).unwrap();
    match path.extension() {
        Some(os_str) => match os_str.to_str() {
            Some("yml" | "yaml" | "json") => merge_extended_image(
                serde_yaml::from_reader(file).map_err(|err| Error::Deserialize(err))?,
            ),
            Some(ext) => Err(Error::Custom(format!(
                "Not managed Dofigen file extension {}",
                ext
            ))),
            None => Err(Error::Custom("The Dofigen file has no extension".into())),
        },
        None => Err(Error::Custom("The Dofigen file has no extension".into())),
    }
}

fn merge_extended_image(image: Extend<ImagePatch>) -> Result<Image> {
    image.merge(&mut LoadContext::new())
}

/// Generates the Dockerfile content from an Image.
///
/// # Examples
///
/// ```
/// use dofigen_lib::*;
///
/// let image = Image {
///     from: Some(ImageName {
///         path: String::from("ubuntu"),
///         ..Default::default()
///     }.into()),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerfile(&image).unwrap();
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\n# syntax=docker/dockerfile:1.7\n\n# runtime\nFROM ubuntu AS runtime\nUSER 1000:1000\n"
/// );
/// ```
pub fn generate_dockerfile(image: &Image) -> Result<String> {
    Ok(format!(
        "{}\n{}\n",
        FILE_HEADER_LINES.join("\n"),
        image
            .generate_dockerfile_lines(&GenerationContext::default())?
            .iter()
            .map(DockerfileLine::generate_content)
            .collect::<Vec<String>>()
            .join("\n")
    ))
}

/// Generates the .dockerignore file content from an Image.
///
/// # Examples
///
/// ## Define the build context
///
/// ```
/// use dofigen_lib::*;
///
/// let image = Image {
///     context: vec![String::from("/src")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\n**\n!/src\n"
/// );
/// ```
///
/// ## Ignore a path
///
/// ```
/// use dofigen_lib::*;
///
/// let image = Image {
///     ignore: vec![String::from("target")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\ntarget\n"
/// );
/// ```
///
/// ## Define context ignoring a specific files
///
/// ```
/// use dofigen_lib::*;
///
/// let image = Image {
///     context: vec![String::from("/src")].into(),
///     ignore: vec![String::from("/src/*.test.rs")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\n**\n!/src\n/src/*.test.rs\n"
/// );
/// ```
pub fn generate_dockerignore(image: &Image) -> String {
    let mut content = String::new();

    content.push_str(FILE_HEADER_LINES.join("\n").as_str());
    content.push_str("\n");

    if !image.context.is_empty() {
        content.push_str("**\n");
        image.context.iter().for_each(|path| {
            content.push_str("!");
            content.push_str(path);
            content.push_str("\n");
        });
    }
    if !image.ignore.is_empty() {
        image.ignore.iter().for_each(|path| {
            content.push_str(path);
            content.push_str("\n");
        });
    }
    content
}

/// Generates the JSON schema for the Image structure.
/// This is useful to validate the structure and IDE autocompletion.
#[cfg(feature = "json_schema")]
pub fn generate_json_schema() -> String {
    let schema = schema_for!(Image);
    serde_json::to_string_pretty(&schema).unwrap()
}

#[cfg(feature = "permissive")]
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn yaml_to_dockerfile() {
        let yaml = r#"
        image: scratch
        builders:
        - name: builder
          from: ekidd/rust-musl-builder
          user: rust
          add: "."
          run:
          - ls -al
          - cargo build --release
          cache: /usr/local/cargo/registry
        - name: watchdog
          from: ghcr.io/openfaas/of-watchdog:0.9.6
        env:
          fprocess: /app
        artifacts:
        - builder: builder
          source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
          target: /app
        - builder: builder
          source: /fwatchdog
          target: /fwatchdog
        expose: 8080
        healthcheck:
          interval: 3s
          cmd: "[ -e /tmp/.lock ] || exit 1"
        cmd: "/fwatchdog"
        ignores:
        - target
        - test
        "#;

        let image: Image = from(yaml.into()).map_err(Error::from).unwrap();
        let dockerfile: String = generate_dockerfile(&image).unwrap();

        assert_eq!(
            dockerfile,
            r#"# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# syntax=docker/dockerfile:1.7

# builder
FROM ekidd/rust-musl-builder AS builder
COPY \
    --chown=rust \
    --link \
    "." "./"
USER rust
RUN \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    ls -al && \
    cargo build --release

# watchdog
FROM ghcr.io/openfaas/of-watchdog:0.9.6 AS watchdog

# runtime
FROM scratch AS runtime
ENV fprocess="/app"
COPY \
    --from=builder \
    --chown=1000:1000 \
    --link \
    "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust" "/app"
COPY \
    --from=builder \
    --chown=1000:1000 \
    --link \
    "/fwatchdog" "/fwatchdog"
USER 1000:1000
EXPOSE 8080
HEALTHCHECK \
    --interval=3s \
    CMD [ -e /tmp/.lock ] || exit 1
CMD ["/fwatchdog"]
"#
        );

        let dockerignore: String = generate_dockerignore(&image);

        assert_eq!(dockerignore, "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\ntarget\ntest\n");
    }

    #[test]
    fn using_dockerfile_overlap_aliases() {
        let yaml = r#"
builders:
- name: builder
  image: ekidd/rust-musl-builder
  adds:
  - "*"
  script:
  - cargo build --release
artifacts:
- builder: builder
  source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
  destination: /app
"#;
        let image: Image = from(yaml.into()).unwrap();
        assert_eq!(
            image,
            Image {
                builders: vec![Builder {
                    name: Some(String::from("builder")),
                    from: ImageName {
                        path: String::from("ekidd/rust-musl-builder"),
                        ..Default::default()
                    }
                    .into(),
                    copy: vec![PermissiveStruct::new(CopyResource::Copy(Copy {
                        paths: vec![String::from("*")].into(),
                        ..Default::default()
                    }))]
                    .into(),
                    run: vec![String::from("cargo build --release")].into(),
                    ..Default::default()
                }]
                .into(),
                artifacts: vec![Artifact {
                    builder: String::from("builder"),
                    source: String::from(
                        "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
                    ),
                    target: String::from("/app"),
                    ..Default::default()
                }]
                .into(),
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
        let image: Image = from(yaml.into()).unwrap();
        let dockerfile: String = generate_dockerfile(&image).unwrap();

        assert_eq!(
            dockerfile,
            r#"# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# syntax=docker/dockerfile:1.7

# runtime
FROM scratch AS runtime
USER 1000:1000
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
        let result = from(yaml.into());
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
        let result = from(yaml.into());
        assert!(
            result.is_err(),
            "The parsing must fail since 'test' is not a valid field"
        );

        // Check the error message
        let error = result.unwrap_err();
        let expected = "Error while deserializing the document at line 3, column 1: unknown field `test`, expected one of ";
        assert_eq!(
            &error.to_string().as_str()[..expected.len()],
            expected,
            "Wrong error message"
        );
    }

    #[test]
    fn manage_plural_aliases() -> Result<()> {
        let yaml = r#"
from: scratch
builders:
- name: builder
  from: ekidd/rust-musl-builder
  user: rust
  adds: 
  - "."
  run:
  - cargo build --release
  caches:
  - /usr/local/cargo/registry
- name: watchdog
  from: ghcr.io/openfaas/of-watchdog:0.9.6
envs:
  fprocess: /app
artifacts:
- builder: builder
  source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
  target: /app
- builder: builder
  source: /fwatchdog
  target: /fwatchdog
ports:
- 8080
ignore:
- target
- test
"#;

        from(yaml.into())?;
        Ok(())
    }

    #[test]
    fn artifact_copy_custom_user() {
        let yaml = r#"
        builders:
        - name: builder
          from: ekidd/rust-musl-builder
          user: rust
          add: "."
          run: cargo build --release
          cache: /usr/local/cargo/registry
        user: 1001
        artifacts:
        - builder: builder
          source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
          target: /app
        run: echo "coucou"
        cache: /tmp
        "#;

        let image: Image = from(yaml.into()).unwrap();
        let dockerfile: String = generate_dockerfile(&image).unwrap();

        assert_eq!(
            dockerfile,
            r#"# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# syntax=docker/dockerfile:1.7

# builder
FROM ekidd/rust-musl-builder AS builder
COPY \
    --chown=rust \
    --link \
    "." "./"
USER rust
RUN \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo build --release

# runtime
FROM scratch AS runtime
COPY \
    --from=builder \
    --chown=1001 \
    --link \
    "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust" "/app"
USER 1001
RUN \
    --mount=type=cache,target=/tmp,sharing=locked,uid=1001 \
    echo "coucou"
"#
        );
    }
}

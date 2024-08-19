//! # dofigen_lib
//!
//! `dofigen_lib` help creating Dockerfile with a simplified structure and made to cache the build with Buildkit.
//! You also can parse the structure from YAML or JSON.

pub mod context;
mod deserialize;
mod dockerfile_struct;
mod dofigen_struct;
mod errors;
mod extend;
#[cfg(feature = "permissive")]
mod from_str;
mod generator;
#[cfg(feature = "json_schema")]
mod json_schema;
pub mod lock;
use context::DofigenContext;
use dockerfile_struct::{DockerfileContent, DockerfileLine};
use generator::{DockerfileGenerator, GenerationContext};
#[cfg(feature = "json_schema")]
use schemars::gen::*;
use std::io::Read;
pub use {deserialize::*, dofigen_struct::*, errors::*, extend::*};

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
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let yaml = "
/// from:
///   path: ubuntu
/// ";
/// let image: Image = from(yaml.into()).unwrap();
/// assert_eq_sorted!(
///     image,
///     Image {
///       stage: Stage {
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         ..Default::default()
///       },
///      ..Default::default()
///     }
/// );
/// ```
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
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
/// assert_eq_sorted!(
///     image,
///     Image {
///         builders: vec![Stage {
///             name: Some(String::from("builder")),
///             from: ImageName { path: "ekidd/rust-musl-builder".into(), ..Default::default() }.into(),
///             copy: vec![CopyResource::Copy(Copy{paths: vec!["*".into()].into(), ..Default::default()}).into()].into(),
///             run: Run {
///                 run: vec!["cargo build --release".parse().unwrap()].into(),
///                 ..Default::default()
///             },
///             ..Default::default()
///         }].into(),
///         stage: Stage {
///             from: Some(ImageName {
///                 path: "ubuntu".into(),
///                 ..Default::default()
///             }.into()),
///             artifacts: vec![Artifact {
///                 builder: String::from("builder"),
///                 source: String::from(
///                     "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///                 ),
///                 target: String::from("/app"),
///                 ..Default::default()
///             }].into(),
///             ..Default::default()
///         },
///         ..Default::default()
///     }
/// );
/// ```
pub fn from(input: String) -> Result<Image> {
    DofigenContext::new().parse_from_string(input.as_str())
}

/// Parse an Image from a reader.
///
/// # Examples
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let yaml = "
/// from:
///   path: ubuntu
/// ";
/// let image: Image = from_reader(yaml.as_bytes()).unwrap();
/// assert_eq_sorted!(
///     image,
///     Image {
///         stage: Stage {
///             from: Some(ImageName {
///                 path: String::from("ubuntu"),
///                 ..Default::default()
///             }.into()),
///             ..Default::default()
///         },
///         ..Default::default()
///     }
/// );
/// ```
///
/// Basic parsing
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
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
/// assert_eq_sorted!(
///     image,
///     Image {
///         builders: vec![Stage {
///             name: Some(String::from("builder")),
///             from: ImageName{path: "ekidd/rust-musl-builder".into(), ..Default::default()}.into(),
///             copy: vec![CopyResource::Copy(Copy{paths: vec!["*".into()].into(), ..Default::default()}).into()].into(),
///             run: Run {
///                 run: vec!["cargo build --release".parse().unwrap()].into(),
///                 ..Default::default()
///             },
///             ..Default::default()
///         }].into(),
///         stage: Stage {
///             from: Some(ImageName {
///                 path: String::from("ubuntu"),
///                 ..Default::default()
///             }.into()),
///             artifacts: vec![Artifact {
///                 builder: String::from("builder"),
///                 source: String::from(
///                     "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
///                 ),
///                 target: String::from("/app"),
///                 ..Default::default()
///             }].into(),
///             ..Default::default()
///         },
///         ..Default::default()
///     }
/// );
/// ```
pub fn from_reader<R: Read>(reader: R) -> Result<Image> {
    DofigenContext::new().parse_from_reader(reader)
}

/// Parse an Image from a YAML or JSON file path.
pub fn from_file_path(path: std::path::PathBuf) -> Result<Image> {
    match path.extension() {
        Some(os_str) => match os_str.to_str() {
            Some("yml" | "yaml" | "json") => {
                DofigenContext::new().parse_from_resource(Resource::File(path))
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
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let image = Image {
///     stage: Stage {
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerfile(&image).unwrap();
/// assert_eq_sorted!(
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
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let image = Image {
///     context: vec![String::from("/src")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq_sorted!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\n**\n!/src\n"
/// );
/// ```
///
/// ## Ignore a path
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let image = Image {
///     ignore: vec![String::from("target")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq_sorted!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\ntarget\n"
/// );
/// ```
///
/// ## Define context ignoring a specific files
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let image = Image {
///     context: vec![String::from("/src")].into(),
///     ignore: vec![String::from("/src/*.test.rs")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&image);
/// assert_eq_sorted!(
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

/// Generates the effective Dofigen content from an Image.
///
/// # Examples
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let image = Image {
///     stage: Stage {
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// let dofigen: String = generate_effective_content(&image).unwrap();
/// assert_eq_sorted!(
///     dofigen,
///     "from:\n  path: ubuntu\n"
/// );
/// ```
pub fn generate_effective_content(image: &Image) -> Result<String> {
    Ok(serde_yaml::to_string(&image)?)
}

/// Generates the JSON schema for the Image structure.
/// This is useful to validate the structure and IDE autocompletion.
#[cfg(feature = "json_schema")]
pub fn generate_json_schema() -> String {
    let settings = SchemaSettings::default().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<Extend<ImagePatch>>();
    serde_json::to_string_pretty(&schema).unwrap()
}

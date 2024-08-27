//! # dofigen_lib
//!
//! `dofigen_lib` help creating Dockerfile with a simplified structure and made to cache the build with Buildkit.
//! You also can parse the structure from YAML or JSON.
//!
//! ```
//! use dofigen_lib::*;
//! use pretty_assertions_sorted::assert_eq_sorted;
//!
//! let mut context = DofigenContext::new();
//!
//! let dofigen = context.parse_from_string(r#"
//! from:
//!   path: ubuntu
//! "#).unwrap();
//!
//! let dockerfile = generate_dockerfile(&dofigen).unwrap();
//! ```

mod context;
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
use dockerfile_struct::{DockerfileContent, DockerfileLine};
use generator::{DockerfileGenerator, GenerationContext};
#[cfg(feature = "json_schema")]
use schemars::gen::*;
pub use {context::*, deserialize::*, dofigen_struct::*, errors::*, extend::*};

#[cfg(all(feature = "strict", feature = "permissive"))]
compile_error!("You can't enable both 'strict' and 'permissive' features at the same time.");

pub(crate) const DOCKERFILE_VERSION: &str = "1.7";

const FILE_HEADER_LINES: [&str; 3] = [
    concat!(
        "# This file is generated by Dofigen v",
        env!("CARGO_PKG_VERSION")
    ),
    concat!("# See ", env!("CARGO_PKG_REPOSITORY")),
    "",
];

/// Generates the Dockerfile content from a Dofigen struct.
///
/// # Examples
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let dofigen = Dofigen {
///     stage: Stage {
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerfile(&dofigen).unwrap();
/// assert_eq_sorted!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\n# syntax=docker/dockerfile:1.7\n\n# runtime\nFROM ubuntu AS runtime\nUSER 1000:1000\n"
/// );
/// ```
pub fn generate_dockerfile(dofigen: &Dofigen) -> Result<String> {
    Ok(format!(
        "{}\n{}\n",
        FILE_HEADER_LINES.join("\n"),
        dofigen
            .generate_dockerfile_lines(&GenerationContext::default())?
            .iter()
            .map(DockerfileLine::generate_content)
            .collect::<Vec<String>>()
            .join("\n")
    ))
}

/// Generates the .dockerignore file content from an Dofigen struct.
///
/// # Examples
///
/// ## Define the build context
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let dofigen = Dofigen {
///     context: vec![String::from("/src")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&dofigen);
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
/// let dofigen = Dofigen {
///     ignore: vec![String::from("target")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&dofigen);
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
/// let dofigen = Dofigen {
///     context: vec![String::from("/src")].into(),
///     ignore: vec![String::from("/src/*.test.rs")].into(),
///     ..Default::default()
/// };
/// let dockerfile: String = generate_dockerignore(&dofigen);
/// assert_eq_sorted!(
///     dockerfile,
///     "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\n**\n!/src\n/src/*.test.rs\n"
/// );
/// ```
pub fn generate_dockerignore(dofigen: &Dofigen) -> String {
    let mut content = String::new();

    content.push_str(FILE_HEADER_LINES.join("\n").as_str());
    content.push_str("\n");

    if !dofigen.context.is_empty() {
        content.push_str("**\n");
        dofigen.context.iter().for_each(|path| {
            content.push_str("!");
            content.push_str(path);
            content.push_str("\n");
        });
    }
    if !dofigen.ignore.is_empty() {
        dofigen.ignore.iter().for_each(|path| {
            content.push_str(path);
            content.push_str("\n");
        });
    }
    content
}

/// Generates the effective Dofigen content from a Dofigen struct.
///
/// # Examples
///
/// ```
/// use dofigen_lib::*;
/// use pretty_assertions_sorted::assert_eq_sorted;
///
/// let dofigen = Dofigen {
///     stage: Stage {
///         from: Some(ImageName {
///             path: String::from("ubuntu"),
///             ..Default::default()
///         }.into()),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// let dofigen: String = generate_effective_content(&dofigen).unwrap();
/// assert_eq_sorted!(
///     dofigen,
///     "from:\n  path: ubuntu\n"
/// );
/// ```
pub fn generate_effective_content(dofigen: &Dofigen) -> Result<String> {
    Ok(serde_yaml::to_string(&dofigen)?)
}

/// Generates the JSON schema for the Dofigen struct.
/// This is useful to validate the structure and IDE autocompletion.
#[cfg(feature = "json_schema")]
pub fn generate_json_schema() -> String {
    let settings = SchemaSettings::default().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = true;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<Extend<DofigenPatch>>();
    serde_json::to_string_pretty(&schema).unwrap()
}

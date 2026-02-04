//! # effective
//!
//! The generate the effective content after merging with the extended Dofigen files.

use std::{fs, path::PathBuf};

use crate::{
    commands::generate::{DEFAULT_DOCKERFILE, DEFAULT_DOFIGEN_FILE},
    *,
};
pub use clap::Args;
use commands::get_file_path;
use dofigen_lib::{Dofigen, Error, Result};

use crate::CliCommand;

#[derive(Args, Debug, Default, Clone)]
pub struct Parse {
    /// The input Dockerfile file. Default value is "Dockerfile".
    /// Use "-" to read from stdin
    #[clap(short, long, default_value = DEFAULT_DOCKERFILE)]
    pub file: Option<String>,

    /// The output Dofigen file
    /// Define to - to write to stdout
    #[clap(short, long, default_value = DEFAULT_DOFIGEN_FILE)]
    output: String,
}

impl CliCommand for Parse {
    fn run(self) -> Result<()> {
        let path = get_file_path(&self.file)?;

        let (dockerfile_content, dockerignore_content): (String, Option<String>) = if path == "-" {
            let content = std::io::read_to_string(std::io::stdin())
                .map_err(|err| Error::Custom(format!("Unable to read stdin: {}", err)))?;
            (content, None)
        } else {
            let dockerfile = PathBuf::from(&path);
            if !dockerfile.exists() {
                return Err(Error::Custom(format!(
                    "No Dockerfile file found at path: {}",
                    dockerfile.display()
                )));
            }
            let filename = dockerfile.file_name().unwrap().to_str().unwrap();
            let mut ignorefile = dockerfile.with_file_name(format!("{}.dockerignore", filename));
            if filename == "Dockerfile" && !ignorefile.exists() {
                ignorefile = dockerfile.clone().with_file_name(".dockerignore");
            }
            let dockerfile_content = fs::read_to_string(&dockerfile).map_err(|err| {
                Error::Custom(format!("Unable to read the Dockerfile file: {}", err))
            })?;
            let dockerignore_content = if ignorefile.exists() {
                Some(fs::read_to_string(&ignorefile).map_err(|err| {
                    Error::Custom(format!("Unable to read the .dockerignore file: {}", err))
                })?)
            } else {
                None
            };
            (dockerfile_content, dockerignore_content)
        };

        let dockerfile = dockerfile_content
            .parse()
            .map_err(|err| Error::Custom(format!("Unable to parse the Dockerfile: {}", err)))?;
        let dockerignore = if let Some(content) = dockerignore_content {
            Some(content.parse()?)
        } else {
            None
        };

        let dofigen = Dofigen::from_dockerfile(dockerfile, dockerignore)?;
        let dofigen_content = serde_yaml::to_string(&dofigen)?;

        if self.output == "-" {
            print!("{}", dofigen_content);
        } else {
            fs::write(PathBuf::from(&self.output), dofigen_content)
                .expect("Unable to write the Dofigen file");
        };
        Ok(())
    }
}

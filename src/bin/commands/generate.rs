//! # generate
//!
//! The generate subcommand generates a Dockerfile and a .dockerignore file from a Dofigen file.

use super::{get_file_path, get_image_from_path, get_lockfile_path, load_lockfile};
use crate::CliCommand;
use clap::Args;
use dofigen_lib::{
    from, generate_dockerfile, generate_dockerignore,
    lock::{Lock, LockContext},
    Error, Result,
};
use std::{collections::HashMap, fs, path::PathBuf};

const DEFAULT_DOCKERFILE: &str = "Dockerfile";

#[derive(Args, Debug, Default, Clone)]
pub struct Generate {
    /// The input file Dofigen file. Default search for the next files: dofigen.yml, dofigen.yaml, dofigen.json
    /// Define to - to read from stdin
    #[clap(short, long)]
    file: Option<String>,
    /// The output Dockerfile file
    /// Define to - to write to stdout
    #[clap(short, long, default_value = DEFAULT_DOCKERFILE)]
    output: String,

    /// Locked version of the image
    #[clap(short, long, action)]
    locked: bool,
}

impl Generate {
    fn write_dockerfile(&self, dockerfile_content: &str, ignore_content: &str) -> Result<()> {
        let dockerfile = PathBuf::from(&self.output);
        fs::write(&dockerfile, dockerfile_content).expect("Unable to write the Dockerfile");

        let filename = dockerfile.file_name().unwrap().to_str().unwrap();
        let ignorefile = if filename == "Dockerfile" {
            dockerfile.with_file_name(".dockerignore")
        } else {
            dockerfile.with_file_name(format!("{}.dockerignore", filename))
        };
        fs::write(ignorefile, ignore_content).expect("Unable to write the .dockerignore file");

        Ok(())
    }
}

impl CliCommand for Generate {
    fn run(&self) -> Result<()> {
        // Get lock file from the file
        let path = get_file_path(&self.file);
        let lockfile_path = get_lockfile_path(path.clone());
        let image = if self.locked {
            if path == "-" {
                return Err(Error::Custom(
                    "The '--locked' option can't be used with stdin".into(),
                ));
            }
            let lockfile =
                load_lockfile(lockfile_path).ok_or(Error::Custom("No lock file found".into()))?;
            from(lockfile.image)?
        } else {
            let image = get_image_from_path(path)?;
            let lockfile = load_lockfile(lockfile_path.clone());

            let mut lock_context = lockfile.map(LockContext::from).unwrap_or(LockContext {
                images: HashMap::new(),
            });

            // Replace images tags with the digest
            let locked_image = image.lock(&mut lock_context)?;
            let new_lockfile = lock_context.to_lockfile(&locked_image)?;

            if let Some(lockfile_path) = lockfile_path {
                serde_yaml::to_writer(
                    std::fs::File::create(lockfile_path).map_err(|err| {
                        Error::Custom(format!("Unable to create the lock file: {}", err))
                    })?,
                    &new_lockfile,
                )
                .map_err(Error::from)?;
            };

            image
        };

        let dockerfile_content = generate_dockerfile(&image)?;

        if self.output == "-" {
            print!("{}", dockerfile_content);
        } else {
            self.write_dockerfile(
                dockerfile_content.as_str(),
                generate_dockerignore(&image).as_str(),
            )?;
        };
        Ok(())
    }
}

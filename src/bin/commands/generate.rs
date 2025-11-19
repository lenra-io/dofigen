//! # generate
//!
//! The generate subcommand generates a Dockerfile and a .dockerignore file from a Dofigen file.

use super::{get_file_path, get_image_from_path, get_lockfile_path, load_lockfile};
use crate::{CliCommand, GlobalOptions};
use clap::Args;
use colored::{Color, Colorize};
use dofigen_lib::{
    DofigenContext, Error, GenerationContext, MessageLevel, Result,
    lock::{Lock, LockFile},
};
use std::{fs, path::PathBuf};

const DEFAULT_DOCKERFILE: &str = "Dockerfile";

#[derive(Args, Debug, Default, Clone)]
pub struct Generate {
    #[command(flatten)]
    pub options: GlobalOptions,

    /// The output Dockerfile file
    /// Define to - to write to stdout
    #[clap(short, long, default_value = DEFAULT_DOCKERFILE)]
    output: String,

    /// Locked version of the dofigen definition
    #[clap(short, long, action)]
    locked: bool,

    /// Do not define the default labels
    #[clap(short, long, action)]
    no_labels: bool,
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
    fn run(self) -> Result<()> {
        let path = get_file_path(&self.options.file)?;
        let lockfile_path = get_lockfile_path(path.clone());
        let lockfile = load_lockfile(lockfile_path.clone());

        let mut context = lockfile
            .as_ref()
            .map(|l| l.to_context())
            .unwrap_or(DofigenContext::new());

        let dofigen = if self.locked {
            if path == "-" {
                return Err(Error::Custom(
                    "The '--locked' option can't be used with stdin".into(),
                ));
            }
            let lockfile = lockfile.ok_or(Error::Custom("No lock file found".into()))?;
            context.parse_from_string(lockfile.effective.as_str())?
        } else {
            context.offline = self.options.offline;
            context.update_file_resources = true;
            context.no_default_labels = self.no_labels;

            let dofigen = get_image_from_path(path, &mut context)?;

            // Replace images tags with the digest
            let locked_image = dofigen.lock(&mut context)?;
            context.clean_unused();
            let new_lockfile = LockFile::from_context(&locked_image, &mut context)?;

            if let Some(lockfile_path) = lockfile_path {
                serde_yaml::to_writer(
                    std::fs::File::create(lockfile_path).map_err(|err| {
                        Error::Custom(format!("Unable to create the lock file: {}", err))
                    })?,
                    &new_lockfile,
                )
                .map_err(Error::from)?;
            };

            locked_image
        };

        let mut generation_context = GenerationContext::from(dofigen);

        let dockerfile_content = generation_context.generate_dockerfile()?;

        let messages = generation_context.get_lint_messages().clone();

        messages.iter().for_each(|message| {
            eprintln!(
                "{}[path={}]: {}",
                match message.level {
                    MessageLevel::Error => "error".color(Color::Red).bold(),
                    MessageLevel::Warn => "warning".color(Color::Yellow).bold(),
                },
                message.path.join(".").color(Color::Blue).bold(),
                message.message
            );
        });

        let errors = messages
            .iter()
            .filter(|m| m.level == MessageLevel::Error)
            .count();

        if errors > 0 {
            return Err(Error::Custom(format!(
                "Could not generate the Dockerfile due to {} previous error{}",
                errors,
                if errors > 1 { "s" } else { "" }
            )));
        }

        if self.output == "-" {
            print!("{}", dockerfile_content);
        } else {
            self.write_dockerfile(
                dockerfile_content.as_str(),
                generation_context.generate_dockerignore()?.as_str(),
            )?;
        };
        Ok(())
    }
}

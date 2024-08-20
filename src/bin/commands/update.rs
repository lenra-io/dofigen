//! # generate
//!
//! The generate subcommand generates a Dockerfile and a .dockerignore file from a Dofigen file.

use super::{get_file_path, get_image_from_path, get_lockfile_path};
use crate::{CliCommand, GlobalOptions};
use clap::Args;
use dofigen_lib::{
    DofigenContext,
    lock::{Lock, LockFile},
    Error, Result,
};

#[derive(Args, Debug, Default, Clone)]
pub struct Update {
    #[command(flatten)]
    pub options: GlobalOptions,

    /// Don't actually write the lockfile
    #[clap(long, action)]
    dry_run: bool,
}

impl CliCommand for Update {
    fn run(self) -> Result<()> {
        // Get lock file from the file
        let path = get_file_path(&self.options.file);
        if path == "-" {
            return Err(Error::Custom(
                "Update command can't be used with stdin".into(),
            ));
        }
        let lockfile_path = get_lockfile_path(path.clone()).ok_or(Error::Custom(
            "The update command needs a lock file to update".into(),
        ))?;

        let mut context = DofigenContext::new();
        context.offline = self.options.offline;

        let image = get_image_from_path(path, &mut context)?;

        // Replace images tags with the digest
        let locked_image = image.lock(&mut context)?;
        let new_lockfile = LockFile::from_context(&locked_image, &context)?;

        // TODO: display lockfile diff

        if self.dry_run {
            println!(
                "{}",
                serde_yaml::to_string(&new_lockfile).map_err(Error::from)?
            );
            return Ok(());
        }

        serde_yaml::to_writer(
            std::fs::File::create(lockfile_path)
                .map_err(|err| Error::Custom(format!("Unable to create the lock file: {}", err)))?,
            &new_lockfile,
        )
        .map_err(Error::from)?;

        Ok(())
    }
}

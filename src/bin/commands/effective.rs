//! # effective
//!
//! The generate the effective content after merging with the extended Dofigen files.

use crate::*;
pub use clap::Args;
use commands::{get_file_path, get_image_from_path, get_lockfile_path, load_lockfile};
use dofigen_lib::{generate_effective_content, lock::Lock, DofigenContext, Error, Result};

use crate::CliCommand;

#[derive(Args, Debug, Default, Clone)]
pub struct Effective {
    #[command(flatten)]
    pub options: GlobalOptions,

    /// Locked version of the dofigen definition
    #[clap(short, long, action)]
    locked: bool,

    /// Do not define the default labels
    #[clap(short, long, action)]
    no_labels: bool,
}

impl CliCommand for Effective {
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
            context.display_updates = false;
            context.no_default_labels = self.no_labels;

            let dofigen = get_image_from_path(path, &mut context)?;

            dofigen.lock(&mut context)?
        };

        println!("{}", generate_effective_content(&dofigen)?);
        Ok(())
    }
}

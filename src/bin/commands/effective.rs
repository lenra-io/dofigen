//! # effective
//!
//! The generate the effective content after merging with the extended Dofigen files.

use crate::*;
pub use clap::Args;
use commands::get_image_from_cli_path;
use dofigen_lib::{DofigenContext, generate_effective_content};

use crate::CliCommand;

#[derive(Args, Debug, Default, Clone)]
pub struct Effective {
    #[command(flatten)]
    pub options: GlobalOptions,
}

impl CliCommand for Effective {
    fn run(self) -> Result<()> {
        let image = get_image_from_cli_path(&self.options.file, &mut DofigenContext::new())?;

        println!("{}", generate_effective_content(&image)?);
        Ok(())
    }
}

//! # effective
//!
//! The generate the effective content after merging with the extended Dofigen files.

use crate::*;
pub use clap::Args;
use commands::get_image_from_cli_path;
use dofigen_lib::generate_effective_content;

use crate::CliCommand;

#[derive(Args, Debug, Default, Clone)]
pub struct Effective {
    /// The input file Dofigen file. Default search for the next files: dofigen.yml, dofigen.yaml, dofigen.json
    /// Define to - to read from stdin
    #[clap(short, long)]
    file: Option<String>,
}

impl CliCommand for Effective {
    fn run(&self) -> Result<()> {
        let image = get_image_from_cli_path(&self.file)?;

        println!("{}", generate_effective_content(&image)?);
        Ok(())
    }
}

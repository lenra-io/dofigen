//! # generate
//!
//! The generate subcommand generates a Dockerfile and a .dockerignore file from a Dofigen file.

use crate::*;
pub use clap::Args;
use dofigen_lib::generate_json_schema;

use crate::CliCommand;

#[derive(Args, Debug, Default, Clone)]
pub struct Schema;

impl CliCommand for Schema {
    fn run(self) -> Result<()> {
        println!("{}", generate_json_schema());
        Ok(())
    }
}

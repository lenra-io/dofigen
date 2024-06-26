use dofigen_lib::Result;
use std::fmt;

use clap::{Parser, Subcommand};

use self::commands::generate::Generate;
#[cfg(feature = "json_schema")]
use self::commands::schema::Schema;

mod commands;

#[derive(clap::ValueEnum, Clone, Debug)]
enum Format {
    Json,
    Yaml,
}
impl fmt::Display for Format {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", format!("{:?}", self).to_lowercase())
    }
}

/// Dofigen is a Dockerfile generator using a simplyfied description in YAML or JSON format.
#[derive(Parser)]
#[clap(author, version, about, long_about = None, rename_all = "kebab-case")]
struct Cli {
    /// The subcommand to run
    #[clap(subcommand)]
    pub command: Command,
}

pub trait CliCommand {
    fn run(&self) -> Result<()>;
    fn need_config(&self) -> bool {
        true
    }
}

/// The subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Generate the Dockerfile and .dockerignore files
    #[clap(alias = "gen")]
    Generate(Generate),
    /// Generate the JSON Schema for the Dofigen structure
    #[cfg(feature = "json_schema")]
    Schema(Schema),
}

impl Command {
    fn run(&self) -> Result<()> {
        match self {
            Command::Generate(g) => g.run(),
            #[cfg(feature = "json_schema")]
            Command::Schema(s) => s.run(),
        }
    }
}

fn main() {
    Cli::parse()
        .command
        .run()
        .unwrap_or_else(|e| eprintln!("{}", e));
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cmd() {
        <Cli as CommandFactory>::command().debug_assert();
    }
}

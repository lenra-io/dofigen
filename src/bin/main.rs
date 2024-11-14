use clap::{Args, Parser, Subcommand};
use colored::{Color, Colorize};
#[cfg(feature = "json_schema")]
use commands::schema::Schema;
use commands::{effective::Effective, generate::Generate, update::Update};
use dofigen_lib::Result;

mod commands;

/// Dofigen is a Dockerfile generator using a simplified description in YAML or JSON format.
#[derive(Parser)]
#[clap(author, version, about, long_about = None, rename_all = "kebab-case")]
struct Cli {
    /// The subcommand to run
    #[clap(subcommand)]
    pub command: Command,
}

/// Represents option common to all subcommands
#[derive(Args, Debug, Default, Clone)]
pub struct GlobalOptions {
    /// The input Dofigen file. Default search for the next files: dofigen.yml, dofigen.yaml, dofigen.json
    /// Use "-" to read from stdin
    #[clap(short, long)]
    pub file: Option<String>,

    /// The command won't load data from any URL.
    /// This disables extending file from URL and loading image tag
    #[clap(long, action)]
    pub offline: bool,
}

pub trait CliCommand {
    fn run(self) -> Result<()>;
}

/// The subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Generate the Dockerfile and .dockerignore files
    #[clap(alias = "gen")]
    Generate(Generate),

    /// Generate the effective Dofigen configuration once the extends are resolved
    Effective(Effective),

    /// Updates the lock file
    Update(Update),

    /// Generate the JSON Schema for the Dofigen structure
    #[cfg(feature = "json_schema")]
    Schema(Schema),
}

impl Command {
    fn run(self) -> Result<()> {
        match self {
            Command::Generate(g) => g.run(),
            Command::Effective(e) => e.run(),
            Command::Update(u) => u.run(),
            #[cfg(feature = "json_schema")]
            Command::Schema(s) => s.run(),
        }
    }
}

fn main() {
    Cli::parse().command.run().unwrap_or_else(|e| {
        eprintln!("{}: {}", "error".color(Color::Red).bold(), e);
        std::process::exit(1);
    });
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

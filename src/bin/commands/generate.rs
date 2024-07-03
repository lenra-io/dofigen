//! # generate
//!
//! The generate subcommand generates a Dockerfile and a .dockerignore file from a Dofigen file.

pub use clap::Args;
use dofigen_lib::{
    from_file_path, from_reader, generate_dockerfile, generate_dockerignore, Result,
};
use std::{fs, path::PathBuf};

use crate::CliCommand;

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
        let file = if let Some(path) = &self.file {
            path
        } else {
            let mut files = vec!["dofigen.yml", "dofigen.yaml", "dofigen.json"];
            files.retain(|f| std::path::Path::new(f).exists());
            if files.is_empty() {
                eprintln!("No Dofigen file found");
                std::process::exit(1);
            }
            &files[0].to_string()
        };
        let image = if file == "-" {
            from_reader(std::io::stdin())
        } else {
            from_file_path(&PathBuf::from(file))
        }
        .expect("Failed to load the Dofigen structure");

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

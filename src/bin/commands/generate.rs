//! # generate
//!
//! The generate subcommand generates a Dockerfile and a .dockerignore file from a Dofigen file.

use crate::*;
pub use clap::Args;

use crate::CliCommand;

#[derive(Args, Debug, Default, Clone)]
pub struct Generate {
    /// The input file Dofigen file. Default reads stdin
    input_file: Option<std::path::PathBuf>,
    /// The output Dockerfile file
    #[clap(short, long, default_value = "Dockerfile")]
    dockerfile: std::path::PathBuf,
    /// The output .dockerignore file
    #[clap(short, long, default_value = ".dockerignore")]
    ignorefile: std::path::PathBuf,
    /// Writes the Dockerfile to the stdout
    #[clap(short, long, action)]
    output: bool,
}

impl CliCommand for Generate {
    fn run(&self) -> Result<()> {
        let output = self.output;
        let dockerfile = &self.dockerfile;
        let ignorefile = &self.ignorefile;

        let image = if let Some(path) = self.input_file.clone() {
            from_file_path(&path)
        } else {
            from_reader(std::io::stdin())
        }
        .expect("Failed to load the Dofigen structure");

        let dockerfile_content = generate_dockerfile(&image);
        if output {
            print!("{}", dockerfile_content);
        } else {
            fs::write(dockerfile, dockerfile_content).expect("Unable to write the Dockerfile");
            let dockerignore_content = generate_dockerignore(&image);
            fs::write(ignorefile, dockerignore_content)
                .expect("Unable to write the .dockerignore file");
        }
        Ok(())
    }
}

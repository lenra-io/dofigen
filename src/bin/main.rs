use dofigen_lib::{
    from_file_path, from_yaml_reader, generate_dockerfile, generate_dockerignore,
};
use std::{fmt, fs};

use clap::Parser;

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
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The input file Dofigen file. Default reads stdin
    #[clap(parse(from_os_str))]
    input_file: Option<std::path::PathBuf>,
    /// Deprecated. The input format [default: yaml]
    #[clap(value_enum, short, long)]
    format: Option<Format>,
    /// The output Dockerfile file
    #[clap(parse(from_os_str), short, long, default_value = "Dockerfile")]
    dockerfile: std::path::PathBuf,
    /// The output .dockerignore file
    #[clap(parse(from_os_str), short, long, default_value = ".dockerignore")]
    ignorefile: std::path::PathBuf,
    /// Writes the Dockerfile to the stdout
    #[clap(short, long, action)]
    output: bool,
}

fn main() {
    let args = Args::parse();
    let output = args.output;
    let dockerfile = args.dockerfile.clone();
    let ignorefile = args.ignorefile.clone();
    let image = if let Some(path) = args.input_file {
        from_file_path(&path)
    } else {
        from_yaml_reader(std::io::stdin())
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
}

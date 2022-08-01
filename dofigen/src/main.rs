use dofigen_lib::{
    from_json_reader, from_yaml_reader, generate_dockerfile, generate_dockerignore, structs::Image,
};
use std::{fmt, fs, io::BufReader, io::Read};

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
    /// The input format [default: yaml]
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
    let input_and_format = get_input_and_format(args);

    let format = input_and_format.format.unwrap_or(Format::Yaml);
    let image: Image = match format {
        Format::Yaml => from_yaml_reader(input_and_format.input),
        Format::Json => from_json_reader(input_and_format.input),
    };
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

struct InputAndFormat {
    input: Box<dyn Read>,
    format: Option<Format>,
}

fn get_input_and_format(args: Args) -> InputAndFormat {
    let mut given_format: Option<Format> = args.format;
    if let Some(path) = args.input_file {
        if given_format.is_none() {
            given_format = match path.extension() {
                None => None,
                Some(os_str) => match os_str.to_str() {
                    Some("yaml") => Some(Format::Yaml),
                    Some("yml") => Some(Format::Yaml),
                    Some("json") => Some(Format::Json),
                    _ => None,
                },
            }
        }
        let file = fs::File::open(path).unwrap();
        return InputAndFormat {
            input: Box::new(BufReader::new(file)),
            format: given_format,
        };
    }

    InputAndFormat {
        input: Box::new(std::io::stdin()),
        format: given_format,
    }
}

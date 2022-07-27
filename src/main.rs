use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead};

#[derive(Serialize, Deserialize)]
struct Artifact {
    buider: String,
    source: String,
    destination: String,
}

// #[derive(Serialize, Deserialize)]
// struct CopyFull {
//     source: String,
//     destination: Option<String>,
//     chown: Option<String>,
// }

// pub trait Copy {}
// impl Copy for String {}
// impl Copy for CopyFull {}

#[derive(Serialize, Deserialize)]
struct Builder {
    // Common part
    image: String,
    #[serde(rename = "rootScript")]
    root_script: Option<Vec<String>>,
    script: Vec<String>,
    workdir: Option<String>,
    copy: Option<Vec<String>>,
    artifacts: Option<Vec<String>>,
    // Specific part
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Image {
    // Common part
    image: String,
    #[serde(rename = "rootScript")]
    root_script: Option<Vec<String>>,
    script: Vec<String>,
    workdir: Option<String>,
    // Specific part
    builders: Option<Vec<Builder>>,
    ignore: Option<Vec<String>>,
}

pub trait Stage {}
impl Stage for Image {}
impl Stage for Builder {}

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        buffer.push_str(&line.unwrap());
        let p: Result<Value, _> = serde_json::from_str(&buffer);
        match p {
            Ok(_json_val) => {
                generate_image(&serde_json::from_str(&buffer).unwrap());
                break;
            }
            Err(_) => {
                buffer.push_str("\n");
            }
        };
    }
    Ok(())
}

fn generate_image(image: &Image) {
    println!("{}", serde_json::to_string(image).unwrap());
    image.builders.iter().for_each(builder {
        generate_stage(&builder);
    });
    if let Some(builders) = image.builders {
        println!("{} builders", builders.len().to_string())
    }
    generate_stage(image);
}

fn generate_stage<T: ?Stage + serde::Serialize>(stage: &T) {
    println!("{}", serde_json::to_string(stage).unwrap());
}

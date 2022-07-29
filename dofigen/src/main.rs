use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{self, BufRead},
};

#[derive(Serialize, Deserialize)]
struct Artifact {
    builder: String,
    source: String,
    destination: String,
}

#[derive(Serialize, Deserialize)]
struct CopyFull {
    source: String,
    destination: Option<String>,
    chown: Option<String>,
}

// pub trait Copy {

// }
// impl Copy for String {}
// impl Copy for CopyFull {}

pub trait Stage {
    fn image(&self) -> &str;
    fn name(&self, position: i32) -> String;
    fn envs(&self) -> Option<&HashMap<String, String>>;
    fn additionnal_generation() {}

    fn generate(&self, previous_builders: &mut Vec<String>) {
        // println!("{}", serde_json::to_string(stage).unwrap());
        let name = self.name(previous_builders.len().try_into().unwrap());
        println!("\n# {}", name);
        println!("FROM {} as {}", self.image(), name);

        // Envs
        if let Some(ref envs) = self.envs() {
            print!("ENV ");
            envs.iter()
                .for_each(|(key, value)| println!("\\\n\t{}=\"{}\"", key, value));
        }

        previous_builders.push(name);
    }
}

#[derive(Serialize, Deserialize)]
struct Builder {
    // Common part
    image: String,
    envs: Option<HashMap<String, String>>,
    #[serde(rename = "rootScript")]
    root_script: Option<Vec<String>>,
    script: Option<Vec<String>>,
    workdir: Option<String>,
    add: Option<Vec<String>>,
    artifacts: Option<Vec<Artifact>>,
    // Specific part
    name: Option<String>,
}
impl Stage for Builder {
    fn image(&self) -> &str {
        &self.image
    }
    fn name(&self, position: i32) -> String {
        match self.name.as_ref() {
            Some(name) => String::from(name),
            None => format!("builder-{}", position),
        }
    }
    fn envs(&self) -> Option<&HashMap<String, String>> {
        self.envs.as_ref()
    }
}

#[derive(Serialize, Deserialize)]
struct Image {
    // Common part
    image: String,
    envs: Option<HashMap<String, String>>,
    #[serde(rename = "rootScript")]
    root_script: Option<Vec<String>>,
    script: Option<Vec<String>>,
    workdir: Option<String>,
    add: Option<Vec<String>>,
    artifacts: Option<Vec<Artifact>>,
    // Specific part
    builders: Option<Vec<Builder>>,
    ignore: Option<Vec<String>>,
}
impl Stage for Image {
    fn image(&self) -> &str {
        &self.image
    }
    fn name(&self, _position: i32) -> String {
        String::from("runtime")
    }
    fn envs(&self) -> Option<&HashMap<String, String>> {
        self.envs.as_ref()
    }
}

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
    println!("# syntax=docker/dockerfile:1.4");
    let mut previous_builders = Vec::new();
    if let Some(ref builders) = image.builders {
        println!("{} builder(s)", builders.len().to_string());
        builders
            .iter()
            .for_each(|builder| builder.generate(&mut previous_builders));
    }
    image.generate(&mut previous_builders);
}

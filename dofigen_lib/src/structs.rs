use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/** Represents a Dockerfile stage */
pub trait Stage {
    fn image(&self) -> &str;
    fn name(&self, position: i32) -> String;
    fn envs(&self) -> Option<&HashMap<String, String>>;
    fn builders(&self) -> Option<&Vec<Builder>> {
        None
    }
    fn additionnal_generation() {}

    fn generate(&self, buffer: &mut String, previous_builders: &mut Vec<String>) {
        let name = self.name(previous_builders.len().try_into().unwrap());
        buffer.push_str(format!("\n# {}\nFROM {} as {}\n", name, self.image(), name).as_str());

        // Envs
        if let Some(ref envs) = self.envs() {
            buffer.push_str("ENV ");
            envs.iter().for_each(|(key, value)| {
                buffer.push_str(format!("\\\n\t{}=\"{}\"", key, value).as_str())
            });
        }

        previous_builders.push(name);
    }
}

/** Represents a Dockerfile builder stage */
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Builder {
    // Common part
    pub image: String,
    pub envs: Option<HashMap<String, String>>,
    #[serde(rename = "rootScript")]
    pub root_script: Option<Vec<String>>,
    pub script: Option<Vec<String>>,
    pub workdir: Option<String>,
    pub add: Option<Vec<String>>,
    pub artifacts: Option<Vec<Artifact>>,
    // Specific part
    pub name: Option<String>,
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

/** Represents the Dockerfile main stage */
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Image {
    // Common part
    pub image: String,
    pub envs: Option<HashMap<String, String>>,
    #[serde(rename = "rootScript")]
    pub root_script: Option<Vec<String>>,
    pub script: Option<Vec<String>>,
    pub workdir: Option<String>,
    pub add: Option<Vec<String>>,
    pub artifacts: Option<Vec<Artifact>>,
    // Specific part
    pub builders: Option<Vec<Builder>>,
    pub ignore: Option<Vec<String>>,
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
    fn builders(&self) -> Option<&Vec<Builder>> {
        self.builders.as_ref()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Artifact {
    pub builder: String,
    pub source: String,
    pub destination: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct CopyFull {
    pub source: String,
    pub destination: Option<String>,
    pub chown: Option<String>,
}

// pub trait Copy {

// }
// impl Copy for String {}
// impl Copy for CopyFull {}

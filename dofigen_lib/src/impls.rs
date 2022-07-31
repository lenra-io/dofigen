use std::{collections::HashMap, io::Result};

use crate::structs::{Artifact, Builder, Image};

/** Represents a Dockerfile stage */
pub trait Stage {
    fn image(&self) -> &str;
    fn name(&self, position: i32) -> String;
    fn user(&self) -> Option<String>;
    fn workdir(&self) -> Option<&String>;
    fn envs(&self) -> Option<&HashMap<String, String>>;
    fn artifacts(&self) -> Option<&Vec<Artifact>>;
    fn adds(&self) -> Option<&Vec<String>>;
    fn root_script(&self) -> Option<&Vec<String>>;
    fn script(&self) -> Option<&Vec<String>>;
    fn caches(&self) -> Option<&Vec<String>>;

    fn generate(&self, buffer: &mut String, previous_builders: &mut Vec<String>) -> Result<()> {
        let name = self.name(previous_builders.len().try_into().unwrap());
        buffer.push_str(format!("\n# {}\nFROM {} as {}\n", name, self.image(), name).as_str());

        // Set env variables
        if let Some(ref envs) = self.envs() {
            buffer.push_str("ENV ");
            envs.iter().for_each(|(key, value)| {
                buffer.push_str(format!("\\\n\t{}=\"{}\"", key, value).as_str())
            });
            buffer.push_str("\n");
        }

        // Set workdir
        if let Some(ref workdir) = self.workdir() {
            buffer.push_str(format!("WORKDIR {}\n", workdir).as_str());
        }

        // Add sources
        if let Some(ref adds) = self.adds() {
            adds.iter()
                .map(|add| format!("ADD --link {} ./\n", add))
                .for_each(|add| buffer.push_str(&add.as_str()));
        }

        let mut root_script: Vec<String> = Vec::new();

        // Copy build artifacts
        if let Some(ref artifacts) = self.artifacts() {
            artifacts
                .iter()
                .map(|artifact| {
                    if !previous_builders.contains(&artifact.builder) {
                        panic!(
                            "The builder '{}' is not found in previous artifacts",
                            artifact.builder
                        )
                    }
                    format!(
                        "COPY --link --from={builder} \"{source}\" \"{destination}\"\n",
                        builder = artifact.builder,
                        source = artifact.source,
                        destination = artifact.destination
                    )
                })
                .for_each(|artifact| buffer.push_str(&artifact.as_str()));
        }

        // Root script
        if let Some(additionnal_root_script) = self.root_script() {
            additionnal_root_script
                .iter()
                .for_each(|script| root_script.push(script.clone()));
        }
        let mut is_root = false;
        if root_script.len() > 0 {
            is_root = true;
            buffer.push_str("USER 0\n");
            add_script(buffer, &root_script, self.caches());
        }

        // Runtime user
        let mut user: Option<String> = None;
        if self.user().is_some() {
            user = self.user()
        }
        else if let Some(script) = self.script() {
            if is_root && script.len()>0 && self.image()!="scratch" {
                user = Some(String::from("1000"));
            }
        }
        if user.is_some() {
            buffer.push_str(format!("USER {}\n", user.unwrap()).as_str());
        }

        // Script
        if let Some(script) = self.script() {
            add_script(buffer, script, self.caches());
        }

        self.additionnal_generation(buffer);

        previous_builders.push(name);
        Ok(())
    }

    fn additionnal_generation(&self, _buffer: &mut String) {}
}

fn add_script(buffer: &mut String, script: &Vec<String>, caches: Option<&Vec<String>>) {
    buffer.push_str("RUN ");
    if let Some(ref paths) = caches {
        paths.iter().for_each(|path| {
            buffer.push_str(format!("\\\n\t--mount=type=cache,uid=1000,gid=1000,target={}", path).as_str())
        })
    }
    script.iter().enumerate().for_each(|(i, cmd)| {
        if i > 0 {
            buffer.push_str(" && ");
        }
        buffer.push_str(format!("\\\n\t{}", cmd).as_str())
    });
    buffer.push_str("\n");
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
    fn user(&self) -> Option<String> {
        self.user.clone()
    }
    fn workdir(&self) -> Option<&String> {
        self.workdir.as_ref()
    }
    fn envs(&self) -> Option<&HashMap<String, String>> {
        self.envs.as_ref()
    }
    fn artifacts(&self) -> Option<&Vec<Artifact>> {
        self.artifacts.as_ref()
    }
    fn adds(&self) -> Option<&Vec<String>> {
        self.adds.as_ref()
    }
    fn root_script(&self) -> Option<&Vec<String>> {
        self.root_script.as_ref()
    }
    fn script(&self) -> Option<&Vec<String>> {
        self.script.as_ref()
    }
    fn caches(&self) -> Option<&Vec<String>> {
        self.caches.as_ref()
    }
}

impl Stage for Image {
    fn image(&self) -> &str {
        &self.image
    }
    fn name(&self, _position: i32) -> String {
        String::from("runtime")
    }
    fn user(&self) -> Option<String> {
        match self.user.as_ref() {
            Some(user) => Some(user.to_string()),
            None => {
                match self.image.as_str() {
                    "scratch" => None,
                    _ => Some(String::from("1000"))
                }
            },
        }
    }
    fn workdir(&self) -> Option<&String> {
        self.workdir.as_ref()
    }
    fn envs(&self) -> Option<&HashMap<String, String>> {
        self.envs.as_ref()
    }
    fn artifacts(&self) -> Option<&Vec<Artifact>> {
        self.artifacts.as_ref()
    }
    fn adds(&self) -> Option<&Vec<String>> {
        self.adds.as_ref()
    }
    fn root_script(&self) -> Option<&Vec<String>> {
        self.root_script.as_ref()
    }
    fn script(&self) -> Option<&Vec<String>> {
        self.script.as_ref()
    }
    fn caches(&self) -> Option<&Vec<String>> {
        self.caches.as_ref()
    }

    fn additionnal_generation(&self, buffer: &mut String) {
        if let Some(ref entrypoint) = self.entrypoint {
            buffer.push_str(
                format!("ENTRYPOINT {}", serde_json::to_string(entrypoint).unwrap()).as_str(),
            );
        }
        if let Some(ref cmd) = self.cmd {
            buffer.push_str(format!("CMD {}", serde_json::to_string(cmd).unwrap()).as_str());
        }
    }
}

impl Image {
    pub fn ignores(&self) -> Option<&Vec<String>> {
        self.ignores.as_ref()
    }
}

// pub trait Copy {

// }
// impl Copy for String {}
// impl Copy for CopyFull {}

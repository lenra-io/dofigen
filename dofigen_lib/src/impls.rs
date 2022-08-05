use std::{collections::HashMap, io::Result};

use crate::structs::{Artifact, Builder, Image, Root};

pub trait ScriptRunner {
    fn script(&self) -> Option<&Vec<String>>;
    fn caches(&self) -> Option<&Vec<String>>;
    fn has_script(&self) -> bool {
        if let Some(script) = self.script() {
            return script.len() > 0;
        }
        false
    }
    fn add_script(&self, buffer: &mut String, uid: u16, gid: u16) {
        if let Some(script) = self.script() {
            buffer.push_str("RUN ");
            if let Some(ref paths) = self.caches() {
                paths.iter().for_each(|path| {
                    buffer.push_str(
                        format!(
                            "\\\n\t--mount=type=cache,sharing=locked,uid={},gid={},target={}",
                            uid, gid, path
                        )
                        .as_str(),
                    )
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
    }
}

impl ScriptRunner for Builder {
    fn script(&self) -> Option<&Vec<String>> {
        self.script.as_ref()
    }
    fn caches(&self) -> Option<&Vec<String>> {
        self.caches.as_ref()
    }
}

impl ScriptRunner for Image {
    fn script(&self) -> Option<&Vec<String>> {
        self.script.as_ref()
    }
    fn caches(&self) -> Option<&Vec<String>> {
        self.caches.as_ref()
    }
}

impl ScriptRunner for Root {
    fn script(&self) -> Option<&Vec<String>> {
        self.script.as_ref()
    }
    fn caches(&self) -> Option<&Vec<String>> {
        self.caches.as_ref()
    }
}

/** Represents a Dockerfile stage */
pub trait Stage: ScriptRunner {
    fn image(&self) -> &str;
    fn name(&self, position: i32) -> String;
    fn user(&self) -> Option<String>;
    fn workdir(&self) -> Option<&String>;
    fn envs(&self) -> Option<&HashMap<String, String>>;
    fn artifacts(&self) -> Option<&Vec<Artifact>>;
    fn adds(&self) -> Option<&Vec<String>>;
    fn root(&self) -> Option<&Root>;

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
                        "COPY --link --chown=1000:1000 --from={builder} \"{source}\" \"{destination}\"\n",
                        builder = artifact.builder,
                        source = artifact.source,
                        destination = artifact.destination
                    )
                })
                .for_each(|artifact| buffer.push_str(&artifact.as_str()));
        }

        // Root script
        let is_root = if let Some(root) = self.root() {
            root.has_script()
        } else {
            false
        };
        if is_root {
            buffer.push_str("USER 0\n");
            self.root().unwrap().add_script(buffer, 0, 0);
        }

        // Runtime user
        let has_script = self.has_script();
        let user: Option<String> = match self.user() {
            Some(u) => Some(u),
            None => {
                if is_root && has_script && self.image() != "scratch" {
                    Some(String::from("1000"))
                } else {
                    None
                }
            }
        };
        if user.is_some() {
            buffer.push_str(format!("USER {}\n", user.unwrap()).as_str());
        }

        // Script
        if has_script {
            self.add_script(buffer, 1000, 1000);
        }

        self.additionnal_generation(buffer);

        previous_builders.push(name);
        Ok(())
    }

    fn additionnal_generation(&self, _buffer: &mut String) {}
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
    fn root(&self) -> Option<&Root> {
        self.root.as_ref()
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
            None => match self.image.as_str() {
                "scratch" => None,
                _ => Some(String::from("1000")),
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
    fn root(&self) -> Option<&Root> {
        self.root.as_ref()
    }

    fn additionnal_generation(&self, buffer: &mut String) {
        if let Some(ports) = &self.ports {
            ports
                .iter()
                .for_each(|port| buffer.push_str(format!("EXPOSE {}\n", port).as_str()));
        }
        if let Some(healthcheck) = &self.healthcheck {
            if let Some(cmd) = &healthcheck.cmd {
                buffer.push_str("HEALTHCHECK ");
                if let Some(interval) = &healthcheck.interval {
                    buffer.push_str(format!("--interval={} ", interval).as_str());
                }
                if let Some(timeout) = &healthcheck.timeout {
                    buffer.push_str(format!("--timeout={} ", timeout).as_str());
                }
                if let Some(start) = &healthcheck.start {
                    buffer.push_str(format!("--start-period={} ", start).as_str());
                }
                if let Some(retries) = &healthcheck.retries {
                    buffer.push_str(format!("--retries={} ", retries).as_str());
                }
                buffer.push_str(format!("CMD {}\n", cmd).as_str());
            }
        }
        if let Some(ref entrypoint) = self.entrypoint {
            buffer.push_str(
                format!(
                    "ENTRYPOINT {}\n",
                    serde_json::to_string(entrypoint).unwrap()
                )
                .as_str(),
            );
        }
        if let Some(ref cmd) = self.cmd {
            buffer.push_str(format!("CMD {}\n", serde_json::to_string(cmd).unwrap()).as_str());
        }
    }
}

impl Image {
    pub fn ignores(&self) -> Option<&Vec<String>> {
        self.ignores.as_ref()
    }
}

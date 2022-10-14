use std::io::Result;

use crate::{
    runners::ScriptRunner,
    structs::{Builder, Image},
};

pub trait StageGenerator: ScriptRunner {
    fn name(&self, position: i32) -> String;
    fn user(&self) -> Option<String>;
    fn additionnal_generation(&self, _buffer: &mut String) {}
}
pub trait Stage: StageGenerator {
    fn generate(&self, buffer: &mut String, previous_builders: &mut Vec<String>) -> Result<()>;
}

impl StageGenerator for Builder {
    fn name(&self, position: i32) -> String {
        match self.name.as_ref() {
            Some(name) => String::from(name),
            None => format!("builder-{}", position),
        }
    }
    fn user(&self) -> Option<String> {
        self.user.clone()
    }
}

impl StageGenerator for Image {
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
    fn additionnal_generation(&self, buffer: &mut String) {
        if let Some(ports) = &self.ports {
            ports
                .iter()
                .for_each(|port| buffer.push_str(format!("EXPOSE {}\n", port).as_str()));
        }
        if let Some(healthcheck) = &self.healthcheck {
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
            buffer.push_str(format!("CMD {}\n", healthcheck.cmd).as_str());
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

macro_rules! impl_Stage {
    (for $($t:ty),+) => {
        $(impl Stage for $t {
            fn generate(&self, buffer: &mut String, previous_builders: &mut Vec<String>) -> Result<()> {
                let name = self.name(previous_builders.len().try_into().unwrap());
                buffer.push_str(format!("\n# {}\nFROM {} as {}\n", name, self.image, name).as_str());

                // Set env variables
                if let Some(ref envs) = self.envs {
                    buffer.push_str("ENV ");
                    envs.iter().for_each(|(key, value)| {
                        if (key != "exec_timeout") {
                            buffer.push_str(format!("\\\n    {}=\"{}\"", key, value).as_str())
                        } else {
                            println!("Cannot set exec_timeout value.")
                        }
                    });
                    buffer.push_str("\\\n    exec_timeout=\"0\"\n");
                    buffer.push_str("\n");
                    // Set timetout to 0

                }



                // Set workdir
                if let Some(ref workdir) = self.workdir {
                    buffer.push_str(format!("WORKDIR {}\n", workdir).as_str());
                }

                // Add sources
                if let Some(ref adds) = self.adds {
                    adds.iter()
                        .map(|add| format!("ADD --link {} ./\n", add))
                        .for_each(|add| buffer.push_str(&add.as_str()));
                }

                // Copy build artifacts
                if let Some(ref artifacts) = self.artifacts {
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
                let is_root = if let Some(root) = &self.root {
                    if root.has_script() {
                        buffer.push_str("USER 0\n");
                        root.add_script(buffer, 0, 0);
                        true
                    }
                    else {
                    false
                    }
                } else {
                    false
                };

                // Runtime user
                let has_script = self.has_script();
                let user: Option<String> = match self.user() {
                    Some(u) => Some(u),
                    None => {
                        if is_root && has_script && self.image != "scratch" {
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
        })*
    }
}

impl_Stage!(for Builder, Image);

impl Image {
    pub fn ignores(&self) -> Option<&Vec<String>> {
        self.ignores.as_ref()
    }
}

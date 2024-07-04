use crate::{
    generator::{DockerfileGenerator, GenerationContext}, runners::ScriptRunner, structs::{Builder, Image}, ImageName, Result
};

pub trait StageGenerator: ScriptRunner {
    fn name(&self, position: i32) -> String;
    fn from(&self, context: &GenerationContext) -> ImageName;
    fn user(&self, context: &GenerationContext) -> Option<String>;
    fn additionnal_generation(&self, _context: &GenerationContext) -> Result<String> {
        Ok("".to_string())
    }
}
pub trait Stage: StageGenerator {
    fn generate(
        &self,
        previous_builders: &mut Vec<String>,
        context: &GenerationContext,
    ) -> Result<String>;
}

impl StageGenerator for Builder {
    fn name(&self, position: i32) -> String {
        match self.name.as_ref() {
            Some(name) => String::from(name),
            None => format!("builder-{}", position),
        }
    }
    fn from(&self, context: &GenerationContext) -> Result<String> {
        self.from.generate_content(context)
    }

    fn user(&self, context: &GenerationContext) -> Option<String> {
        self.user.clone().or(context.user.clone())
    }
}

impl StageGenerator for Image {
    fn name(&self, _position: i32) -> String {
        String::from("runtime")
    }
    fn from(&self, context: &GenerationContext) -> Result<String> {
        if let Some(image_name) = &self.from {
            image_name.generate_content(context)
        } else {
            Ok(String::from("scratch"))
        }
    }
    fn user(&self, _context: &GenerationContext) -> Option<String> {
        self.user.clone().or(Some(String::from("1000")))
    }
    fn additionnal_generation(&self, _context: &GenerationContext) -> Result<String> {
        let mut buffer = String::new();
        if let Some(ports) = &self.expose {
            ports
                .clone()
                .to_vec()
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
            buffer.push_str(format!("ENTRYPOINT {}\n", string_vec_to_string(entrypoint)).as_str());
        }
        if let Some(ref cmd) = self.cmd {
            buffer.push_str(format!("CMD {}\n", string_vec_to_string(cmd)).as_str());
        }
        Ok(buffer)
    }
}

macro_rules! impl_Stage {
    (for $($t:ty),+) => {
        $(impl Stage for $t {
            fn generate(&self, previous_builders: &mut Vec<String>, context: &GenerationContext) -> Result<String> {
                let mut buffer = String::new();
                let user = self.user(context);
                let context = &GenerationContext {
                    user: user.clone(),
                    ..context.clone()
                };
                let name = self.name(previous_builders.len().try_into().unwrap());
                buffer.push_str(format!("\n# {}\nFROM {} AS {}\n", name, self.from(context)?, name).as_str());

                // Set env variables
                if let Some(ref envs) = self.env {
                    buffer.push_str("ENV ");
                    envs.iter().for_each(|(key, value)| {
                        buffer.push_str(format!("\\\n    {}=\"{}\"", key, value).as_str())
                    });
                    buffer.push_str("\n");
                }

                // Set workdir
                if let Some(ref workdir) = self.workdir {
                    buffer.push_str(format!("WORKDIR {}\n", workdir).as_str());
                }

                // Add sources
                if let Some(ref adds) = self.copy {
                    buffer.push_str(adds
                    .clone()
                    .to_vec()
                    .iter()
                    .map(|add| add.to_dockerfile_content(context).map(|add| format!("{}\n", add)))
                    .collect::<Result<Vec<String>>>()?
                    .join("").as_str());
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
                                "COPY --link --chown={chown} --from={builder} \"{source}\" \"{target}\"\n",
                                chown = context.user.clone().unwrap_or("1000:1000".to_string()),
                                builder = artifact.builder,
                                source = artifact.source,
                                target = artifact.target
                            )
                        })
                        .for_each(|artifact| buffer.push_str(&artifact.as_str()));
                }

                // Root script
                if let Some(root) = &self.root {
                    if root.has_script() {
                        buffer.push_str("USER 0\n");
                        root.add_script(&mut buffer, &GenerationContext {
                            user: Some("0".to_string()),
                            ..context.clone()
                        });
                    }
                };

                // Runtime user
                if let Some(user) = user {
                    buffer.push_str(format!("USER {}\n", user).as_str());
                }

                // Script
                if self.has_script() {
                    self.add_script(&mut buffer, context);
                }

                buffer.push_str(self.additionnal_generation(context)?.as_str());

                previous_builders.push(name);
                Ok(buffer)
            }
        })*
    }
}

impl_Stage!(for Builder, Image);

pub fn string_vec_to_string(string_vec: &Vec<String>) -> String {
    format!(
        "[{}]",
        string_vec
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<String>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use crate::ImageName;

    use super::*;

    #[test]
    fn test_builder_name_with_name() {
        let builder = Builder {
            name: Some(String::from("my-builder")),
            ..Default::default()
        };
        let position = 1;
        let name = builder.name(position);
        assert_eq!(name, "my-builder");
    }

    #[test]
    fn test_builder_name_without_name() {
        let builder = Builder::default();
        let position = 2;
        let name = builder.name(position);
        assert_eq!(name, "builder-2");
    }

    #[test]
    fn test_builder_user_with_user() {
        let builder = Builder {
            user: Some(String::from("my-user")),
            ..Default::default()
        };
        let user = builder.user(&GenerationContext::default());
        assert_eq!(user, Some(String::from("my-user")));
    }

    #[test]
    fn test_builder_user_without_user() {
        let builder = Builder::default();
        let user = builder.user(&GenerationContext::default());
        assert_eq!(user, None);
    }

    #[test]
    fn test_image_name() {
        let image = Image {
            from: Some(ImageName {
                path: String::from("my-image"),
                ..Default::default()
            }),
            ..Default::default()
        };
        let position = 3;
        let name = image.name(position);
        assert_eq!(name, "runtime");
    }

    #[test]
    fn test_image_user_with_user() {
        let image = Image {
            user: Some(String::from("my-user")),
            from: Some(ImageName {
                path: String::from("my-image"),
                ..Default::default()
            }),
            ..Default::default()
        };
        let user = image.user(&GenerationContext::default());
        assert_eq!(user, Some(String::from("my-user")));
    }

    #[test]
    fn test_image_user_without_user() {
        let image = Image {
            from: Some(ImageName {
                path: String::from("my-image"),
                ..Default::default()
            }),
            ..Default::default()
        };
        let user = image.user(&GenerationContext::default());
        assert_eq!(user, Some(String::from("1000")));
    }
}

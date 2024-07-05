use crate::{
    dockerfile::{DockerfileInsctruction, InstructionOption, InstructionOptionOption},
    generator::{GenerationContext, LINE_SEPARATOR},
    structs::{Builder, Image, Root},
    Result,
};

pub trait ScriptRunner {
    fn script(&self) -> Option<&Vec<String>>;
    fn caches(&self) -> Option<&Vec<String>>;

    fn to_run_inscruction(
        &self,
        context: &GenerationContext,
    ) -> Result<Option<DockerfileInsctruction>> {
        if let Some(script) = self.script() {
            let script = script.join(" &&\n");
            let script_lines = script.lines().collect::<Vec<&str>>();
            let content = match script_lines.len() {
                0 => {
                    return Ok(None);
                }
                1 => script_lines[0].to_string(),
                _ => script_lines.join(LINE_SEPARATOR),
                // _ => format!("<<EOF\n{}\nEOF", script_lines.join("\n")),
            };
            let mut options = vec![];
            if let Some(caches) = self.caches() {
                caches.iter().for_each(|cache| {
                    let mut cache_options = vec![
                        InstructionOptionOption::new("type", "cache"),
                        InstructionOptionOption::new("target", cache),
                        InstructionOptionOption::new("sharing", "locked"),
                    ];
                    if let Some(user) = &context.user {
                        if let Some(uid) = user.uid() {
                            cache_options
                                .push(InstructionOptionOption::new("uid", &uid.to_string()));
                        }
                        if let Some(gid) = user.gid() {
                            cache_options
                                .push(InstructionOptionOption::new("gid", &gid.to_string()));
                        }
                    }
                    options.push(InstructionOption::WithOptions(
                        "mount".to_string(),
                        cache_options,
                    ));
                });
            }
            return Ok(Some(DockerfileInsctruction {
                command: "RUN".to_string(),
                content,
                options,
            }));
        }
        Ok(None)
    }
}

macro_rules! impl_ScriptRunner {
    (for $($t:ty),+) => {
        $(impl ScriptRunner for $t {
            fn script(&self) -> Option<&Vec<String>> {
                self.run.as_ref()
            }
            fn caches(&self) -> Option<&Vec<String>> {
                self.cache.as_ref()
            }
        })*
    }
}

impl_ScriptRunner!(for Builder, Image, Root);

#[cfg(test)]
mod test {

    use crate::User;

    use super::*;

    #[test]
    fn to_run_inscruction_with_script() {
        let builder = Builder {
            run: Some(vec!["echo Hello".to_string()]),
            ..Default::default()
        };
        assert_eq!(
            builder
                .to_run_inscruction(&GenerationContext::default())
                .unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".to_string(),
                content: "echo Hello".to_string(),
                options: vec![],
            })
        );
    }

    #[test]
    fn to_run_inscruction_without_script() {
        let builder = Builder {
            ..Default::default()
        };
        assert_eq!(
            builder
                .to_run_inscruction(&GenerationContext::default())
                .unwrap(),
            None
        );
    }

    #[test]
    fn to_run_inscruction_with_empty_script() {
        let builder = Builder {
            run: Some(vec![]),
            ..Default::default()
        };
        assert_eq!(
            builder
                .to_run_inscruction(&GenerationContext::default())
                .unwrap(),
            None
        );
    }

    #[test]
    fn to_run_inscruction_with_script_and_caches_with_named_user() {
        let builder = Builder {
            run: Some(vec!["echo Hello".to_string()]),
            cache: Some(vec!["/path/to/cache".to_string()]),
            ..Default::default()
        };
        let context = GenerationContext {
            user: Some(User::new("test")),
            ..Default::default()
        };
        assert_eq!(
            builder.to_run_inscruction(&context).unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".to_string(),
                content: "echo Hello".to_string(),
                options: vec![InstructionOption::WithOptions(
                    "mount".to_string(),
                    vec![
                        InstructionOptionOption::new("type", "cache"),
                        InstructionOptionOption::new("target", "/path/to/cache"),
                        InstructionOptionOption::new("sharing", "locked"),
                    ],
                )],
            })
        );
    }

    #[test]
    fn to_run_inscruction_with_script_and_caches_with_uid_user() {
        let builder = Builder {
            run: Some(vec!["echo Hello".to_string()]),
            cache: Some(vec!["/path/to/cache".to_string()]),
            ..Default::default()
        };
        let context = GenerationContext {
            user: Some(User::new("1000")),
            ..Default::default()
        };
        assert_eq!(
            builder.to_run_inscruction(&context).unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".to_string(),
                content: "echo Hello".to_string(),
                options: vec![InstructionOption::WithOptions(
                    "mount".to_string(),
                    vec![
                        InstructionOptionOption::new("type", "cache"),
                        InstructionOptionOption::new("target", "/path/to/cache"),
                        InstructionOptionOption::new("sharing", "locked"),
                        InstructionOptionOption::new("uid", "1000"),
                        InstructionOptionOption::new("gid", "1000"),
                    ],
                )],
            })
        );
    }

    #[test]
    fn to_run_inscruction_with_script_and_caches_with_uid_user_without_group() {
        let builder = Builder {
            run: Some(vec!["echo Hello".to_string()]),
            cache: Some(vec!["/path/to/cache".to_string()]),
            ..Default::default()
        };
        let context = GenerationContext {
            user: Some(User::new_without_group("1000")),
            ..Default::default()
        };
        assert_eq!(
            builder.to_run_inscruction(&context).unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".to_string(),
                content: "echo Hello".to_string(),
                options: vec![InstructionOption::WithOptions(
                    "mount".to_string(),
                    vec![
                        InstructionOptionOption::new("type", "cache"),
                        InstructionOptionOption::new("target", "/path/to/cache"),
                        InstructionOptionOption::new("sharing", "locked"),
                        InstructionOptionOption::new("uid", "1000"),
                    ],
                )],
            })
        );
    }
}

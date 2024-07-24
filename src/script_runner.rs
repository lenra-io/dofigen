use crate::{
    dockerfile_struct::{DockerfileInsctruction, InstructionOption, InstructionOptionOption},
    dofigen_struct::{Builder, Image, Root},
    generator::{GenerationContext, LINE_SEPARATOR},
    Result,
};

pub trait ScriptRunner {
    fn script(&self) -> Vec<String>;
    fn caches(&self) -> Vec<String>;

    fn to_run_inscruction(
        &self,
        context: &GenerationContext,
    ) -> Result<Option<DockerfileInsctruction>> {
        let script = self.script();
        if !script.is_empty() {
            let script = script.join(" &&\n");
            let script_lines = script.lines().collect::<Vec<&str>>();
            let content = match script_lines.len() {
                0 => {
                    return Ok(None);
                }
                1 => script_lines[0].into(),
                _ => script_lines.join(LINE_SEPARATOR),
                // _ => format!("<<EOF\n{}\nEOF", script_lines.join("\n")),
            };
            let mut options = vec![];
            self.caches().iter().for_each(|cache| {
                let mut cache_options = vec![
                    InstructionOptionOption::new("type", "cache"),
                    InstructionOptionOption::new("target", cache),
                    InstructionOptionOption::new("sharing", "locked"),
                ];
                if let Some(user) = &context.user {
                    if let Some(uid) = user.uid() {
                        cache_options.push(InstructionOptionOption::new("uid", &uid.to_string()));
                    }
                    if let Some(gid) = user.gid() {
                        cache_options.push(InstructionOptionOption::new("gid", &gid.to_string()));
                    }
                }
                options.push(InstructionOption::WithOptions(
                    "mount".into(),
                    cache_options,
                ));
            });
            return Ok(Some(DockerfileInsctruction {
                command: "RUN".into(),
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
            fn script(&self) -> Vec<String> {
                self.run.to_vec()
            }
            fn caches(&self) -> Vec<String> {
                self.cache.to_vec()
            }
        })*
    }
}

impl_ScriptRunner!(for Builder, Image, Root);

#[cfg(test)]
mod test {
    use super::*;
    use crate::User;

    #[test]
    fn to_run_inscruction_with_script() {
        let builder = Builder {
            run: vec!["echo Hello".into()].into(),
            ..Default::default()
        };
        assert_eq!(
            builder
                .to_run_inscruction(&GenerationContext::default())
                .unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".into(),
                content: "echo Hello".into(),
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
            run: vec![].into(),
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
            run: vec!["echo Hello".into()].into(),
            cache: vec!["/path/to/cache".into()].into(),
            ..Default::default()
        };
        let context = GenerationContext {
            user: Some(User::new("test")),
            ..Default::default()
        };
        assert_eq!(
            builder.to_run_inscruction(&context).unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".into(),
                content: "echo Hello".into(),
                options: vec![InstructionOption::WithOptions(
                    "mount".into(),
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
            run: vec!["echo Hello".into()].into(),
            cache: vec!["/path/to/cache".into()].into(),
            ..Default::default()
        };
        let context = GenerationContext {
            user: Some(User::new("1000")),
            ..Default::default()
        };
        assert_eq!(
            builder.to_run_inscruction(&context).unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".into(),
                content: "echo Hello".into(),
                options: vec![InstructionOption::WithOptions(
                    "mount".into(),
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
            run: vec!["echo Hello".into()].into(),
            cache: vec!["/path/to/cache".into()].into(),
            ..Default::default()
        };
        let context = GenerationContext {
            user: Some(User::new_without_group("1000")),
            ..Default::default()
        };
        assert_eq!(
            builder.to_run_inscruction(&context).unwrap(),
            Some(DockerfileInsctruction {
                command: "RUN".into(),
                content: "echo Hello".into(),
                options: vec![InstructionOption::WithOptions(
                    "mount".into(),
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

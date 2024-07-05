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
mod tests {

    use super::*;

    // TODO recreate unit tests

    // #[test]
    // fn test_has_script_with_script() {
    //     let builder = Builder {
    //         run: Some(vec!["echo Hello".to_string()]),
    //         ..Default::default()
    //     };
    //     assert_eq!(builder.has_script(), true);
    // }

    // #[test]
    // fn test_has_script_without_script() {
    //     let builder = Builder {
    //         ..Default::default()
    //     };
    //     assert_eq!(builder.has_script(), false);
    // }

    // #[test]
    // fn test_has_script_with_empty_script() {
    //     let builder = Builder {
    //         run: Some(vec![]),
    //         ..Default::default()
    //     };
    //     assert_eq!(builder.has_script(), false);
    // }

    // #[test]
    // fn test_has_script_without_script_with_cache() {
    //     let builder = Builder {
    //         cache: Some(vec!["/path/to/cache".to_string()]),
    //         ..Default::default()
    //     };
    //     assert_eq!(builder.has_script(), false);
    // }

    // #[test]
    // fn test_add_script_with_script_and_caches() {
    //     let mut buffer = String::new();
    //     let builder = Builder {
    //         run: Some(vec!["echo Hello".to_string()]),
    //         cache: Some(vec!["/path/to/cache".to_string()]),
    //         ..Default::default()
    //     };
    //     builder.add_script(&mut buffer, &GenerationContext::default());
    //     assert_eq!(
    //         buffer,
    //         "RUN \\\n    --mount=type=cache,sharing=locked,target=/path/to/cache \\\n    echo Hello\n"
    //     );
    // }

    // #[test]
    // fn test_add_script_with_script_and_caches_with_user() {
    //     let mut buffer = String::new();
    //     let builder = Builder {
    //         run: Some(vec!["echo Hello".to_string()]),
    //         cache: Some(vec!["/path/to/cache".to_string()]),
    //         ..Default::default()
    //     };
    //     builder.add_script(
    //         &mut buffer,
    //         &GenerationContext {
    //             user: Some("1000".to_string()),
    //             ..Default::default()
    //         },
    //     );
    //     assert_eq!(
    //         buffer,
    //         "RUN \\\n    --mount=type=cache,sharing=locked,uid=1000,target=/path/to/cache \\\n    echo Hello\n"
    //     );
    // }

    // #[test]
    // fn test_add_script_with_script_without_caches() {
    //     let mut buffer = String::new();
    //     let builder = Builder {
    //         run: Some(vec!["echo Hello".to_string()]),
    //         ..Default::default()
    //     };
    //     builder.add_script(&mut buffer, &GenerationContext::default());
    //     assert_eq!(buffer, "RUN \\\n    echo Hello\n");
    // }

    // #[test]
    // fn test_add_script_without_script() {
    //     let mut buffer = String::new();
    //     let builder = Builder {
    //         ..Default::default()
    //     };
    //     builder.add_script(&mut buffer, &GenerationContext::default());
    //     assert_eq!(buffer, "");
    // }

    // #[test]
    // fn test_add_script_with_empty_script() {
    //     let mut buffer = String::new();
    //     let builder = Builder {
    //         run: Some(vec![]),
    //         ..Default::default()
    //     };
    //     builder.add_script(&mut buffer, &GenerationContext::default());
    //     assert_eq!(buffer, "");
    // }
}

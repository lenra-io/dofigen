use crate::structs::{Builder, Image, Root};

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
            if script.is_empty() {
                return;
            }
            let mut lines: Vec<String> = script
                .join(" &&\n")
                .lines()
                .map(str::to_string)
                .collect::<Vec<String>>();
            if let Some(ref paths) = self.caches() {
                lines.splice(
                    0..0,
                    paths
                        .iter()
                        .map(|path| {
                            format!(
                                "--mount=type=cache,sharing=locked,uid={},gid={},target={}",
                                uid, gid, path
                            )
                        })
                        .collect::<Vec<String>>(),
                );
            }
            lines.insert(0, "RUN".to_string());
            buffer.push_str(&lines.join(" \\\n    "));
            buffer.push_str("\n");
        }
    }
}

macro_rules! impl_ScriptRunner {
    (for $($t:ty),+) => {
        $(impl ScriptRunner for $t {
            fn script(&self) -> Option<&Vec<String>> {
                self.script.as_ref()
            }
            fn caches(&self) -> Option<&Vec<String>> {
                self.caches.as_ref()
            }
        })*
    }
}

impl_ScriptRunner!(for Builder, Image, Root);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_script_with_script() {
        let builder = Builder {
            script: Some(vec!["echo Hello".to_string()]),
            caches: None,
            ..Default::default()
        };
        assert_eq!(builder.has_script(), true);
    }

    #[test]
    fn test_has_script_without_script() {
        let builder = Builder {
            script: None,
            caches: None,
            ..Default::default()
        };
        assert_eq!(builder.has_script(), false);
    }

    #[test]
    fn test_has_script_with_empty_script() {
        let builder = Builder {
            script: Some(vec![]),
            caches: None,
            ..Default::default()
        };
        assert_eq!(builder.has_script(), false);
    }

    #[test]
    fn test_has_script_without_script_with_cache() {
        let builder = Builder {
            script: None,
            caches: Some(vec!["/path/to/cache".to_string()]),
            ..Default::default()
        };
        assert_eq!(builder.has_script(), false);
    }

    #[test]
    fn test_add_script_with_script_and_caches() {
        let mut buffer = String::new();
        let builder = Builder {
            script: Some(vec!["echo Hello".to_string()]),
            caches: Some(vec!["/path/to/cache".to_string()]),
            ..Default::default()
        };
        builder.add_script(&mut buffer, 1000, 1000);
        assert_eq!(
            buffer,
            "RUN \\\n    --mount=type=cache,sharing=locked,uid=1000,gid=1000,target=/path/to/cache \\\n    echo Hello\n"
        );
    }

    #[test]
    fn test_add_script_with_script_without_caches() {
        let mut buffer = String::new();
        let builder = Builder {
            script: Some(vec!["echo Hello".to_string()]),
            caches: None,
            ..Default::default()
        };
        builder.add_script(&mut buffer, 1000, 1000);
        assert_eq!(buffer, "RUN \\\n    echo Hello\n");
    }

    #[test]
    fn test_add_script_without_script() {
        let mut buffer = String::new();
        let builder = Builder {
            script: None,
            caches: None,
            ..Default::default()
        };
        builder.add_script(&mut buffer, 1000, 1000);
        assert_eq!(buffer, "");
    }

    #[test]
    fn test_add_script_with_empty_script() {
        let mut buffer = String::new();
        let builder = Builder {
            script: Some(vec![]),
            caches: None,
            ..Default::default()
        };
        builder.add_script(&mut buffer, 1000, 1000);
        assert_eq!(buffer, "");
    }
}

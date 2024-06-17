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

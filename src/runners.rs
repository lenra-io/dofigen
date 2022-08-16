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
            buffer.push_str("RUN ");
            if let Some(ref paths) = self.caches() {
                paths.iter().for_each(|path| {
                    buffer.push_str(
                        format!(
                            "\\\n    --mount=type=cache,sharing=locked,uid={},gid={},target={}",
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
                buffer.push_str(format!("\\\n    {}", cmd).as_str())
            });
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

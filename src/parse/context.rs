use std::collections::HashMap;

use crate::{
    DockerFileInsctruction, Dofigen, DofigenPatch, Error, LintMessage, Result, Run, Stage,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ParseContext {
    pub dofigen: Dofigen,
    pub messages: Vec<LintMessage>,
    pub current_stage_name: Option<String>,
    pub current_stage: Option<Stage>,
    pub current_root: Option<Run>,
    pub last_inscruction: Option<DockerFileInsctruction>,
    pub current_shell: Option<Vec<String>>,
    pub builders: HashMap<String, Stage>,
    pub builder_dofigen_patches: HashMap<String, DofigenPatch>,
}

impl ParseContext {
    pub fn current_stage<'a>(
        &'a mut self,
        instruction: Option<&'a DockerFileInsctruction>,
    ) -> Result<&'a mut Stage> {
        self.current_stage
            .as_mut()
            .ok_or(no_from_error(instruction))
    }

    pub fn current_stage_name(
        &self,
        instruction: Option<&DockerFileInsctruction>,
    ) -> Result<String> {
        self.current_stage_name
            .clone()
            .ok_or(no_from_error(instruction))
    }

    pub fn apply_root(&mut self) -> Result<()> {
        if let Some(stage) = self.current_stage.as_mut() {
            if let Some(root) = self.current_root.as_ref() {
                stage.root = Some(root.clone());
                self.current_root = None;
            }
        }
        Ok(())
    }
}

fn no_from_error(instruction: Option<&DockerFileInsctruction>) -> Error {
    if let Some(ins) = instruction {
        Error::Custom(format!("No FROM instruction found before line: {:?}", ins))
    } else {
        Error::Custom("No FROM instruction found".to_string())
    }
}

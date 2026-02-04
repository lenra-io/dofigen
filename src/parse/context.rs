use std::collections::HashMap;

use crate::{
    CopyResource, DockerFileInsctruction, DockerFileLine, Dofigen, DofigenPatch, Error,
    FromContext, LintMessage, Result, Run, Stage,
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
    /// Names of the Dockerfile stages in the order they were defined
    pub stage_names: Vec<String>,
    pub builder_dofigen_patches: HashMap<String, DofigenPatch>,
}

impl ParseContext {
    pub fn current_stage<'a>(
        &'a mut self,
        instruction: &'a DockerFileInsctruction,
    ) -> Result<&'a mut Stage> {
        self.current_stage.as_mut().ok_or(Error::Custom(format!(
            "No FROM instruction found before line: {:?}",
            instruction
        )))
    }

    pub fn current_dofigen_patch<'a>(
        &'a mut self,
        instruction: &'a DockerFileInsctruction,
    ) -> Result<&'a mut DofigenPatch> {
        self.current_stage(instruction)?; // Ensure there is a current stage
        let name = self
            .current_stage_name
            .clone()
            .unwrap_or("runtime".to_string());
        Ok(self
            .builder_dofigen_patches
            .entry(name)
            .or_insert_with(DofigenPatch::default))
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

    pub fn add_current_stage_as_builder(&mut self, name: String) -> Result<()> {
        let stage = self.current_stage.clone().ok_or(Error::Custom(
            "No current stage to add as builder".to_string(),
        ))?;
        self.current_stage = None;
        self.dofigen.builders.insert(name, stage);
        Ok(())
    }

    pub fn get_current_message_path(&self, line: &DockerFileLine) -> Vec<String> {
        let mut path = vec![];
        if let Some(stage_name) = &self.current_stage_name {
            path.push(stage_name.clone());
        }
        if let DockerFileLine::Instruction(DockerFileInsctruction { command, .. }) = line {
            path.push(command.to_string());
        }
        path
    }

    pub fn split_current_stage(&mut self) -> Result<()> {
        let name = self
            .current_stage_name
            .clone()
            .unwrap_or("runtime".to_string());
        let builder_name_base = format!("{}-builder-", name);
        let mut counter = 1;
        let mut builder_name = format!("{}{}", builder_name_base, counter);
        while self.dofigen.builders.contains_key(&builder_name) {
            counter += 1;
            builder_name = format!("{}{}", builder_name_base, counter);
        }
        self.add_current_stage_as_builder(builder_name.clone())?;
        self.current_stage = Some(Stage {
            from: FromContext::FromBuilder(builder_name),
            ..Default::default()
        });
        Ok(())
    }

    pub fn add_copy<'a>(
        &'a mut self,
        instruction: &'a DockerFileInsctruction,
        copy: CopyResource,
    ) -> Result<()> {
        let stage = self.current_stage(instruction)?;
        if !(stage.run.is_empty() && stage.root.is_none()) {
            self.split_current_stage()?;
            self.current_stage(instruction)?.copy.push(copy);
        } else {
            stage.copy.push(copy);
        }
        Ok(())
    }
}

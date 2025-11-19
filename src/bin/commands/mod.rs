use dofigen_lib::{Dofigen, DofigenContext, Error, Resource, Result, lock::LockFile};
use std::path::PathBuf;

pub mod effective;
pub mod generate;
#[cfg(feature = "json_schema")]
pub mod schema;
pub mod update;

pub(crate) fn get_file_path(path: &Option<String>) -> Result<String> {
    if let Some(path) = path {
        Ok(path.clone())
    } else {
        let mut files = vec!["dofigen.yml", "dofigen.yaml", "dofigen.json"];
        files.retain(|f| std::path::Path::new(f).exists());
        if files.is_empty() {
            return Err(Error::Custom("No Dofigen file found".into()));
        }
        Ok(files[0].into())
    }
}

pub(crate) fn get_lockfile_path(path: String) -> Option<PathBuf> {
    if path == "-" {
        None
    } else {
        Some(PathBuf::from(path).with_extension("lock"))
    }
}

pub(crate) fn get_image_from_path(path: String, context: &mut DofigenContext) -> Result<Dofigen> {
    if path == "-" {
        context.parse_from_reader(std::io::stdin())
    } else {
        context.parse_from_resource(path.parse()?)
    }
}

pub(crate) fn load_lockfile(path: Option<PathBuf>) -> Option<LockFile> {
    path.map(|path| {
        if path.exists() {
            let mut context = DofigenContext::new();
            context.display_updates = false;
            Resource::File(path).load(&mut context).ok()
        } else {
            None
        }
    })
    .flatten()
}

use dofigen_lib::{
    from_file_path, from_reader, lock::LockFile, Image, LoadContext, Resource, Result,
};
use std::path::PathBuf;

pub mod effective;
pub mod generate;
#[cfg(feature = "json_schema")]
pub mod schema;
pub mod update;

pub(crate) fn get_file_path(path: &Option<String>) -> String {
    if let Some(path) = path {
        path.clone()
    } else {
        let mut files = vec!["dofigen.yml", "dofigen.yaml", "dofigen.json"];
        files.retain(|f| std::path::Path::new(f).exists());
        if files.is_empty() {
            eprintln!("No Dofigen file found");
            std::process::exit(1);
        }
        files[0].into()
    }
}

pub(crate) fn get_lockfile_path(path: String) -> Option<PathBuf> {
    if path == "-" {
        None
    } else {
        Some(PathBuf::from(path).with_extension("lock"))
    }
}

pub(crate) fn get_image_from_path(path: String) -> Result<Image> {
    if path == "-" {
        from_reader(std::io::stdin())
    } else {
        from_file_path(&PathBuf::from(path))
    }
}

pub(crate) fn get_image_from_cli_path(path: &Option<String>) -> Result<Image> {
    get_image_from_path(get_file_path(path))
}

pub(crate) fn load_lockfile(path: Option<PathBuf>) -> Option<LockFile> {
    path.map(|path| {
        if path.exists() {
            Resource::File(path).load(&mut LoadContext::new()).ok()
        } else {
            None
        }
    })
    .flatten()
}

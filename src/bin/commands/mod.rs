use dofigen_lib::{lock::LockFile, DofigenContext, Image, Resource, Result};
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

pub(crate) fn get_image_from_path(path: String, context: &mut DofigenContext) -> Result<Image> {
    if path == "-" {
        context.parse_from_reader(std::io::stdin())
    } else {
        context.parse_from_resource(path.parse()?)
    }
}

pub(crate) fn get_image_from_cli_path(
    path: &Option<String>,
    context: &mut DofigenContext,
) -> Result<Image> {
    get_image_from_path(get_file_path(path), context)
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

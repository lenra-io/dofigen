use dofigen_lib::{from_file_path, from_reader, Image, Result};
use std::path::PathBuf;

pub mod effective;
pub mod generate;
#[cfg(feature = "json_schema")]
pub mod schema;

pub(crate) fn load_image_from_cli_path(path: &Option<String>) -> Result<Image> {
    let file = if let Some(path) = path {
        path
    } else {
        let mut files = vec!["dofigen.yml", "dofigen.yaml", "dofigen.json"];
        files.retain(|f| std::path::Path::new(f).exists());
        if files.is_empty() {
            eprintln!("No Dofigen file found");
            std::process::exit(1);
        }
        &files[0].into()
    };
    if file == "-" {
        from_reader(std::io::stdin())
    } else {
        from_file_path(&PathBuf::from(file))
    }
}

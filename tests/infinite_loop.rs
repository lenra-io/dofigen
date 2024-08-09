use dofigen_lib::*;
use pretty_assertions_sorted::assert_eq_sorted;
use std::path::PathBuf;

#[test]
fn test_self_loop() {
    let path = PathBuf::from("tests/infinite_loop/self.yml");
    let image: Result<Image> = from_resource(Resource::File(path.clone()));

    assert!(image.is_err());

    let error = image.unwrap_err();

    let canonical_path = std::fs::canonicalize(&path).unwrap();
    let canonical_path = canonical_path.to_str().unwrap();

    assert_eq_sorted!(
        error.to_string(),
        format!("Circular dependency detected while loading resource File(\"{canonical_path}\") -> File(\"{canonical_path}\")")
    );
}

#[test]
fn test_a_b_loop() {
    let path = PathBuf::from("tests/infinite_loop/a.yml");
    let image: Result<Image> = from_resource(Resource::File(path.clone()));

    assert!(image.is_err());

    let error = image.unwrap_err();

    let a_path = std::fs::canonicalize(&path).unwrap();
    let a_path = a_path.to_str().unwrap();
    let b_path = std::fs::canonicalize("tests/infinite_loop/b.yml").unwrap();
    let b_path = b_path.to_str().unwrap();

    assert_eq_sorted!(
        error.to_string(),
        format!("Circular dependency detected while loading resource File(\"{a_path}\") -> File(\"{b_path}\") -> File(\"{a_path}\")")
    );
}

#[test]
fn test_stack_size() {
    let path = PathBuf::from("tests/infinite_loop/stack_size.yml");
    let image: Result<Image> = from_resource(Resource::File(path.clone()));

    assert!(image.is_err());

    let error = image.unwrap_err();

    let base_path = std::fs::canonicalize(&path).unwrap();
    let base_path = base_path.to_str().unwrap();

    let stack_files: String = (0..10)
        .map(|i| {
            std::fs::canonicalize(&format!("{}{}.yml", "tests/infinite_loop/stack_size_", i))
                .unwrap()
        })
        .map(|p| format!("File(\"{p}\")", p = p.to_str().unwrap()))
        .collect::<Vec<_>>()
        .join(" -> ");

    assert_eq_sorted!(
        error.to_string(),
        format!("Max load stack size exceeded while loading resource File(\"{base_path}\") -> {stack_files}")
    );
}

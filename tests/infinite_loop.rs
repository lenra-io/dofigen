use dofigen_lib::*;
use pretty_assertions_sorted::assert_eq_sorted;
use std::path::PathBuf;

#[test]
fn test_self_loop() {
    let path = PathBuf::from("tests/infinite_loop/self.yml");
    let dofigen: Result<Dofigen> =
        DofigenContext::new().parse_from_resource(Resource::File(path.clone()));

    assert!(dofigen.is_err());

    let error = dofigen.unwrap_err();

    assert_eq_sorted!(
        error.to_string(),
        format!(
            "Circular dependency detected while loading resource File(\"{path}\") -> File(\"{path}\")",
            path = path.to_str().unwrap()
        )
    );
}

#[test]
fn test_self_relative_loop() {
    let path = PathBuf::from("tests/infinite_loop/self_relative.yml");
    let dofigen: Result<Dofigen> =
        DofigenContext::new().parse_from_resource(Resource::File(path.clone()));

    assert!(dofigen.is_err());

    let error = dofigen.unwrap_err();

    assert_eq_sorted!(
        error.to_string(),
        format!(
            "Circular dependency detected while loading resource File(\"{path}\") -> File(\"{path}\")",
            path = path.to_str().unwrap()
        )
    );
}

#[test]
fn test_self_relative_from_parent_loop() {
    let path = PathBuf::from("tests/infinite_loop/self_relative_from_parent.yml");
    let dofigen: Result<Dofigen> =
        DofigenContext::new().parse_from_resource(Resource::File(path.clone()));

    assert!(dofigen.is_err());

    let error = dofigen.unwrap_err();

    assert_eq_sorted!(
        error.to_string(),
        format!(
            "Circular dependency detected while loading resource File(\"{path}\") -> File(\"{path}\")",
            path = path.to_str().unwrap()
        )
    );
}

#[test]
fn test_a_b_loop() {
    let path = PathBuf::from("tests/infinite_loop/a.yml");
    let dofigen: Result<Dofigen> =
        DofigenContext::new().parse_from_resource(Resource::File(path.clone()));

    assert!(dofigen.is_err());

    let error = dofigen.unwrap_err();

    let a_path = path.to_str().unwrap();
    let b_path = PathBuf::from("tests/infinite_loop/b.yml");
    let b_path = b_path.to_str().unwrap();

    assert_eq_sorted!(
        error.to_string(),
        format!(
            "Circular dependency detected while loading resource File(\"{a_path}\") -> File(\"{b_path}\") -> File(\"{a_path}\")"
        )
    );
}

#[test]
fn test_stack_size() {
    let path = PathBuf::from("tests/infinite_loop/stack_size.yml");
    let dofigen: Result<Dofigen> =
        DofigenContext::new().parse_from_resource(Resource::File(path.clone()));

    assert!(dofigen.is_err());

    let error = dofigen.unwrap_err();

    let base_path = path.to_str().unwrap();

    let stack_files: String = (0..10)
        .map(|i| PathBuf::from(&format!("{}{}.yml", "tests/infinite_loop/stack_size_", i)))
        .map(|p| format!("File(\"{p}\")", p = p.to_str().unwrap()))
        .collect::<Vec<_>>()
        .join(" -> ");

    assert_eq_sorted!(
        error.to_string(),
        format!(
            "Max load stack size exceeded while loading resource File(\"{base_path}\") -> {stack_files}"
        )
    );
}

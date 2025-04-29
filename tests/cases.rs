use dofigen_lib::*;
use pretty_assertions_sorted::assert_eq_sorted;
use std::path::PathBuf;

const PERMISSIVE_SUFFIX: &str = ".permissive";

#[test]
fn test_cases() {
    // Get all the files in the tests/cases directory
    let paths: Vec<PathBuf> = std::fs::read_dir("tests/cases")
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();

    let path_ref: Vec<&PathBuf> = paths.iter().collect();

    // Get the YAML results by filtering the files ending with .result.yml in a map with the basename as key
    let (yaml_results, path_ref) = filter_to_map(path_ref, ".result.yml");

    // Get the Dockerfile results by filtering the files ending with .result.Dockerfile in a map with the basename as key
    let (dockerfile_results, path_ref) = filter_to_map(path_ref, ".result.Dockerfile");

    // Get the lock files
    let (_, path_ref) = filter_to_map(path_ref, ".lock");

    // Iterate over remaining files and generate the effective YAML and Dockerfile to compare with the expected results
    for path in path_ref {
        let mut basename = path.to_str().unwrap().to_string();
        basename.truncate(basename.len() - path.extension().unwrap().to_str().unwrap().len() - 1);

        if basename.ends_with(PERMISSIVE_SUFFIX) {
            #[cfg(not(feature = "permissive"))]
            continue;

            #[cfg(feature = "permissive")]
            basename.truncate(basename.len() - PERMISSIVE_SUFFIX.len());
        }

        println!("Processing {}", basename);

        let mut context = DofigenContext::new();
        let dofigen: Dofigen = context
            .parse_from_resource(Resource::File(path.clone()))
            .unwrap();

        if let Some(content) = yaml_results.get(basename.as_str()) {
            println!("Compare with YAML result");
            let yaml = generate_effective_content(&dofigen).unwrap();
            assert_eq_sorted!(&yaml, content);
        }

        if let Some(content) = dockerfile_results.get(basename.as_str()) {
            println!("Compare with Dockerfile result");
            let dockerfile = GenerationContext::from(dofigen)
                .generate_dockerfile()
                .unwrap();
            assert_eq_sorted!(&dockerfile, content);
        }
    }

    fn filter_to_map<'a>(
        files: Vec<&'a PathBuf>,
        filter: &str,
    ) -> (std::collections::HashMap<String, String>, Vec<&'a PathBuf>) {
        let (results, files): (_, Vec<_>) = files
            .into_iter()
            .partition(|path| path.to_str().unwrap().ends_with(filter));

        let results = results
            .iter()
            .filter_map(|path| {
                let mut basename = path.to_str().unwrap().to_string();
                basename.truncate(basename.len() - filter.len());
                let content = std::fs::read_to_string(path).unwrap();
                Some((basename, content))
            })
            .collect::<std::collections::HashMap<String, String>>();

        (results, files)
    }
}

#[test]
fn test_load_url() {
    use httptest::{matchers::*, responders::*, Expectation, Server};
    use url::Url;

    let test_case_dir = PathBuf::from("tests/cases/");
    let server = Server::run();
    let files = vec!["springboot-maven.base.yml", "springboot-maven.override.yml"];
    for file in files {
        server.expect(
            Expectation::matching(request::method_path("GET", format!("/{}", file))).respond_with(
                status_code(200).body(std::fs::read_to_string(test_case_dir.join(file)).unwrap()),
            ),
        );
    }

    let url = server.url("/springboot-maven.override.yml");

    println!("URL: {}", url);

    let url = url.to_string();
    let url: Url = url.parse().unwrap();

    let dofigen: Dofigen = DofigenContext::new()
        .parse_from_resource(Resource::Url(url))
        .unwrap();

    let yaml = generate_effective_content(&dofigen).unwrap();
    assert_eq_sorted!(
        yaml,
        std::fs::read_to_string(test_case_dir.join("springboot-maven.override.result.yml"))
            .unwrap()
    );

    let dockerfile = GenerationContext::from(dofigen)
        .generate_dockerfile()
        .unwrap();
    assert_eq_sorted!(
        dockerfile,
        std::fs::read_to_string(test_case_dir.join("springboot-maven.override.result.Dockerfile"))
            .unwrap()
    );
}

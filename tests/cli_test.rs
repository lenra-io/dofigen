#[cfg(feature = "cli")]
mod cli {
    use core::str;
    use std::fs::read_to_string;

    use assert_cmd::output::OutputOkExt;
    use assert_fs::{
        assert::PathAssert,
        prelude::{FileWriteStr, PathChild, PathCopy},
    };
    use escargot::CargoRun;
    use lazy_static::lazy_static;
    use pretty_assertions_sorted::assert_eq_sorted;
    use regex::Regex;

    lazy_static! {
        static ref BIN: CargoRun = generate_bin();
    }

    fn generate_bin() -> CargoRun {
        let mut cargo_build = escargot::CargoBuild::new()
            .bin(env!("CARGO_PKG_NAME"))
            .current_release()
            .no_default_features()
            .features("cli");

        #[cfg(feature = "permissive")]
        {
            cargo_build = cargo_build.features("permissive");
        }
        #[cfg(feature = "strict")]
        {
            cargo_build = cargo_build.features("strict");
        }
        #[cfg(feature = "json_schema")]
        {
            cargo_build = cargo_build.features("json_schema");
        }

        cargo_build.run().unwrap()
    }

    fn output_starts_with(output: &Vec<u8>, expected: &str) {
        let mut output = str::from_utf8(output).unwrap().to_string();

        output.truncate(expected.len());

        assert_eq_sorted!(output, expected);
    }

    fn check_help_result(output: &Vec<u8>) {
        #[cfg(feature = "json_schema")]
        {
            let out = str::from_utf8(output).unwrap().to_string();
            assert!(out.contains("schema"));
        }

        output_starts_with(
            output,
            r#"A Dockerfile generator using a simplified description in YAML or JSON format create

Usage: dofigen <COMMAND>"#,
        );
    }

    #[test]
    fn without_subcommand() {
        let mut cmd = BIN.command();

        let output = cmd.unwrap_err();
        let output = output.as_output().unwrap();

        assert!(output.stdout.is_empty());
        check_help_result(&output.stderr)
    }

    #[test]
    fn help() {
        let mut cmd = BIN.command();
        cmd.arg("help");
        check_help_result(&cmd.unwrap().stdout);
    }

    #[test]
    fn help_option() {
        let mut cmd = BIN.command();
        cmd.arg("--help");
        check_help_result(&cmd.unwrap().stdout);
    }

    #[test]
    fn version() {
        let mut cmd = BIN.command();
        cmd.arg("--version");

        let output = cmd.unwrap().stdout;
        let output = str::from_utf8(&output).unwrap();

        assert_eq!(output, format!("dofigen {}\n", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn generate_specified_file_offline() {
        let temp = assert_fs::TempDir::new().unwrap();

        let mut cmd = BIN.command();
        cmd.current_dir(temp.path());
        cmd.arg("generate").arg("-f");

        #[cfg(not(feature = "permissive"))]
        {
            temp.copy_from("tests/cases", &["simple.yml"]).unwrap();
            cmd.arg("simple.yml");
        }

        #[cfg(feature = "permissive")]
        {
            temp.copy_from("tests/cases", &["simple.permissive.yml"])
                .unwrap();
            cmd.arg("simple.permissive.yml");
        }

        let output = cmd.unwrap();

        assert!(output.status.success());

        output_starts_with(&output.stdout, "        Add resource simple.");

        let dockerfile = temp.child("Dockerfile");
        let dockerignore = temp.child(".dockerignore");

        dockerfile.assert(predicates::path::is_file());
        dockerignore.assert(predicates::path::is_file());

        let dockerfile_content = read_to_string(dockerfile.path()).unwrap();

        // Remove the sha256 hash
        let re = Regex::new(r"@sha256:\S+").unwrap();
        let dockerfile_content = re.replace_all(dockerfile_content.as_str(), "");

        assert_eq_sorted!(
            dockerfile_content,
            read_to_string("tests/cases/simple.result.Dockerfile").unwrap()
        );

        dockerignore.assert(
            r#"# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

"#,
        );

        temp.close().unwrap();
    }

    #[test]
    fn generate_file_not_found() {
        let temp = assert_fs::TempDir::new().unwrap();

        let mut cmd = BIN.command();
        cmd.current_dir(temp.path());
        cmd.arg("generate");

        let output = cmd.unwrap_err();
        let output = output.as_output().unwrap();

        assert!(!output.status.success());

        assert!(output.stdout.is_empty());

        output_starts_with(&output.stderr, "error: No Dofigen file found");

        temp.close().unwrap();
    }

    #[test]
    fn extend_not_existing_url() {
        let temp = assert_fs::TempDir::new().unwrap();

        let mut cmd = BIN.command();
        cmd.current_dir(temp.path());
        cmd.arg("generate");

        let file = temp.child("dofigen.yml");
        file.write_str(
            r#"extend:
  - http://localhost:1/not-existing.yml
"#,
        )
        .unwrap();

        let output = cmd.unwrap_err();
        let output = output.as_output().unwrap();

        assert!(!output.status.success());

        let output = str::from_utf8(&output.stderr).unwrap().to_string();

        assert_eq_sorted!(output, "error: error sending request for url (http://localhost:1/not-existing.yml)\n\tCaused by: client error (Connect)\n\tCaused by: tcp connect error: Connection refused (os error 111)\n\tCaused by: Connection refused (os error 111)\n");

        temp.close().unwrap();
    }
}

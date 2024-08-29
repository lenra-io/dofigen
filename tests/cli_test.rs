#[cfg(feature = "cli")]
mod cli {
    use core::str;

    use assert_cmd::output::OutputOkExt;
    use escargot::CargoRun;
    use lazy_static::lazy_static;
    use pretty_assertions_sorted::assert_eq_sorted;

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

    fn check_help_result(output: &Vec<u8>) {
        let expected = r#"A Dockerfile generator using a simplified description in YAML or JSON format create

Usage: dofigen <COMMAND>"#;

        let mut out = str::from_utf8(output).unwrap().to_string();

        #[cfg(feature = "json_schema")]
        {
            assert!(out.contains("schema"));
        }

        out.truncate(expected.len());

        assert_eq_sorted!(out, expected);
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
    fn generate() {
        let temp = assert_fs::TempDir::new().unwrap();

        let mut cmd = BIN.command();
        cmd.current_dir(temp.path());
        cmd.arg("generate");

    }
}

use async_trait::async_trait;
use buildkit_frontend::oci::*;
use buildkit_frontend::run_frontend;
use buildkit_frontend::{Bridge, Frontend, FrontendOutput};
use buildkit_llb::ops::*;
use dofigen_lib::DofigenContext;
use failure::Error;
use fs::LayerPath;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

// mod debug;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(args);

    if let Err(error) = run_frontend(DofigenFrontend).await {
        eprintln!("{}", error);

        for cause in error.iter_causes() {
            eprintln!("  caused by: {}", cause);
        }

        std::process::exit(1);
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Options {
    /// Path to the `Dockerfile` - in our case it's a path to `dofigen.yml`
    pub filename: Option<PathBuf>,

    /// Dofigen lockfile
    pub lockfile: Option<PathBuf>,
}

pub struct DofigenFrontend;

#[async_trait]
impl Frontend<Options> for DofigenFrontend {
    async fn run(
        self,
        bridge: Bridge,
        options: Options,
    ) -> std::result::Result<FrontendOutput, Error> {
        eprintln!("\n\n===============================\n\n");
        eprintln!("Running DofigenFrontend");
        dbg!(&options.filename);
        dbg!(&options.lockfile);

        let dofigen_file = options
            .filename
            .map(|filename| filename.to_string_lossy().to_string())
            .unwrap_or("dofigen.yml".into());
        let dofigen_lockfile = options
            .lockfile
            .map(|filename| filename.to_string_lossy().to_string())
            .unwrap_or("dofigen.lock".into());
        let dockerfile_source = Source::local("dockerfile");
        dbg!(&dockerfile_source);
        let dockerfile_layer = bridge
            .solve(Terminal::with(
                dockerfile_source
                    .add_include_pattern(&dofigen_file)
                    .add_include_pattern(&dofigen_lockfile)
                    .output(),
            ))
            .await?;

        dbg!(&dockerfile_layer);

        let dockerfile_contents = String::from_utf8(
            bridge
                .read_file(&dockerfile_layer, dofigen_file, None)
                .await?,
        )?;
        dbg!(&dockerfile_contents);

        let lockfile_contents = String::from_utf8(
            bridge
                .read_file(&dockerfile_layer, dofigen_lockfile, None)
                .await?,
        )?;
        dbg!(&lockfile_contents);

        let mut context = DofigenContext::new();
        let dofigen = context.parse_from_string(&dockerfile_contents)?;
        dbg!(dofigen);

        let out_spec = ImageSpecification {
            created: None,
            author: None,

            architecture: Architecture::Amd64,
            os: OperatingSystem::Linux,

            config: Some(ImageConfig {
                entrypoint: Some(vec!["/bin/sh".into()]),
                cmd: Some(vec!["-c".into(), "/usr/bin/sha256sum *".into()]),
                env: None,
                user: None,
                working_dir: Some("/app".into()),

                labels: None,
                volumes: None,
                exposed_ports: None,
                stop_signal: None,
            }),

            rootfs: None,
            history: None,
        };

        // let llb = {
        //     // let alpine = Source::image("alpine:latest").ref_counted();
        //     // let destination = LayerPath::Other(alpine.output(), OUTPUT_FILENAME);

        //     // FileSystem::mkfile(OutputIdx(0), destination)
        //     //     .data(transformed_contents.into_bytes())
        //     //     .into_operation()
        //     //     .ref_counted()
        //     //     .output(0)
        // };
        let image = Source::image("alpine:latest");
        let llb = image.output();

        let out_ref = bridge.solve_with_cache(Terminal::with(llb), &[]).await?;

        let out = FrontendOutput::with_spec_and_ref(out_spec, out_ref);

        Ok(out)
    }
}

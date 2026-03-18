use async_trait::async_trait;
use buildkit_frontend::run_frontend;
use buildkit_frontend::{Bridge, Frontend, FrontendOutput};
use buildkit_llb::ops::*;
use dofigen_lib::DofigenContext;
use dofigen_lib::bin::get_lockfile_path;
use dofigen_lib::lock::LockFile;
use failure::Error;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

use crate::spec::ImageSpecificationExt;

mod spec;

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

    /// Target platform for the build (e.g., `linux/amd64`, `linux/arm64`, etc.)
    pub platform: Vec<String>,
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
        dbg!(&options.platform);

        let dofigen_file = options
            .filename
            .map(|filename| filename.to_string_lossy().to_string())
            .unwrap_or("dofigen.yml".into());
        let dofigen_lockfile =
            get_lockfile_path(dofigen_file.clone()).map(|path| path.to_string_lossy().to_string());
        let dockerfile_source = Source::local("dockerfile");
        dbg!(&dockerfile_source);
        let mut sources = dockerfile_source.add_include_pattern(&dofigen_file);
        if let Some(dofigen_lockfile) = dofigen_lockfile.as_ref() {
            sources = sources.add_include_pattern(dofigen_lockfile);
        }
        let dockerfile_layer = bridge.solve(Terminal::with(sources.output())).await?;

        dbg!(&dockerfile_layer);

        let dockerfile_contents = String::from_utf8(
            bridge
                .read_file(&dockerfile_layer, dofigen_file, None)
                .await?,
        )?;
        dbg!(&dockerfile_contents);

        let lockfile = if let Some(dofigen_lockfile) = dofigen_lockfile {
            let bytes = bridge
                .read_file(&dockerfile_layer, dofigen_lockfile, None)
                .await;
            if let Ok(bytes) = bytes {
                let lockfile_contents = String::from_utf8(bytes)?;
                dbg!(&lockfile_contents);
                let lockfile: LockFile = serde_yaml::from_str(lockfile_contents.as_str())?;
                Some(lockfile)
            } else {
                eprintln!("Failed to read lockfile: {}", bytes.err().unwrap());
                None
            }
        } else {
            None
        };

        let mut context = DofigenContext::new();

        let dofigen = if let Some(lockfile) = lockfile {
            context = lockfile.to_context();
            context.parse_from_string(dockerfile_contents.as_str())?
        } else {
            context.parse_from_string(&dockerfile_contents)?
        };
        dbg!(&dofigen);

        // let llb = {
        //     // let alpine = Source::image("alpine:latest").ref_counted();
        //     // let destination = LayerPath::Other(alpine.output(), OUTPUT_FILENAME);

        //     // FileSystem::mkfile(OutputIdx(0), destination)
        //     //     .data(transformed_contents.into_bytes())
        //     //     .into_operation()
        //     //     .ref_counted()
        //     //     .output(0)
        // };

        // TODO: Handle multiplaform builds: https://docs.docker.com/build/building/multi-platform/
        // And https://github.com/moby/buildkit/blob/eaa4de09fec1edc751dace2cb698342ce611a853/client/llb/state.go#L735

        let image = Source::image("alpine:latest");
        let llb = image.output();

        let out_ref = bridge.solve_with_cache(Terminal::with(llb), &[]).await?;

        let out = FrontendOutput::with_spec_and_ref(dofigen.image_specification(), out_ref);

        Ok(out)
    }
}

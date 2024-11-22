use async_trait::async_trait;
use buildkit_frontend::oci::*;
use buildkit_frontend::run_frontend;
use buildkit_frontend::{Bridge, Frontend, FrontendOutput};
use buildkit_llb::ops::*;
use failure::Error;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

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
        dbg!(&options);

        Ok(FrontendOutput::with_spec_and_ref(
            ImageSpecification {
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
            },
            bridge
                .solve_with_cache(Terminal::with(Source::image("alpine:latest").output()), &[])
                .await?,
        ))
    }
}

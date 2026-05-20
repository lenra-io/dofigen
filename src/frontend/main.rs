use async_trait::async_trait;
use buildkit_frontend::run_frontend;
use buildkit_frontend::{Bridge, Frontend, FrontendOutput, MultiPlatformEntry};
use buildkit_llb::ops::platform::{Platform, linux_amd64};
use buildkit_llb::ops::*;
use colored::{Color, Colorize};
use dofigen_lib::bin::get_lockfile_path;
use dofigen_lib::lock::LockFile;
use dofigen_lib::{DofigenContext, LintMessage, LintSession, MessageLevel};
use failure::Error;
use serde::{Deserialize, Deserializer, de};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use crate::llb::LlbBuilder;
use crate::spec::ImageSpecificationExt;

mod llb;
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
    #[serde(deserialize_with = "de_vec_from_str")]
    pub platform: Vec<Platform>,
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
        let dockerfile_source = Source::local("dockerfile").custom_name("Loading Dofigen file");
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

        let platforms = options.platform;

        let lint_messages = LintSession::analyze(&dofigen).messages();
        print_lint_messages(&lint_messages);

        if platforms.len() > 1 {
            let mut entries = Vec::with_capacity(platforms.len());
            for plat in &platforms {
                let image_spec = dofigen.image_specification(plat);
                let mut builder = LlbBuilder::new(dofigen.clone(), Some(plat.clone()));
                let image_ref = bridge
                    .solve_with_cache(Terminal::with(builder.build()), &[])
                    .await?;
                entries
                    .push(MultiPlatformEntry::new(plat.clone(), image_ref).with_spec(image_spec));
            }
            Ok(FrontendOutput::with_multi_platform(entries))
        } else {
            // TODO: default to the platform of the host if not specified, instead of defaulting to linux/amd64
            let platform = platforms.into_iter().next().unwrap_or(linux_amd64());
            let image_spec = dofigen.image_specification(&platform);
            dbg!(&image_spec);
            let mut builder = LlbBuilder::new(dofigen, Some(platform));
            let image_ref = bridge
                .solve_with_cache(Terminal::with(builder.build()), &[])
                .await?;
            Ok(FrontendOutput::with_spec_and_ref(image_spec, image_ref))
        }
    }
}

fn print_lint_messages(messages: &[LintMessage]) {
    messages.iter().for_each(|message| {
        eprintln!(
            "{}[path={}]: {}",
            match message.level {
                MessageLevel::Error => "error".color(Color::Red).bold(),
                MessageLevel::Warn => "warning".color(Color::Yellow).bold(),
            },
            message.path.join(".").color(Color::Blue).bold(),
            message.message
        );
    });
}

fn de_vec_from_str<'de, D>(deserializer: D) -> Result<Vec<Platform>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec = Vec::<String>::deserialize(deserializer)?;
    vec.into_iter()
        .map(|s| Platform::from_str(&s).map_err(de::Error::custom))
        .collect()
}

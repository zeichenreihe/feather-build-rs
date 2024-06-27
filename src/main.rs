use anyhow::Result;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use clap::{Parser, Subcommand};
use tokio::task::JoinSet;
use crate::download::Downloader;
use crate::download::versions_manifest::MinecraftVersion;
use crate::version_graph::VersionGraph;

mod version_graph;
mod download;
mod specialized_methods;

mod build;
mod sus;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
/// The version id used in the mappings diffs and mappings files.
/// This can end in `-client` and `-server`, or not have any suffix at all.
pub(crate) struct Version(String);

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Version {
    fn get_environment(&self) -> Environment {
        if self.0.ends_with("-client") {
            Environment::Client
        } else if self.0.ends_with("-server") {
            Environment::Server
        } else {
            Environment::Merged
        }
    }

    fn get_minecraft_version(&self) -> MinecraftVersion {
        if let Some(without) = self.0.strip_suffix("-client") {
            MinecraftVersion(without.to_owned())
        } else if let Some(without) = self.0.strip_suffix("-server") {
            MinecraftVersion(without.to_owned())
        } else {
            MinecraftVersion(self.0.to_owned())
        }
    }
}

#[derive(Debug, PartialEq)]
enum Environment {
    Merged,
    Client,
    Server,
}


/*
TODO: publish

TODO: version: uses `feather_version` for the version of the maven publication
 */

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    dbg!(&cli);

    match cli.command {
        Command::Build { all, versions } => {
            let downloader = Downloader::new(!cli.no_cache);

            let dir = Path::new("mappings/mappings");

            let start = Instant::now();

            let v = VersionGraph::resolve(dir)?;
            let v = Arc::new(v);

            let versions: Vec<&Version> = if all {
                v.versions().collect()
            } else {
                versions.into_iter()
                    .map(|version| v.get(&version))
                    .collect::<Result<_>>()?
            };

            println!("graph took {:?}", start.elapsed());

            let start = Instant::now();

            let versions_manifest = downloader.get_versions_manifest().await?;
            let versions_manifest = Arc::new(versions_manifest);

            let mut futures: JoinSet<_> = versions.into_iter()
                .map(|version| {
                    let downloader = downloader.clone();
                    let v = v.clone();
                    let versions_manifest = versions_manifest.clone();
                    let version = version.clone();
                    async move {
                        build::build(&downloader, &v, &versions_manifest, &version).await
                    }
                })
                .collect();

            let mut outputs = Vec::with_capacity(futures.len());
            while let Some(next) = futures.join_next().await {
                outputs.push(next??);
            }

            for result in outputs {
                dbg!(result.merged_feather);
                dbg!(result.unmerged_feather);
            }

            println!("building took {:?}", start.elapsed());

            Ok(())
        },
        Command::Sus { versions } => {
            let result = sus::report_sus().await?;

            dbg!(result);

            Ok(())
        },
        Command::Feather { version } => {
            Ok(())
        },
        Command::PropagateMappings {} => {
            Ok(())
        },
        Command::PropagateMappingsDown {} => {
            Ok(())
        },
        Command::PropagateMappingsUp {} => {
            Ok(())
        },
    }
}

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Disable the caching to disk for downloaded files
    #[arg(long = "no-cache")]
    no_cache: bool, // TODO: currently this is not implemented

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Make a build
    Build {
        /// Build all versions
        #[arg(long = "all")]
        all: bool,

        /// The versions to build
        #[arg(trailing_var_arg = true)]
        versions: Vec<String>,
    },
    /// Report all sus mappings
    Sus {
        /// The versions to check
        #[arg(trailing_var_arg = true, required = true)]
        versions: Vec<String>,
    },
    /// Open Enigma to edit the mappings of a version
    Feather {
        /// The version to edit the mappings of
        version: String,
    },
    // TODO: doc
    PropagateMappings,
    // TODO: doc
    PropagateMappingsDown,
    // TODO: doc
    PropagateMappingsUp,
}
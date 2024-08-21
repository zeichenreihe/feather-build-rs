use anyhow::{anyhow, bail, Context, Result};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use clap::{ArgAction, Parser, Subcommand};
use log::{info, trace};
use tokio::task::JoinSet;
use dukebox::Jar;
use dukebox::parsed::ParsedJar;
use dukenest::{NesterOptions, Nests};
use maven_dependency_resolver::coord::MavenCoord;
use maven_dependency_resolver::{DependencyScope, FoundDependency};
use maven_dependency_resolver::resolver::Resolver;
use quill::remapper::{ARemapper, BRemapper, NoSuperClassProvider};
use quill::tree::mappings::{Mappings};
use quill::tree::mappings_diff::MappingsDiff;
use crate::download::Downloader;
use crate::download::versions_manifest::MinecraftVersion;
use crate::dukelaunch::JavaRunConfig;
use crate::version_graph::VersionGraph;

mod version_graph;
mod download;
mod specialized_methods;

mod build;
// TODO: replace four spaces with tab, and click Replace all
mod sus;

mod dukelaunch;

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

pub(crate) fn setup_logger(verbose: u8) -> Result<()> {
    let level_filter = match verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3 => log::LevelFilter::Trace,
        x => bail!("the -v option may be specified up to three times, got {x} times"),
    };

    fern::Dispatch::new()
        //.level(log::LevelFilter::Off)
        //.level_for("feather_build_rs", level_filter)
        .level(level_filter)
        .level_for("serde_xml_rs", log::LevelFilter::Off)
        .level_for("reqwest", log::LevelFilter::Off)

        .level_for("feather_build_rs::download", log::LevelFilter::Off)

        .format({
            let start = Instant::now();
            move |out, message, record| {
                let elapsed = start.elapsed();

                let seconds = elapsed.as_secs();
                let micros = elapsed.subsec_micros();

                let level = record.level();
                let target = record.target();

                out.finish(format_args!("{seconds:4?}.{micros:06?} {level:5} {target} {message}"))
            }
        })
        .chain(std::io::stderr())
        .apply()
        .with_context(|| anyhow!("failed to set logger config with log level filter {level_filter:?}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    setup_logger(cli.verbose)?;
    trace!("parsed command line arguments as {cli:?}");

    // TODO: check where we actually need this...
    //dukelaunch::JavaLauncher::from_env_var()
    //    .check_java_version(17)
    //    .with_context(|| anyhow!("feathers buildscript requires java 17 or higher"))?;

    let mappings_dir =
        //mappings_dir.unwrap_or_else(|| "mappings".into())
        cli.mappings_dir.unwrap_or_else(|| "mappings/mappings".into());

    let downloader = Downloader::new(cli.no_cache, cli.offline);

    let project_enigma_version = "1.9.0";
    let project_quilt_enigma_plugin_version = "1.3.0";

    match cli.command {
        Command::Build { all, versions } => {
            let start = Instant::now();

            let v = VersionGraph::resolve(mappings_dir)?;
            let v = Arc::new(v);

            //TODO: consider the version_shortcuts in feather.py
            // (more generally: do everything feather.py does as well)
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
            let result = sus::report_sus(mappings_dir, downloader).await?;

            dbg!(result);

            Ok(())
        },
        Command::Feather { version } => {
            let java_launcher = dukelaunch::JavaLauncher::from_env_var();
            let java_launcher = dukelaunch::JavaLauncher::new("/usr/lib/jvm/java-17-openjdk/bin/java".to_owned());

            java_launcher.check_java_version(17)
                .with_context(|| anyhow!("feathers buildscript requires java 17 or higher"))?;

            async fn make_classpath(downloader: &Downloader, resolvers: &[Resolver<'_>], dependencies: &[(MavenCoord, DependencyScope)],
                    cache: Option<&[&str]>) -> Result<Vec<PathBuf>> {
                let dependencies: Vec<FoundDependency> = if let Some(cached) = cache {
                    cached.iter().map(|&x| FoundDependency::try_from(x)).collect::<Result<_>>()?
                } else {
                    let r = maven_dependency_resolver::get_maven_dependencies(downloader, resolvers, dependencies).await?;

                    // fixup the vineflower dependency
                    let mut r = r;
                    if let Some(x) = r.iter_mut().find(|x| x.coord.group == "org.vineflower" && x.coord.artifact == "vineflower") {
                        x.coord.classifier = Some("slim".to_owned())
                    }
                    let r = r;

                    eprintln!("add these lines to not try to request the whole dependency tree over network");
                    for i in &r {
                        eprintln!("\t\"{i}\",");
                    }

                    r
                };

                let mut paths = Vec::with_capacity(dependencies.len());
                for i in dependencies {
                    let url = i.make_url();
                    let jar = downloader.get_jar(&url).await?;
                    let path = jar.path;
                    let path = std::fs::canonicalize(&path).with_context(|| anyhow!("failed to canonicalize path {path:?}"))?;
                    paths.push(path);
                }
                Ok(paths)
            }

            let resolvers = [
                Resolver::new("Maven Central", "https://repo.maven.apache.org/maven2/"),
                Resolver::new("Ornithe", "https://maven.ornithemc.net/releases"),
                Resolver::new("Mojang", "https://libraries.minecraft.net/"),
                Resolver::new("Quilt Repository", "https://maven.quiltmc.org/repository/release/"),
                Resolver::new("Quilt Snapshot Repository", "https://maven.quiltmc.org/repository/snapshot/"),
                Resolver::new("Fabric Repository", "https://maven.fabricmc.net"),
                Resolver::new("Procyon Repository", "https://oss.sonatype.org"),
                Resolver::new("Vineflower Snapshots", "https://s01.oss.sonatype.org/content/repositories/snapshots/"),
            ];

            // this stores the resolved dependency tree, allowing us to not fetch all the poms
            let dependency_tree_cache = { [
                "net.ornithemc:enigma-swing:jar:1.9.0:runtime @ https://maven.ornithemc.net/releases",
                "org.quiltmc:quilt-enigma-plugin:jar:1.3.0:runtime @ https://maven.quiltmc.org/repository/release/",
                "com.google.guava:guava:jar:32.0.1-jre:runtime @ https://repo.maven.apache.org/maven2/",
                "com.google.code.gson:gson:jar:2.10.1:runtime @ https://repo.maven.apache.org/maven2/",
                "org.tinylog:tinylog-api:jar:2.6.2:runtime @ https://repo.maven.apache.org/maven2/",
                "org.tinylog:tinylog-impl:jar:2.6.2:runtime @ https://repo.maven.apache.org/maven2/",
                "net.ornithemc:enigma:jar:1.9.0:runtime @ https://maven.ornithemc.net/releases",
                "net.ornithemc:enigma-server:jar:1.9.0:runtime @ https://maven.ornithemc.net/releases",
                "net.sf.jopt-simple:jopt-simple:jar:6.0-alpha-3:runtime @ https://repo.maven.apache.org/maven2/",
                "com.formdev:flatlaf:jar:3.1.1:runtime @ https://repo.maven.apache.org/maven2/",
                "com.formdev:flatlaf-extras:jar:3.1.1:runtime @ https://repo.maven.apache.org/maven2/",
                "de.sciss:syntaxpane:jar:1.2.1:runtime @ https://repo.maven.apache.org/maven2/",
                "com.github.lukeu:swing-dpi:jar:0.10:runtime @ https://repo.maven.apache.org/maven2/",
                "org.drjekyll:fontchooser:jar:2.5.2:runtime @ https://repo.maven.apache.org/maven2/",
                "org.ow2.asm:asm:jar:9.3:runtime @ https://repo.maven.apache.org/maven2/",
                "org.ow2.asm:asm-commons:jar:9.3:runtime @ https://repo.maven.apache.org/maven2/",
                "org.ow2.asm:asm-tree:jar:9.3:runtime @ https://repo.maven.apache.org/maven2/",
                "org.ow2.asm:asm-util:jar:9.3:runtime @ https://repo.maven.apache.org/maven2/",
                "cuchaz:enigma:jar:1.1.7:runtime @ https://maven.quiltmc.org/repository/release/",
                "org.quiltmc:quilt-json5:jar:1.0.1:runtime @ https://maven.quiltmc.org/repository/release/",
                "org.jetbrains:annotations:jar:23.0.0:runtime @ https://repo.maven.apache.org/maven2/",
                "com.google.guava:failureaccess:jar:1.0.1:runtime @ https://repo.maven.apache.org/maven2/",
                "com.google.guava:listenablefuture:jar:9999.0-empty-to-avoid-conflict-with-guava:runtime @ https://repo.maven.apache.org/maven2/",
                "com.google.code.findbugs:jsr305:jar:3.0.2:runtime @ https://repo.maven.apache.org/maven2/",
                "org.checkerframework:checker-qual:jar:3.33.0:runtime @ https://repo.maven.apache.org/maven2/",
                "com.google.errorprone:error_prone_annotations:jar:2.18.0:runtime @ https://repo.maven.apache.org/maven2/",
                "com.google.j2objc:j2objc-annotations:jar:2.8:runtime @ https://repo.maven.apache.org/maven2/",
                "org.vineflower:vineflower:jar:slim:1.10.0-20230713.053900-2:runtime @ https://s01.oss.sonatype.org/content/repositories/snapshots/",
                "net.fabricmc:cfr:jar:0.2.1:runtime @ https://maven.fabricmc.net",
                "org.bitbucket.mstrobel:procyon-compilertools:jar:0.6.0:runtime @ https://repo.maven.apache.org/maven2/",
                "com.formdev:svgSalamander:jar:1.1.3:runtime @ https://repo.maven.apache.org/maven2/",
                "org.ow2.asm:asm-analysis:jar:9.3:runtime @ https://repo.maven.apache.org/maven2/",
                "org.quiltmc:procyon-quilt-compilertools:jar:0.5.35.local:runtime @ https://maven.quiltmc.org/repository/release/",
                "org.quiltmc:cfr:jar:0.0.6:runtime @ https://maven.quiltmc.org/repository/release/",
                "org.bitbucket.mstrobel:procyon-core:jar:0.6.0:runtime @ https://repo.maven.apache.org/maven2/",
                "org.quiltmc:procyon-quilt-core:jar:0.5.35.local:runtime @ https://maven.quiltmc.org/repository/release/"
            ].as_slice() };
            let dependency_tree_cached = Some(dependency_tree_cache);
            //let dependency_tree_cached = None;

            let enigma = MavenCoord::from_group_artifact_version("net.ornithemc", "enigma-swing", project_enigma_version);
            let enigma_plugin = MavenCoord::from_group_artifact_version("org.quiltmc", "quilt-enigma-plugin", project_quilt_enigma_plugin_version);

            let dependencies = [
                (enigma, DependencyScope::Runtime),
                (enigma_plugin, DependencyScope::Runtime),
            ];

            let classpath = make_classpath(&downloader, &resolvers, &dependencies, dependency_tree_cached).await?;

            let version_graph = VersionGraph::resolve(mappings_dir)?;
            let version = version_graph.get(&version)?;

            // this is the "mainJar" remapped to calamus mappings
            let calamus_jar = map_calamus_jar(&downloader, version).await?;

            let nested_jar = nest_jar(&downloader, version, &calamus_jar).await?;

            fn write(x: impl Jar) -> Result<PathBuf> {
                todo!("put jar into some place on disk")
            }

            let jar_path = if let Some(nested_jar) = nested_jar {
                write(nested_jar)?
            } else {
                write(calamus_jar)?
            };

            async fn separate_mappings(downloader: &Downloader, version_graph: &VersionGraph, version: &Version) -> Result<PathBuf> {
                let mappings = version_graph.apply_diffs(version)? // calamus -> named
                    .extend_inner_class_names("named")?;

                let mappings = if let Some(nests) = patch_nests(downloader, version).await? {
                    MappingUtils::apply_nests(mappings, nests)?
                } else {
                    mappings
                };

                let mappings = mappings.remove_dummy("named")?;

                // let working_mappings = todo!("figure this out");
                // quill::enigma_dir::write(&working_mappings_dir, mappings)?;
                // Ok(working_mappings)
                todo!("write mappings to a path")
            }
            let working_mappings = separate_mappings(&downloader, &version_graph, version).await?;

            let arg = JavaRunConfig {
                main_class: "cuchaz.enigma.gui.Main".into(),
                classpath: classpath.into_iter().map(|x| x.into_os_string()).collect(),
                jvm_args: vec![
                    "-Xmx2048m".into(),
                ],
                args: vec![
                    "-jar".into(), jar_path.into_os_string(),
                    "-mappings".into(), working_mappings.into_os_string(),
                    "-profile".into(), "enigma_profile.json".into(), // TODO: this should be specified by abs. path, with the mappings dir somehow...
                ],
            };

            java_launcher.launch(&arg)
        },
        Command::InsertMappings {} => {
            let version = Version(String::from("1.12.2"));

            info!("saving mappings for {version}");

            insert_mappings(mappings_dir, &downloader, &version, PropagationDirection::None).await
        }
        Command::PropagateMappings {} => {
            let version = Version(String::from("1.12.2"));

            info!("saving mappings for {version}");

            insert_mappings(mappings_dir, &downloader, &version, PropagationDirection::Both).await
        },
        Command::PropagateMappingsUp {} => {
            let version = Version(String::from("1.12.2"));

            info!("saving mappings for {version}");

            insert_mappings(mappings_dir, &downloader, &version, PropagationDirection::Up).await
        },
        Command::PropagateMappingsDown {} => {
            let version = Version(String::from("1.12.2"));

            info!("saving mappings for {version}");

            insert_mappings(mappings_dir, &downloader, &version, PropagationDirection::Down).await
        },
    }
}

enum PropagationDirection {
    None,
    Both,
    Up,
    Down,
}

struct PropagationOptions {
    direction: PropagationDirection,
    lenient: bool,
}

async fn insert_mappings(mappings_dir: PathBuf, downloader: &Downloader, version: &Version, direction: PropagationDirection) -> Result<()> {
    // TODO: this is input...
    let working_mappings_dir = Path::new("mappings/run/1.12.2"); // TODO: from `working_mappings` (see enigma launch code above!)

    let version_graph = VersionGraph::resolve(mappings_dir)?;

    let separated_mappings = version_graph.apply_diffs(version)? // calamus -> named
        .extend_inner_class_names("named")?;

    let start = Instant::now();

    let namespaces = separated_mappings.info.namespaces.clone();
    let working_mappings = quill::enigma_dir::read(working_mappings_dir, namespaces).context("enigma dir read!")?;

    dbg!("reading enigma mappings took: {}", start.elapsed());

    let calamus_nests_file = patch_nests(downloader, version).await?;

    let working_mappings = if let Some(nests) = calamus_nests_file {
        MappingUtils::undo_nests(working_mappings, nests)?
    } else {
        working_mappings
    };

    let changes = MappingsDiff::diff(&separated_mappings, &working_mappings)?;

    // TODO: (comment): this is the INSERT_DUMMY validator
    let changes = changes.insert_dummy_and_contract_inner_names()?;

    let options = PropagationOptions {
        direction,
        lenient: true,
    };
    MappingUtils::insert_mappings(options, version_graph, changes, version)?;

    FileUtils::delete(working_mappings_dir)?;

    Ok(())
}

// TODO: implement these
struct MappingUtils;
impl MappingUtils {
    fn apply_nests(mappings: Mappings<2>, nests: Nests) -> Result<Mappings<2>> {
        todo!()
    }
    fn undo_nests(working_mappings: Mappings<2>, nests: Nests) -> Result<Mappings<2>> {
        todo!()
    }
    fn insert_mappings(options: PropagationOptions, version_graph: VersionGraph, changes: MappingsDiff, version: &Version) -> Result<()> {
        todo!()
    }
}
struct FileUtils;
impl FileUtils {
    fn delete(dir: &Path) -> Result<()> {
        todo!()
    }
}

// output is `calamusJar`
// maps the mainJar (either server/client/mergedJar, selected in dlVersionDetails) from "official" to "calamus", to calamusJar
async fn map_calamus_jar(downloader: &Downloader, version: &Version) -> Result<ParsedJar> {
    let versions_manifest = downloader.get_versions_manifest().await?;
    let environment = version.get_environment();
    let version_details = downloader.version_details(&versions_manifest, version, &environment).await?;

    let client = downloader.get_jar(&version_details.downloads.client.url).await?;
    let server = downloader.get_jar(&version_details.downloads.server.url).await?;

    let start = Instant::now();

    let main_jar = dukebox::merge::merge(client, server).with_context(|| anyhow!("failed to merge jars for version {version}"))?;

    println!("jar merging took {:?}", start.elapsed());

    let calamus = downloader.calamus_v2(version).await?;

    let inheritance = NoSuperClassProvider::new();
    let out_jar = dukebox::remap::remap(main_jar, calamus.remapper_b_first_to_second(inheritance)?)?;

    println!("remapping done!");

    Ok(out_jar)
}

async fn nest_jar(downloader: &Downloader, version: &Version, calamus_jar: &impl Jar) -> Result<Option<ParsedJar>> {

    let calamus_nests_file = patch_nests(downloader, version).await?;

    if let Some(calamus_nests_file) = calamus_nests_file {
        // calamus_jar is the "mainJar" remapped to calamus mappings

        let nested_jar = dukenest::nest_jar(
            //NesterOptions::new().silent(true),
            NesterOptions::default().silent(false),
            calamus_jar,
            calamus_nests_file,
        )?;

        Ok(Some(nested_jar))
    } else {
        Ok(None)
    }
}

// note: `calamusNestsFile` is result of the `patchNests` task
async fn patch_nests(downloader: &Downloader, version: &Version) -> Result<Option<Nests>> {
    if let Some(nests) = downloader.download_nests(version).await? {
        let calamus = downloader.calamus_v2(version).await?;

        let dst = dukenest::map_nests(&calamus, nests)?;

        Ok(Some(dst))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Verbose mode. Errors and warnings are always logged. Multiple options increase verbosity.
    ///
    /// The maximum is 3. First comes info, then debug and then trace.
    #[arg(short = 'v', action = ArgAction::Count)]
    verbose: u8,

    /// Disable the caching to disk for downloaded files
    #[arg(long = "no-cache")]
    no_cache: bool,

    /// Run offline
    #[arg(long = "offline")]
    offline: bool,

    /// The mappings directory, default is 'mappings'
    ///
    /// This directory contains the '.tinydiff' and one '.tiny' file.
    #[arg(short = 'm', long = "mappings-dir")]
    mappings_dir: Option<PathBuf>,

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
    InsertMappings,
    // TODO: doc
    PropagateMappings,
    // TODO: doc
    PropagateMappingsUp,
    // TODO: doc
    PropagateMappingsDown,
}
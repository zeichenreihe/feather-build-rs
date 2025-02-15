use std::ffi::OsStr;
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use log::{info, trace};
use tokio::task::JoinSet;
use dukebox::storage::{ClassRepr, Jar, ParsedJar};
use dukenest::nest::Nests;
use maven_dependency_resolver::coord::MavenCoord;
use maven_dependency_resolver::{DependencyScope, FoundDependency};
use maven_dependency_resolver::resolver::Resolver;
use quill::tree::mappings::Mappings;
use quill::tree::mappings_diff::MappingsDiff;
use crate::download::Downloader;
use crate::dukelaunch::JavaRunConfig;
use crate::version_graph::{VersionEntry, VersionGraph};

mod version_graph;
mod download;
mod specialized_methods;

mod build;
// TODO: replace four spaces with tab, and click Replace all
mod sus;

mod dukelaunch;
mod insert_mappings;

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

struct Official;
struct Intermediary;
struct Named;

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    setup_logger(cli.verbose)?;
    trace!("parsed command line arguments as {cli:?}");

    // TODO: check where we actually need this...
    //dukelaunch::JavaLauncher::from_env_var()
    //    .check_java_version(17)
    //    .with_context(|| anyhow!("feathers buildscript requires java 17 or higher"))?;

    // TODO: better default (at least second and third)
    let default_mappings_dir = "mappings/mappings"; // TODO: switch back to "mappings"
    let default_working_mappings_base_dir = Path::new("/tmp/mappings_run"); // TODO: switch back to "mappings/run";
    let default_enigma_prepared_jar = Path::new("/tmp/enigma_run_jar_cache.jar");

    // TODO: orignally "enigma_profile.json"
    let default_enigma_profile_json = Path::new("mappings/enigma_profile.json"); // TODO: default should be in mappings dir?


    let mappings_dir = cli.mappings_dir
        .unwrap_or_else(|| default_mappings_dir.into());

    let working_mappings_dir = |working_mappings_base_dir: Option<PathBuf>, version: VersionEntry<'_>| -> PathBuf {
        let mut x = working_mappings_base_dir
            .unwrap_or_else(|| default_working_mappings_base_dir.to_owned());
        x.push(version.as_str());
        x
    };

    let downloader = Downloader::new(cli.no_cache, cli.offline);

    let project_enigma_version = "1.9.0";
    let project_quilt_enigma_plugin_version = "1.3.0";

    match cli.command {
        Command::Build { all, versions } => {
            let start = Instant::now();

            let v = VersionGraph::resolve(mappings_dir)?;
            let v = Arc::new(v);

            let versions: Vec<VersionEntry<'_>> = if all {
                v.versions().collect()
            } else {
                v.get_all(versions.iter()
                    .map(|potential_shortcut| version_graph::map_shortcut(&potential_shortcut))
                )?
                    .into_iter()
                    .map(|(split, v)| v)
                    .collect()
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
                    let version = version.make_owned();
                    async move {
                        build::build(&downloader, &v, &versions_manifest, version.make_borrowed()).await
                            .with_context(|| anyhow!("while building version {:?}", version.make_borrowed().as_str()))
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
        Command::Feather { working_mappings_base_dir, enigma_prepared_jar, enigma_profile, version } => {
            let java_launcher = dukelaunch::JavaLauncher::from_env_var()
                //.unwrap_or_default();
                .unwrap_or_else(|| dukelaunch::JavaLauncher::new("/usr/lib/jvm/java-17-openjdk/bin/java"));

            java_launcher.check_java_version(17)
                .with_context(|| anyhow!("feathers buildscript requires java 17 or higher"))?;

            async fn make_classpath(
                downloader: &Downloader,
                resolvers: &[Resolver<'_>],
                dependencies: &[(MavenCoord, DependencyScope)],
                cache: Option<&[&str]>
            ) -> Result<Vec<PathBuf>> {
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
            let (split, version) = version_graph.get(version_graph::map_shortcut(&version))?;

            let working_mappings_dir = working_mappings_dir(working_mappings_base_dir, version);

            // this is the "mainJar" remapped to calamus mappings
            let calamus_v2 = downloader.calamus_v2(version).await?;
            let calamus_jar = map_calamus_jar(&downloader, version, &calamus_v2).await?;

            let nested_jar = nest_jar(&downloader, version, &calamus_jar, &calamus_v2).await?;

            let jar_path = enigma_prepared_jar.as_deref()
                .unwrap_or(default_enigma_prepared_jar);

            nested_jar.as_ref()
                .unwrap_or(&calamus_jar)
                .put_to_file(jar_path)?;

            let profile_json_path = enigma_profile.as_deref()
                .unwrap_or(default_enigma_profile_json);

            let mappings = version_graph.apply_diffs(version)?; // calamus -> named

            let mappings = if let Some(nests) = patch_nests(&downloader, version, &calamus_v2).await? {
                dukenest::apply_nests_to_mappings(mappings, &nests)?
            } else {
                mappings
            };

            let mappings = mappings.remove_dummy("named")?;

            // TODO: warn the user about potentially destroying progress when running this command
            quill::enigma_dir::write(&mappings, &working_mappings_dir)?;

            let arg = JavaRunConfig {
                main_class: "cuchaz.enigma.gui.Main".into(),
                classpath: classpath.into_iter().map(|x| x.into_os_string()).collect(),
                jvm_args: vec![
                    "-Xmx2048m".into(),
                ],
                args: vec![
                    OsStr::new("-jar"), jar_path.as_os_str(),
                    OsStr::new("-mappings"), working_mappings_dir.as_os_str(),
                    OsStr::new("-profile"), profile_json_path.as_os_str(),
                ],
            };

            java_launcher.launch(&arg)
        },
        Command::PropagateMappings { working_mappings_base_dir, keep_directory, direction, version } => {
            let version_graph = VersionGraph::resolve(mappings_dir)?;

            let (split, version) = version_graph.get(version_graph::map_shortcut(&version))?;

            let working_mappings_dir = working_mappings_dir(working_mappings_base_dir, version);

            info!("saving mappings for {version:?}");

            let separated_mappings = version_graph.apply_diffs(version)?; // calamus -> named

            let start = Instant::now();

            let namespaces = separated_mappings.info.namespaces.clone();
            let working_mappings = quill::enigma_dir::read(&working_mappings_dir, namespaces)
                .with_context(|| anyhow!("failed to read enigma directory from {working_mappings_dir:?}"))?;

            dbg!("reading enigma mappings took: {}", start.elapsed());

            let calamus_v2 = downloader.calamus_v2(version).await?;
            let calamus_nests_file = patch_nests(&downloader, version, &calamus_v2).await?;

            let working_mappings = if let Some(nests) = calamus_nests_file {
                dukenest::undo_nests_to_mappings(working_mappings, &nests)?
            } else {
                working_mappings
            };

            let changes = MappingsDiff::diff(&separated_mappings, &working_mappings)?;

            // TODO: (comment): this is the INSERT_DUMMY validator
            let changes = changes.insert_dummy_and_contract_inner_names()?;

            insert_mappings::insert_mappings(direction, true, &version_graph, changes, version)?;

            if !keep_directory {
                std::fs::remove_dir_all(&working_mappings_dir)
                    .with_context(|| anyhow!("failed to delete working mappings directory {working_mappings_dir:?}"))?;
            }

            Ok(())
        },
        Command::DumpVersionGraph { output } => {
            let version_graph = VersionGraph::resolve(mappings_dir)?;

            let mut f = std::fs::File::create(&output)
                .with_context(|| anyhow!("failed to create file {output:?}"))?;

            version_graph.write_as_dot(&mut f)
                .with_context(|| anyhow!("failed to dump version graph to file {output:?}"))?;

            Ok(())
        },
    }
}

// output is `calamusJar`
// maps the mainJar (either server/client/mergedJar, selected in dlVersionDetails) from "official" to "calamus", to calamusJar
async fn map_calamus_jar(downloader: &Downloader, version: VersionEntry<'_>, calamus_v2: &Mappings<2, (Official, Intermediary)>)
        -> Result<ParsedJar<ClassRepr, Vec<u8>>> {
    let versions_manifest = downloader.get_versions_manifest().await?;
    let version_details = downloader.version_details(&versions_manifest, version).await?;

    // TODO: unwrap
    let client = downloader.get_jar(&version_details.downloads.client.as_ref().unwrap().url).await?;
    // TODO: unwrap
    let server = downloader.get_jar(&version_details.downloads.server.as_ref().unwrap().url).await?;

    // TODO: but don't merge for split versions
    let start = Instant::now();

    let main_jar = dukebox::merge::merge(client, server)
        .with_context(|| anyhow!("failed to merge jars for version {version:?}"))?;

    println!("jar merging took {:?}", start.elapsed());

    // TODO: should probably also add in the libraries here...
    let inheritance = main_jar.get_super_classes_provider()?;
    let out_jar = dukebox::remap::remap(main_jar, calamus_v2.remapper_b_first_to_second(&inheritance)?)?;

    println!("remapping done!");

    Ok(out_jar)
}

async fn nest_jar(downloader: &Downloader, version: VersionEntry<'_>, calamus_jar: &impl Jar, calamus_v2: &Mappings<2, (Official, Intermediary)>)
    -> Result<Option<ParsedJar<ClassRepr, Vec<u8>>>> {

    let calamus_nests_file = patch_nests(downloader, version, calamus_v2).await?;

    if let Some(calamus_nests_file) = calamus_nests_file {
        // calamus_jar is the "mainJar" remapped to calamus mappings

        let nested_jar = dukenest::nest_jar(
            true,
            calamus_jar,
            calamus_nests_file,
        )?;

        Ok(Some(nested_jar))
    } else {
        Ok(None)
    }
}

// note: `calamusNestsFile` is result of the `patchNests` task
async fn patch_nests(downloader: &Downloader, version: VersionEntry<'_>, calamus_v2: &Mappings<2, (Official, Intermediary)>)
        -> Result<Option<Nests<Intermediary>>> {
    if let Some(nests) = downloader.download_nests(version).await? {
        let dst = dukenest::remap_nests(&nests, &calamus_v2)?;

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
        /// The working mappings base directory, default is a temporary one
        ///
        /// This directory will contain a directory with the version name.
        ///
        /// Inside that are then the packages with enigmas '.mapping' files.
        #[arg(short = 'w', long = "working-mappings-dir")]
        working_mappings_base_dir: Option<PathBuf>,

        // TODO: document default
        /// The path to store the prepared jar for enigma to
        #[arg(long = "enigma-prepared-jar")]
        enigma_prepared_jar: Option<PathBuf>,

        // TODO: document default
        /// Path to the 'profile.json' for enigma
        #[arg(long = "enigma-profile")]
        enigma_profile: Option<PathBuf>,

        /// The version to edit the mappings of
        version: String,
    },

    // insert-mappings -> propagate-mappings none
    // propagate-mappings -> propagate-mappings both
    // propagate-mappings-up -> propagate-mappings up
    // propagate-mappings-down -> propagate-mappings down
    // TODO: some kind of --only-propagate-method-names-as-far-as-parameters (as parameter names can only be propagated to arguments of the same type)
    PropagateMappings {
        /// The working mappings base directory to take the mappings from.
        ///
        /// This directory will contain a directory with the version name.
        ///
        /// Inside that are then the packages with enigmas '.mapping' files.
        #[arg(short = 'w', long = "working-mappings-dir")]
        working_mappings_base_dir: Option<PathBuf>,

        /// Keep the working mappings directory
        ///
        /// Without this flag, deletes the working mappings directory.
        #[arg(long = "keep")]
        keep_directory: bool,

        /// The direction to propagate the changes in
        #[arg(short = 'd', long = "direction", value_enum, default_value_t)]
        direction: PropagationDirection,

        version: String,
    },

    /// Write the version graph in '.dot' format. This is intended for debugging.
    DumpVersionGraph {
        output: PathBuf,
    },
}

// TODO: doc
#[derive(Debug, Default, Copy, Clone, ValueEnum)]
enum PropagationDirection {
    None,
    #[default]
    Both,
    Up,
    Down,
}
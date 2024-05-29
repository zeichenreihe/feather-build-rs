use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use anyhow::{anyhow, bail, Context, Result};
use class_file::tree::method::ParameterName;
use crate::download::Downloader;
use crate::download::versions_manifest::MinecraftVersion;
use crate::jar::Jar;
use mappings_rw::tree::mappings::Mappings;
use mappings_rw::tree::names::Names;
use crate::version_graph::VersionGraph;

mod version_graph;
mod download;
mod specialized_methods;

mod jar;

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

fn inspect<const N: usize>(mappings: &Mappings<N>, path: &str) -> Result<()> {

    // fix the order of the other file being looked at, which makes diffing easier...
    /*
	let mut file = File::open("/tmp/original.tiny")?;
	let mut file_out = File::create("/tmp/original_sorted_rs.tiny")?;
	let m: Mappings<3> = reader::tiny_v2::read(&mut file)?;
	writer::tiny_v2::write(&m, &mut file_out)?;
	 */

    let mut file = File::create(path)?;
    mappings_rw::tiny_v2::write(mappings, &mut file)?;
    Ok(())
}

async fn build(downloader: &mut Downloader, version_graph: &VersionGraph, version: &Version) -> Result<BuildResult> {

    let feather_version = next_feather_version(downloader, version, false).await?;

    let mappings = version_graph.apply_diffs(version)? // calamus -> named
        .remove_dummy("named")?;

    let main_jar = main_jar(downloader, version).await?;
    let calamus_v2 = downloader.calamus_v2(version).await?;
    let libraries = downloader.mc_libs(version).await?;

    let build_feather_tiny = Jar::add_specialized_methods_to_mappings(&main_jar, &calamus_v2, &libraries, &mappings)
        .context("failed to add specialized methods to mappings")?;

    let merge_v2 = merge_v2(&build_feather_tiny, &calamus_v2)?;

    inspect(&merge_v2, "/tmp/out.tiny")?;

    let name = format!("feather-{feather_version}-mergedv2.jar");
    let data = mappings_rw::tiny_v2::write_zip_file(&merge_v2)?;
    let merged_feather = Jar::new_mem(name, data);

    let name = format!("feather-{feather_version}-v2.jar");
    let data = mappings_rw::tiny_v2::write_zip_file(&build_feather_tiny)?;
    let unmerged_feather = Jar::new_mem(name, data);

    Ok(BuildResult { merged_feather, unmerged_feather })
}

/// Merges the calamus intermediary with the created mappings.
///
/// The namespaces are `official` to `intermediary` and `named` here.
fn merge_v2(feather: &Mappings<2>, calamus: &Mappings<2>) -> Result<Mappings<3>> {
    let mappings_a = feather.clone().rename_namespaces(["calamus", "named"], ["intermediary", "named"])?;
    let mappings_b = calamus.clone().reorder(["intermediary", "official"])?;

    let merged = Mappings::merge(&mappings_b, &mappings_a)?
        .apply_our_fix()?;

    merged.reorder(["official", "intermediary", "named"])
}

trait ApplyFix: Sized { fn apply_our_fix(self) -> Result<Self>; }

impl ApplyFix for Mappings<3> {
    fn apply_our_fix(mut self) -> Result<Self> {
        let official = self.get_namespace("official")?;
        let intermediary = self.get_namespace("intermediary")?;
        let named = self.get_namespace("named")?;

        fn check_names<T: Debug>(names: &Names<3, T>) -> Result<()> {
            let names: &[Option<T>; 3] = names.into();

            for n in names {
                if n.is_none() {
                    bail!("at least one `None` in {names:?}");
                }
            }

            // TODO: if there's a None in names, fail
            Ok(())
        }

        for c in self.classes.values_mut() {
            if c.info.names[named].is_none() {
                c.info.names[named] = c.info.names[intermediary].clone();
            }

            /*

            // TODO: consider deref / .as_str() method..
            let names: [Option<ClassName>; N] = names.into();
            let names: [Option<String>; N] = names.map(|x| x.map(|x| x.into()));

            if names.len() >= 3 && names[1].as_ref().is_some_and(|x| x.contains('$')) {
                let a = names[1].as_ref().unwrap().chars().filter(|x| *x == '$').count();
                let b = names[2].as_ref().map(|y| y.chars().filter(|x| *x == '$').count()).unwrap_or(0);

                if a != b {
                    let wrong_inner_class_name = names[2].as_deref().unwrap_or("");
                    dbg!(wrong_inner_class_name);
                }
            }

            let names: [Option<ClassName>; N] = names.map(|x| x.map(|x| x.into()));
            let names = names.into();
            */


            check_names(&c.info.names)?;

            for f in c.fields.values_mut() {
                if c.info.names[named].is_none() {
                    c.info.names[named] = c.info.names[intermediary].clone();
                }

                if f.info.names[named].is_none() {
                    f.info.names[named] = f.info.names[intermediary].clone();
                }

                check_names(&f.info.names)?;
            }

            for m in c.methods.values_mut() {
                if m.info.names[named].is_none() {
                    m.info.names[named] = m.info.names[intermediary].clone();
                }

                if m.info.names[official].is_none() && m.info.names[intermediary].as_ref().is_some_and(|x| x == "<init>") {
                    m.info.names[official] = m.info.names[intermediary].clone();
                }

                if m.info.names[official].is_none() && m.info.names[intermediary].as_ref().is_some_and(|x| !x.as_str().starts_with("m_")) {
                    m.info.names[official] = m.info.names[intermediary].clone();
                }

                if m.info.names[official].is_none() {
                    m.info.names[official] = m.info.names[intermediary].clone();
                    //TODO: this fixes the issues where you have
                    // c  net/minecraft/server/MinecraftServer  net/minecraft/server/MinecraftServer  net/minecaft/server/MinecraftServer
                    //   m  ()Z  m_5733001  m_5733001  isSnooperEnabled
                    // c  um  net/minecraft/unmapped/C_3978417  net/minecraft/snooper/SnooperPopulator
                    //   m  ()Z  Z  m_5733001  isSnooperEnabled
                    // which ofc is incorrect
                    // further: make this something for the sus module

                    //println!("no names: {:?}", m.info);
                    //continue;
                }

                check_names(&m.info.names)?;

                for p in m.parameters.values_mut() {
                    if p.info.names[official].is_none() {
                        p.info.names[official] = Some(ParameterName::from(""));
                    }

                    check_names(&p.info.names)?;
                }
            }
        }

        Ok(self)
    }
}

/// Gets the jar from mojang. If it's a merged environment ([Environment::Merged]), then
/// the two jars (client and server) will be merged.
///
/// This jar is in the `official` mappings, i.e. obfuscated.
async fn main_jar(downloader: &mut Downloader, version: &Version) -> Result<Jar> {
    let environment = version.get_environment();

    let version_details = downloader.version_details(version, &environment).await?;

    match environment {
        Environment::Merged => {
            let client = downloader.get_jar(&version_details.downloads.client.url).await?;
            let server = downloader.get_jar(&version_details.downloads.server.url).await?;

            Jar::merge(client, server).with_context(|| anyhow!("failed to merge jars for version {version}"))
        },
        Environment::Client => downloader.get_jar(&version_details.downloads.client.url).await,
        Environment::Server => downloader.get_jar(&version_details.downloads.server.url).await,
    }
}

#[derive(Debug)]
struct BuildResult {
    merged_feather: Jar,
    unmerged_feather: Jar,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut downloader = Downloader::new();

    let dir = Path::new("mappings/mappings");

    let start = Instant::now();

    let v = VersionGraph::resolve(dir)?;

    println!("graph took {:?}", start.elapsed());

    let version = v.get("1.12.2").unwrap();

    let start = Instant::now();

    let result = build(&mut downloader, &v, version).await?;

    println!("building took {:?}", start.elapsed());

	dbg!(result.merged_feather);
	dbg!(result.unmerged_feather);

    Ok(())
}

async fn next_feather_version(downloader: &mut Downloader, version: &Version, local: bool) -> Result<String> {
    if local {
        Ok(format!("{version}+build.local"))
    } else {
        let url = "https://maven.ornithemc.net/releases/net/ornithemc/feather/maven-metadata.xml";

        let mut build_number = 0;

        // Note: we consider it a hard failure if the maven-metadata.xml file does no exist.
        // However if you don't have this file yet, you can comment out the lines below to start at build number 1.
        let metadata = downloader.get_maven_metadata_xml(url).await?;

        let version_build = format!("{version}");

        for version in metadata.versioning.versions.versions {
            if let Some((left, right)) = version.split_once("+build.") {
                if left == version_build {
                    let number = right.parse()?;
                    build_number = build_number.max(number);
                }
            }
        }

        let next_build_number = build_number + 1;

        Ok(format!("{version}+build.{next_build_number}"))
    }
}

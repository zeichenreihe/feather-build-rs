use std::fmt::Debug;
use std::fs::File;
use std::io::Cursor;
use anyhow::{anyhow, bail, Context, Result};
use log::info;
use zip::write::FileOptions;
use zip::ZipWriter;
use duke::tree::class::ObjClassName;
use duke::tree::method::MethodName;
use dukebox::storage::{FileJar, Jar, NamedMemJar};
use dukenest::nest::Nests;
use crate::download::Downloader;
use crate::download::versions_manifest::VersionsManifest;
use quill::tree::mappings::Mappings;
use quill::tree::names::{Names, Namespace};
use crate::{Intermediary, Named, Official};
use crate::version_graph::{Environment, VersionEntry, VersionGraph};

trait Inspect {
	fn inspect(self, path: &str) -> Result<Self> where Self: Sized;
}
impl<const N: usize, Ns> Inspect for Mappings<N, Ns> {
	fn inspect(self, path: &str) -> Result<Self> {
		info!("starting inspecting to {path:?}");

		// fix the order of the other file being looked at, which makes diffing easier...
		/*
		let mut file = File::open("/tmp/original.tiny")?;
		let mut file_out = File::create("/tmp/original_sorted_rs.tiny")?;
		let m: Mappings<3> = reader::tiny_v2::read(&mut file)?;
		writer::tiny_v2::write(&m, &mut file_out)?;
		 */

		let mut file = File::create(path)?;
		quill::tiny_v2::write(&self, &mut file)?;

		info!("finished inspecting to {path:?}");
		Ok(self)
	}
}

pub(crate) async fn build(
	downloader: &Downloader,
	version_graph: &VersionGraph,
	versions_manifest: &VersionsManifest,
	version: VersionEntry<'_>,
) -> Result<BuildResult> {
	let version_details = downloader.version_details(versions_manifest, version).await?;

	let feather_version = next_feather_version(downloader, version, false).await?;

	let calamus_v2 = downloader.calamus_v2(version).await?;
	let libraries = downloader.mc_libs(versions_manifest, version).await?;
	let nests = downloader.download_nests(version).await?;

	// Get the jar from mojang. If it's a merged environment, then merge the two jars (client and server).
	match version.get_environment() {
		Environment::Merged => {
			// TODO: unwrap
			let client = downloader.get_jar(&version_details.downloads.client.as_ref().unwrap().url).await?;
			// TODO: unwrap
			let server = downloader.get_jar(&version_details.downloads.server.as_ref().unwrap().url).await?;

			info!("{version:?} starting merging");
			let main_jar = dukebox::merge::merge(client, server)
				.with_context(|| anyhow!("failed to merge jars for version {version:?}"))?;
			info!("{version:?} finished merging");

			build_inner(feather_version, calamus_v2, libraries, version_graph, version, nests, &main_jar)
		},
		Environment::Client => {
			// TODO: unwrap
			let main_jar = downloader.get_jar(&version_details.downloads.client.as_ref().unwrap().url).await?;

			build_inner(feather_version, calamus_v2, libraries, version_graph, version, nests, &main_jar)
		},
		Environment::Server => {
			// TODO: unwrap
			let main_jar = downloader.get_jar(&version_details.downloads.server.as_ref().unwrap().url).await?;

			build_inner(feather_version, calamus_v2, libraries, version_graph, version, nests, &main_jar)
		},
	}
}

async fn next_feather_version(downloader: &Downloader, version: VersionEntry<'_>, local: bool) -> Result<String> {
	if local {
		Ok(format!("{version}+build.local", version = version.as_str()))
	} else {
		let url = "https://maven.ornithemc.net/releases/net/ornithemc/feather/maven-metadata.xml";

		let mut build_number = 0;

		// Note: we consider it a hard failure if the maven-metadata.xml file does no exist.
		// However if you don't have this file yet, you can comment out the lines below to start at build number 1.
		let metadata = downloader.get_maven_metadata_xml(url).await?;

		for version in metadata.versioning.versions.versions {
			if let Some((left, right)) = version.split_once("+build.") {
				if left == version.as_str() {
					let number = right.parse()?;
					build_number = build_number.max(number);
				}
			}
		}

		let next_build_number = build_number + 1;

		Ok(format!("{version}+build.{next_build_number}", version = version.as_str()))
	}
}

fn build_inner(
	feather_version: String,
	calamus_v2: Mappings<2, (Official, Intermediary)>,
	libraries: Vec<FileJar>,
	version_graph: &VersionGraph,
	version: VersionEntry<'_>,
	nests: Option<Nests<Official>>,
	main_jar: &impl Jar
) -> Result<BuildResult> {
	info!("{version:?} starting getting mappings from version graph");
	let mappings = version_graph.apply_diffs(version)? // calamus -> named
		.remove_dummy("named")?;
	info!("{version:?} finished getting mappings from version graph");
	let mappings = if let Some(nests) = nests {
		let nests = dukenest::remap_nests(&nests, &calamus_v2)?; // calamus
		dukenest::undo_nests_to_mappings(mappings, &nests)?
	} else { mappings }
		.inspect("/tmp/inspect_mappings.tiny")?;

	let build_feather_tiny = crate::specialized_methods::add_specialized_methods_to_mappings(main_jar, &calamus_v2, &libraries, &mappings)
		.context("failed to add specialized methods to mappings")?;

	// merge w.r.t. "intermediary", and then reorder from "intermediary -> official, named" to "official -> intermediary, named"
	let merge_v2 = Mappings::merge(
		&calamus_v2
			.inspect("/tmp/inspect_calamus.tiny")?
			.reorder(["intermediary", "official"])?
			.inspect("/tmp/inspect_calamus_.tiny")?,
		&build_feather_tiny.clone()
			.rename_namespaces(["intermediary", "named"], ["intermediary", "named"])? // TODO: same names to same names
			.inspect("/tmp/inspect_build_feather_.tiny")?
	)?
		.inspect("/tmp/inspect.tiny")?
		.apply_our_fix()?
		.reorder::<(Official, Intermediary, Named)>(["official", "intermediary", "named"])?
		.inspect("/tmp/out.tiny")?;

	let name = format!("feather-{feather_version}-mergedv2.jar");
	let data = tiny_v2_write_zip_file(&merge_v2)?;
	let merged_feather = NamedMemJar { name, data };

	let name = format!("feather-{feather_version}-v2.jar");
	let data = tiny_v2_write_zip_file(&build_feather_tiny)?;
	let unmerged_feather = NamedMemJar { name, data };

	Ok(BuildResult { merged_feather, unmerged_feather })
}

fn tiny_v2_write_zip_file<const N: usize, Ns>(mappings: &Mappings<N, Ns>) -> Result<Vec<u8>> {
	let mut zip = ZipWriter::new(Cursor::new(Vec::new()));

	zip.start_file("mappings/mappings.tiny", FileOptions::<()>::default())?;

	quill::tiny_v2::write(mappings, &mut zip)?;

	Ok(zip.finish()?.into_inner())
}

trait ApplyFix: Sized { fn apply_our_fix(self) -> Result<Self>; }

impl ApplyFix for Mappings<3, (Intermediary, Official, Named)> {
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

			Ok(())
		}


		fn check_eq_num_of_dollar<const N: usize>(names: &Names<N, ObjClassName>, a: Namespace<N>, b: Namespace<N>) -> Result<()> {
			fn count_dollars(x: &Option<ObjClassName>) -> usize {
				x.as_ref().map_or(0, |x| x.as_inner().chars().filter(|x| *x == '$').count())
			}

			let n = count_dollars(&names[a]);
			let m = count_dollars(&names[b]);
			if n != m {
				bail!("number of `$` doesn't match in the namespaces {a:?} and {b:?} - {n:?} vs. {m:?}: {names:?}");
			}
			Ok(())
		}

		fn replace_empty<const N: usize, T: Clone>(names: &mut Names<N, T>, to_replace: Namespace<N>, take_from: Namespace<N>) {
			if names[to_replace].is_none() {
				names[to_replace] = names[take_from].clone();
			}
		}

		fn copy_init_and_starting_with_not_m<const N: usize>(names: &mut Names<N, MethodName>, from: Namespace<N>, to: Namespace<N>) {
			if names[to].is_none() && names[from].as_ref().is_some_and(|x| x == MethodName::INIT || !x.as_inner().starts_with("m_")) {
				names[to] = names[from].clone();
			}
		}

		for c in self.classes.values_mut() {
			replace_empty(&mut c.info.names, named, intermediary);

			check_eq_num_of_dollar(&c.info.names, named, intermediary)?;

			check_names(&c.info.names)?;

			for f in c.fields.values_mut() {
				replace_empty(&mut f.info.names, named, intermediary);

				check_names(&f.info.names)?;
			}

			for m in c.methods.values_mut() {
				replace_empty(&mut m.info.names, named, intermediary);

				copy_init_and_starting_with_not_m(&mut m.info.names, intermediary, official);

				replace_empty(&mut m.info.names, official, intermediary);
				//TODO: this fixes the issues where you have
				// c  net/minecraft/server/MinecraftServer  net/minecraft/server/MinecraftServer  net/minecaft/server/MinecraftServer
				//   m  ()Z  m_5733001  m_5733001  isSnooperEnabled
				// c  um  net/minecraft/unmapped/C_3978417  net/minecraft/snooper/SnooperPopulator
				//   m  ()Z  Z  m_5733001  isSnooperEnabled
				// which ofc is incorrect
				// further: make this something for the sus module

				check_names(&m.info.names)?;
			}
		}

		Ok(self)
	}
}

#[derive(Debug)]
pub(crate) struct BuildResult {
	pub(crate) merged_feather: NamedMemJar,
	pub(crate) unmerged_feather: NamedMemJar,
}
use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::map::Entry;
use duke::tree::method::MethodName;
use dukebox::storage::{FileJar, Jar};
use quill::remapper::{BRemapper, JarSuperProv};
use quill::tree::mappings::{Mappings, MethodMapping, MethodNowodeMapping};
use quill::tree::names::Names;
use quill::tree::{NodeInfo, ToKey};
use crate::download::Downloader;
use crate::download::versions_manifest::VersionsManifest;
use crate::{Intermediary, Named, Official};
use crate::specialized_methods::GetSpecializedMethods;
use crate::version_graph::{Environment, VersionEntry, VersionGraph};

pub(crate) async fn report_sus(mappings_dir: PathBuf, downloader: Downloader) -> Result<SusResult> {
	let start = Instant::now();

	let v = VersionGraph::resolve(mappings_dir)?;

	println!("graph took {:?}", start.elapsed());

	let (split, version) = v.get("1.12.2").unwrap();

	let start = Instant::now();

	let versions_manifest= downloader.get_versions_manifest().await?;
	let result = sus(&downloader, &v, &versions_manifest, version).await?;

	println!("sus took {:?}", start.elapsed());

	Ok(result)
}

#[derive(Debug)]
pub(crate) struct SusResult;

async fn sus(
	downloader: &Downloader,
	version_graph: &VersionGraph,
	versions_manifest: &VersionsManifest,
	version: VersionEntry<'_>
) -> Result<SusResult> {
	let calamus_v2 = downloader.calamus_v2(version).await?;
	let libraries = downloader.mc_libs(versions_manifest, version).await?;

	let version_details = downloader.version_details(versions_manifest, version).await?;

	match version.get_environment() {
		Environment::Merged => {
			// TODO: unwrap
			let client = downloader.get_jar(&version_details.downloads.client.as_ref().unwrap().url).await?;
			// TODO: unwrap
			let server = downloader.get_jar(&version_details.downloads.server.as_ref().unwrap().url).await?;

			let main_jar = dukebox::merge::merge(client, server)
				.with_context(|| anyhow!("failed to merge jars for version {version:?}"))?;

			sus_inner(calamus_v2, libraries, version_graph, version, &main_jar)
		},
		Environment::Client => {
			// TODO: unwrap
			let main_jar = downloader.get_jar(&version_details.downloads.client.as_ref().unwrap().url).await?;

			sus_inner(calamus_v2, libraries, version_graph, version, &main_jar)
		},
		Environment::Server => {
			// TODO: unwrap
			let main_jar = downloader.get_jar(&version_details.downloads.server.as_ref().unwrap().url).await?;

			sus_inner(calamus_v2, libraries, version_graph, version, &main_jar)
		},
	}
}

fn sus_inner(
	calamus_v2: Mappings<2, (Official, Intermediary)>,
	libraries: Vec<FileJar>,
	version_graph: &VersionGraph,
	version: VersionEntry<'_>,
	main_jar: &impl Jar
) -> Result<SusResult> {
	// TODO: properly update based on build.rs

	println!("sus!");

	let mappings = version_graph.apply_diffs(version)?
		.remove_dummy("named")?;

	let build_feather_tiny = add_specialized_methods_to_mappings(main_jar, &calamus_v2, &libraries, mappings)
		.context("failed to add specialized methods to mappings")?;

	let mappings_a = build_feather_tiny.rename_namespaces(["calamus", "named"], ["intermediary", "named"])?;
	let mappings_b = calamus_v2.reorder(["intermediary", "official"])?;

	let merged = Mappings::merge(&mappings_b, &mappings_a)?.apply_our_fix()?;

	let merge_v2 = merged.reorder::<(Official, Intermediary, Named)>(["official", "intermediary", "named"])?;

	Ok(SusResult)
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

		for c in self.classes.values_mut() {
			if c.info.names[named].is_none() {
				c.info.names[named] = c.info.names[intermediary].clone();
			}

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

				if m.info.names[official].is_none() && m.info.names[intermediary].as_ref().is_some_and(|x| x == MethodName::INIT) {
					m.info.names[official] = m.info.names[intermediary].clone();
				}

				if m.info.names[official].is_none() && m.info.names[intermediary].as_ref().is_some_and(|x| !x.as_inner().starts_with("m_")) {
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
			}
		}

		Ok(self)
	}
}

fn add_specialized_methods_to_mappings(
	main_jar: &impl Jar, // official
	calamus: &Mappings<2, (Official, Intermediary)>, // official -> intermediary
	libraries: &[impl Jar], // official
	mappings: Mappings<2, (Intermediary, Named)> // intermediary -> named
) -> Result<Mappings<2, (Intermediary, Named)>> {
	let mut super_classes_provider = vec![main_jar.get_super_classes_provider()?];
	for library in libraries {
		super_classes_provider.push(library.get_super_classes_provider()?);
	}

	let remapper_calamus = calamus.remapper_b(
		calamus.get_namespace("official")?,
		calamus.get_namespace("intermediary")?,
		&super_classes_provider
	)?;
	let x = JarSuperProv::remap(&remapper_calamus, &super_classes_provider)?;
	let remapper_named = mappings.remapper_b(
		mappings.get_namespace("calamus")?,
		mappings.get_namespace("named")?,
		&x
	)?;

	let specialized_methods =
		main_jar.get_specialized_methods()? // official
			.remap(&remapper_calamus)?; // intermediary

	let mut mappings = mappings.clone();

	for (bridge, specialized) in specialized_methods.bridge_to_specialized {
		let named_specialized = remapper_named.map_method_ref_obj(&bridge)?.name;

		let info = MethodMapping {
			names: [specialized.name, named_specialized].into(),
			desc: specialized.desc,
		};

		if let Some(class) = mappings.classes.get_mut(&bridge.class) {
			match class.methods.entry(info.get_key()?) {
				Entry::Occupied(mut e) => {
					if e.get().info != info {
						eprintln!("sus: method already existing: {:?} != {:?}", e.get().info, info);
					} else {
						eprintln!("sus: method already existing: {:?} (eq)", info);
					}

					// only replace the info, not the rest
					e.get_mut().info = info;
				},
				Entry::Vacant(e) => {
					e.insert(MethodNowodeMapping::new(info));
				},
			}
		}
	}

	Ok(mappings)
}
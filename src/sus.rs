use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::map::Entry;
use duke::tree::method::MethodName;
use quill::remapper::{BRemapper, JarSuperProv};
use quill::tree::mappings::{Mappings, MethodMapping, MethodNowodeMapping};
use quill::tree::names::Names;
use quill::tree::{NodeInfo, ToKey};
use crate::download::Downloader;
use dukebox::Jar;
use crate::download::versions_manifest::VersionsManifest;
use crate::specialized_methods::GetSpecializedMethods;
use crate::version_graph::{Environment, Version, VersionGraph};

pub(crate) async fn report_sus(mappings_dir: PathBuf, downloader: Downloader) -> Result<SusResult> {
	let start = Instant::now();

	let v = VersionGraph::resolve(mappings_dir)?;

	println!("graph took {:?}", start.elapsed());

	let version = v.get("1.12.2").unwrap();

	let start = Instant::now();

	let versions_manifest= downloader.get_versions_manifest().await?;
	let result = sus(&downloader, &v, &versions_manifest, version).await?;

	println!("sus took {:?}", start.elapsed());

	Ok(result)
}

#[derive(Debug)]
pub(crate) struct SusResult;

async fn sus(downloader: &Downloader, version_graph: &VersionGraph, versions_manifest: &VersionsManifest, version: &Version) -> Result<SusResult> {
	let environment = version.get_environment();
	let version_details = downloader.version_details(versions_manifest, version, &environment).await?;

	match environment {
		Environment::Merged => {
			let client = downloader.get_jar(&version_details.downloads.client.url).await?;
			let server = downloader.get_jar(&version_details.downloads.server.url).await?;

			let main_jar = dukebox::merge::merge(client, server).with_context(|| anyhow!("failed to merge jars for version {version}"))?;

			sus_inner(downloader, version_graph, versions_manifest, version, &main_jar).await
		},
		Environment::Client => {
			let main_jar = downloader.get_jar(&version_details.downloads.client.url).await?;

			sus_inner(downloader, version_graph, versions_manifest, version, &main_jar).await
		},
		Environment::Server => {
			let main_jar = downloader.get_jar(&version_details.downloads.server.url).await?;

			sus_inner(downloader, version_graph, versions_manifest, version, &main_jar).await
		},
	}

}
async fn sus_inner(downloader: &Downloader, version_graph: &VersionGraph, versions_manifest: &VersionsManifest, version: &Version, main_jar: &impl Jar)
	-> Result<SusResult> {

	println!("sus!");

	let mappings = version_graph.apply_diffs(version)?
		.extend_inner_class_names("named")?
		.remove_dummy("named")?;

	let calamus_v2 = downloader.calamus_v2(version).await?;
	let libraries = downloader.mc_libs(versions_manifest, version).await?;

	let build_feather_tiny = add_specialized_methods_to_mappings(main_jar, &calamus_v2, &libraries, mappings)
		.context("failed to add specialized methods to mappings")?;

	let mappings_a = build_feather_tiny.rename_namespaces(["calamus", "named"], ["intermediary", "named"])?;
	let mappings_b = calamus_v2.reorder(["intermediary", "official"])?;

	let merged = Mappings::merge(&mappings_b, &mappings_a)?.apply_our_fix()?;

	let merge_v2 = merged.reorder(["official", "intermediary", "named"])?;

	Ok(SusResult)
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
			}
		}

		Ok(self)
	}
}

fn add_specialized_methods_to_mappings(
	main_jar: &impl Jar, // official
	calamus: &Mappings<2>, // official -> intermediary
	libraries: &[impl Jar], // official
	mappings: Mappings<2> // intermediary -> named
) -> Result<Mappings<2>> {
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
		let named_specialized = remapper_named.map_method_ref(&bridge)?.name;

		let info = MethodMapping {
			names: Names::try_from([specialized.name, named_specialized])?,
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
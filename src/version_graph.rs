use std::collections::VecDeque;
use anyhow::{anyhow, bail, Context, Result};
use std::fmt::{Debug, Display, Formatter, Write};
use std::path::{Path, PathBuf};
use indexmap::IndexMap;
use petgraph::{Direction, Graph};
use petgraph::graph::NodeIndex;
use quill::tree::mappings::Mappings;
use crate::download::versions_manifest::MinecraftVersion;

/// The version id used in the mappings diffs and mappings files.
/// This can end in `-client` and `-server`, or not have any suffix at all.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub(crate) struct Version(String);

impl Display for Version {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl Version {
	pub(crate) fn as_str(&self) -> &str {
		&self.0
	}

	pub(crate) fn get_environment(&self) -> Environment {
		if self.0.ends_with("-client") {
			Environment::Client
		} else if self.0.ends_with("-server") {
			Environment::Server
		} else {
			Environment::Merged
		}
	}

	pub(crate) fn get_minecraft_version(&self) -> MinecraftVersion {
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
pub(crate) enum Environment {
	Merged,
	Client,
	Server,
}

const MAPPINGS_EXTENSION: &str = ".tiny";
const DIFF_EXTENSION: &str = ".tinydiff";

#[derive(Debug)]
pub(crate) struct VersionGraph {
	root: NodeIndex,
	root_mapping: Mappings<2>,

	versions: IndexMap<Version, NodeIndex>,

	graph: Graph<Version, PathBuf>,
}

impl VersionGraph {
	pub(crate) fn resolve(dir: impl AsRef<Path>) -> Result<VersionGraph> {
		let mut graph: Graph<Version, PathBuf> = Graph::new();

		let mut root: Option<(NodeIndex, PathBuf)> = None;

		let mut versions = IndexMap::new();

		for file in std::fs::read_dir(&dir)
			.with_context(|| anyhow!("cannot read version graph from {:?}", dir.as_ref()))?
		{
			let file = file?;

			let path = file.path();

			let file_name = file.file_name().into_string()
				.map_err(|file_name| anyhow!("failed to turn file name {file_name:?} into a string"))?;

			let mut add_node = |version: &str| *versions.entry(Version(version.to_owned()))
				.or_insert_with_key(|k| graph.add_node(k.clone()));

			if let Some(version) = file_name.strip_suffix(MAPPINGS_EXTENSION) {
				let v = add_node(version);

				if let Some((old_root, ref old_path)) = root {
					bail!("multiple roots present: {old_version:?} ({old_path:?}) and {version} ({path:?})", old_version = &graph[old_root]);
				}
				root = Some((v, path));
			} else if let Some(raw_versions) = file_name.strip_suffix(DIFF_EXTENSION) {
				let Some((parent, version)) = raw_versions.split_once('#') else {
					bail!("expected there to be exactly one `#` in the diff file name {file_name:?}");
				};

				let v = add_node(version);
				let p = add_node(parent);

				graph.add_edge(p, v, path);
			}
		}

		let (root, root_path) = root.context("version graph does not have a root")?;

		let root_mapping = quill::tiny_v2::read_file(&root_path)
			.with_context(|| anyhow!("failed to parse version mapping from {root_path:?}"))?;

		let mut walkers: VecDeque<_> = [ (Vec::new(), root) ].into();
		while let Some((path, head)) = walkers.pop_front() {
			for v in graph.neighbors_directed(head, Direction::Outgoing) {
				if path.contains(&v) {
					bail!("found a loop in the version graph: {:?}", v);
				}

				let mut path = path.clone();
				path.push(v);

				walkers.push_back((path, v));
			}
		}

		Ok(VersionGraph { root, root_mapping, versions, graph })
	}

	pub(crate) fn versions(&self) -> impl Iterator<Item=&Version> + '_ {
		self.versions.keys()
	}

	pub(crate) fn get(&self, string: &str) -> Result<&Version> {
		let version = Version(string.to_owned());
		self.versions.get(&version).copied().map(|node| &self.graph[node]).with_context(|| anyhow!("unknown version {string:?}"))
	}

	pub(crate) fn apply_diffs(&self, target_version: &Version) -> Result<Mappings<2>> {
		let to_node = self.versions.get(target_version).unwrap();

		petgraph::algo::astar(&self.graph, self.root, |n| n == *to_node, |_| 1, |_| 0)
			.ok_or_else(|| anyhow!("there is no path in between {:?} and {target_version:?}", &self.root))?
			.1
			.windows(2) // TODO: once array_windows is stable, use that
			.try_fold(self.root_mapping.clone(), |m, x| {
				let (a, b) = (x[0], x[1]);

				let from = &self.graph[a];
				let to = &self.graph[b];

				let edge = self.graph.find_edge(a, b)
					.ok_or_else(|| anyhow!("there is no edge between {a:?} ({from:?}) and {b:?} ({to:?})"))?;

				let path = &self.graph[edge];

				let diff = quill::tiny_v2_diff::read_file(path)
					.with_context(|| anyhow!("failed to parse version diff from {path:?}"))?;

				diff.apply_to(m, "named")
					.with_context(|| anyhow!("failed to apply diff from version {from:?} to version {to:?} to mappings, for version {target_version:?}"))
			})
	}
}
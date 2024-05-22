use std::collections::VecDeque;
use anyhow::{anyhow, bail, Context, Result};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use indexmap::IndexMap;
use petgraph::{Direction, Graph};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use crate::tree::mappings_diff::MappingsDiff;
use crate::tree::mappings::Mappings;


#[derive(Debug, Clone, PartialEq, Hash, Eq, Deserialize, Serialize)]
pub(crate) struct Version(pub(crate) String);
const MAPPINGS_EXTENSION: &str = ".tiny";
const DIFF_EXTENSION: &str = ".tinydiff";

#[derive(Debug)]
pub(crate) struct VersionGraph {
	root: NodeIndex,
	root_mapping: Mappings<2>,

	versions: IndexMap<Version, NodeIndex>,

	graph: Graph<Version, MappingsDiff>,
}

impl VersionGraph {
	fn add_node(versions: &mut IndexMap<Version, NodeIndex>, graph: &mut Graph<Version, MappingsDiff>, version: &str) -> NodeIndex {
		*versions.entry(Version(version.to_owned()))
			.or_insert_with(|| graph.add_node(Version(version.to_owned())))
	}

	pub(crate) fn resolve(dir: &Path) -> Result<VersionGraph> {
		let mut graph: Graph<Version, MappingsDiff> = Graph::new();

		let mut root = None;
		let mut root_mapping = None;

		let mut versions = IndexMap::new();

		Self::iterate_versions(
			dir,
			|parent, version, path| {
				if let Some(parent) = parent {
					let v = Self::add_node(&mut versions, &mut graph, version);
					let p = Self::add_node(&mut versions, &mut graph, parent);

					let diff = crate::reader::tiny_v2_diff::read_file(&path)
						.with_context(|| anyhow!("failed to parse version diff from {path:?}"))?;

					graph.add_edge(p, v, diff);
				} else {
					if let Some(root) = root {
						bail!("multiple roots present: {:?}, {version} ({path:?})", graph[root]);
					}

					let v = Self::add_node(&mut versions, &mut graph, version);

					let mapping = crate::reader::tiny_v2::read_file(&path)
						.with_context(|| anyhow!("failed to parse version mapping from {path:?}"))?;

					root = Some(v);
					root_mapping = Some(mapping);
				}

				Ok(())
			}
		).context("failed to read versions")?;

		let root = root.context("version graph does not have a root!")?;
		let root_mapping = root_mapping.unwrap(); // see above + setting them together

		let g = VersionGraph {
			root,
			root_mapping,

			versions,

			graph,
		};

		let mut walkers = VecDeque::from([ (Vec::new(), g.root) ]);
		while let Some((path, head)) = walkers.pop_front() {
			for v in g.graph.neighbors_directed(head, Direction::Outgoing) {
				if path.contains(&v) {
					bail!("found a loop in the version graph: {:?}", v);
				}

				let mut path = path.clone();
				path.push(v);

				walkers.push_back((path, v));
			}
		}

		Ok(g)
	}

	fn iterate_versions<F>(dir: &Path, mut operation: F) -> Result<()>
	where
		F: FnMut(Option<&str>, &str, PathBuf) -> Result<()>,
	{
		for file in std::fs::read_dir(dir)? {
			let file = file?;

			let file_name: String = file.file_name().into_string().unwrap();

			if let Some(version) = file_name.strip_suffix(MAPPINGS_EXTENSION) {
				operation(None, version, file.path())
					.with_context(|| anyhow!("failed to read operate on {version} at {file:?}"))?;
			} else if let Some(raw_versions) = file_name.strip_suffix(DIFF_EXTENSION) {
				let versions: Vec<_> = raw_versions.split('#').collect();

				if versions.len() == 2 {
					let parent = versions[0];
					let version = versions[1];

					operation(Some(parent), version, file.path())
						.with_context(|| anyhow!("failed to read operate on {} # {} at {file:?}", versions[0], versions[1]))?;
				} else {
					bail!("expected exactly two versions in diff file name {file_name:?}, got {versions:?}");
				}
			}
		}

		Ok(())
	}

	pub(crate) fn versions(&self) -> impl Iterator<Item=&Version> + '_ {
		self.versions.keys()
	}

	pub(crate) fn get(&self, string: &str) -> Option<&Version> {
		let version = Version(string.to_owned());
		self.versions.get(&version)
			.cloned()
			.map(|node| &self.graph[node])
	}

	pub(crate) fn get_diffs_from_root(&self, to: &Version) -> Result<Vec<(&Version, &Version, &MappingsDiff)>> {

		let to_node = self.versions.get(to).unwrap();

		petgraph::algo::astar(
			&self.graph,
			self.root,
			|n| n == *to_node,
			|_| 1,
			|_| 0
		)
			.ok_or_else(|| anyhow!("there is no path in between {:?} and {to:?}", &self.root))?
			.1
			.windows(2)
			.map(|x| {
				let a = x[0];
				let b = x[1];

				if let Some(edge) = self.graph.find_edge(a, b) {
					Ok((&self.graph[a], &self.graph[b], &self.graph[edge]))
				} else {
					bail!("there is no edge between {a:?} and {b:?}");
				}
			})
			.collect()
	}

	pub(crate) fn apply_diffs(&self, to: &Version) -> Result<Mappings<2>> {
		self.get_diffs_from_root(to)?
			.iter()
			.try_fold(self.root_mapping.clone(), |m, (diff_from, diff_to, diff)| {
				diff.apply_to(m)
					.with_context(|| anyhow!("Failed to apply diff from version {:?} to version {:?} to mappings, for version {:?}",
					diff_from, diff_to, to.clone()
				))
			})
	}

	}
}
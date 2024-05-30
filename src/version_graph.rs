use std::collections::VecDeque;
use anyhow::{anyhow, bail, Context, Result};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use indexmap::IndexMap;
use petgraph::{Direction, Graph};
use petgraph::graph::NodeIndex;
use quill::tree::mappings_diff::MappingsDiff;
use quill::tree::mappings::Mappings;
use crate::Version;

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

					let diff = quill::tiny_v2_diff::read_file(&path)
						.with_context(|| anyhow!("failed to parse version diff from {path:?}"))?;

					graph.add_edge(p, v, diff);
				} else {
					if let Some(root) = root {
						bail!("multiple roots present: {:?}, {version} ({path:?})", graph[root]);
					}

					let v = Self::add_node(&mut versions, &mut graph, version);

					let mapping = quill::tiny_v2::read_file(&path)
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

				let diff = &self.graph[edge];

				diff.apply_to(m, "named")
					.with_context(|| anyhow!("failed to apply diff from version {from:?} to version {to:?} to mappings, for version {target_version:?}"))
			})
	}
}
use anyhow::{anyhow, bail, Context, Result};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use indexmap::IndexMap;
use petgraph::{Direction, Graph};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use crate::tree::mappings_diff::MappingsDiff;
use crate::tree::mappings::Mappings;

#[derive(Debug, Clone)]
pub(crate) enum Format {
	TinyV2,
}

impl Format {
	fn mappings_extension(&self) -> &'static str {
		".tiny"
	}
	fn diff_extension(&self) -> &'static str {
		".tinydiff"
	}
}

#[derive(Debug, Clone, PartialEq, Hash, Eq, Deserialize, Serialize)]
pub(crate) struct Version(pub(crate) String);

#[derive(Debug)]
pub(crate) struct VersionGraph {
	root: NodeIndex,
	root_mapping: Mappings,

	versions: IndexMap<Version, NodeIndex>,

	graph: Graph<Version, MappingsDiff>,
}

impl VersionGraph {
	fn add_node(versions: &mut IndexMap<Version, NodeIndex>, graph: &mut Graph<Version, MappingsDiff>, version: String) -> NodeIndex {
		versions.entry(Version(version.clone()))
			.or_insert_with(|| graph.add_node(Version(version)))
			.clone()
	}

	pub(crate) fn resolve(dir: &Path) -> Result<VersionGraph> {
		let mut graph: Graph<Version, MappingsDiff> = Graph::new();

		let mut root = None;
		let mut root_mapping = None;

		let format = Format::TinyV2;
		let mut versions = IndexMap::new();

		Self::iterate_versions(
			&format,
			dir,
			|parent, version, path| {
				if let Some(parent) = parent {
					let v = Self::add_node(&mut versions, &mut graph, version);
					let p = Self::add_node(&mut versions, &mut graph, parent);

					let diff = crate::reader::tiny_v2_diff::read_file(&path)
						.with_context(|| anyhow!("Failed to parse version diff from {path:?}"))?;

					graph.add_edge(p, v, diff);
				} else {
					if let Some(root) = root {
						bail!("multiple roots present: {:?}, {version} ({path:?})", graph[root]);
					}

					let v = Self::add_node(&mut versions, &mut graph, version);

					let mapping = crate::reader::tiny_v2::read_file(&path)
						.with_context(|| anyhow!("Failed to parse version mapping from {path:?}"))?;

					root = Some(v);
					root_mapping = Some(mapping);
				}

				Ok(())
			}
		).context("Failed to read versions")?;

		let root = root.context("version graph does not have a root!")?;
		let root_mapping = root_mapping.unwrap(); // see above + setting them together

		let mut g = VersionGraph {
			root,
			root_mapping,

			versions,

			graph,
		};

		g.walk()?;

		Ok(g)
	}

	fn iterate_versions<F>(format: &Format, path: &Path, mut operation: F) -> Result<()>
	where
		F: FnMut(Option<String>, String, PathBuf) -> Result<()>,
	{
		for file in std::fs::read_dir(path)? {
			let file = file?;

			let file_name: String = file.file_name().into_string().unwrap();

			if file_name.ends_with(format.mappings_extension()) {
				let version_length = file_name.len() - format.mappings_extension().len();
				let version = file_name.split_at(version_length).0;

				operation(None, version.to_owned(), file.path())
					.with_context(|| anyhow!("Failed to read operate on {version} at {file:?}"))?;
			}

			if file_name.ends_with(format.diff_extension()) {
				let version_length = file_name.len() - format.diff_extension().len();
				let raw_versions = file_name.split_at(version_length).0;

				let versions: Vec<_> = raw_versions.split('#').collect();

				if versions.len() == 2 {
					let parent = versions[0].to_owned();
					let version = versions[1].to_owned();

					operation(Some(parent), version, file.path())
						.with_context(|| anyhow!("Failed to read operate on {} # {} at {file:?}", versions[0], versions[1]))?;
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
			self.root.clone(),
			|n| n == *to_node,
			|_| 1,
			|_| 0
		)
			.ok_or_else(|| anyhow!("There is no path in between {:?} and {to:?}", &self.root))?
			.1
			.windows(2)
			.map(|x| {
				let last = x[0];
				let item = x[1];

				if let Some(edge) = self.graph.find_edge(last, item.clone()) {
					Ok((&self.graph[last], &self.graph[item], &self.graph[edge]))
				} else {
					bail!("there is no edge between {last:?} and {item:?}");
				}
			})
			.collect()
	}

	pub(crate) fn apply_diffs(&self, to: &Version) -> Result<Mappings> {
		self.get_diffs_from_root(to)?
			.iter()
			.try_fold(self.root_mapping.clone(), |m, (diff_from, diff_to, diff)| {
				diff.apply_to(m)
					.with_context(|| anyhow!("Failed to apply diff from version {:?} to version {:?} to mappings, for version {:?}",
					diff_from, diff_to, to.clone()
				))
			})
	}

	fn walk(&self) -> Result<()> {
		let mut walkers = vec![
			(Vec::new(), self.root.clone())
		];

		while !walkers.is_empty() {
			let (path, head) = walkers.remove(0);

			for v in self.graph.neighbors_directed(head, Direction::Outgoing) {
				if path.contains(&v) {
					bail!("found a loop in the version graph: {:?}", v);
				}

				let path = {
					let mut p = path.clone();
					p.push(v.clone());
					p
				};

				walkers.push((path, v));
			}
		}

		Ok(())
	}
}
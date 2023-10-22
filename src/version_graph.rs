use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use petgraph::{Direction, Graph};
use petgraph::graph::NodeIndex;
use crate::tiny::v2::Mappings;
use crate::tiny::v2_diff::{ApplyDiff, Diffs};


#[derive(Debug, Clone)]
pub enum Format {
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

pub struct Version {
	pub name: String,
	mapping: Option<Mappings>,
}

impl Debug for Version {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.name)
	}
}

#[derive(Debug)]
pub struct VersionGraph {
	root: NodeIndex,
	versions: HashMap<String, NodeIndex>,

	graph: Graph<Version, Diffs>,
}

impl VersionGraph {
	fn add_node(versions: &mut HashMap<String, NodeIndex>, graph: &mut Graph<Version, Diffs>, version: String) -> NodeIndex {
		versions.entry(version.clone())
			.or_insert_with(|| graph.add_node(Version {
				name: version,
				mapping: None,
			}))
			.clone()
	}

	pub fn resolve(dir: &Path) -> Result<VersionGraph> {
		let mut graph: Graph<Version, Diffs> = Graph::new();

		let mut root = None;

		let format = Format::TinyV2;
		let mut versions = HashMap::new();

		Self::iterate_versions(
			&format,
			dir,
			|parent, version, path| {
				if let Some(parent) = parent {
					let v = Self::add_node(&mut versions, &mut graph, version);
					let p = Self::add_node(&mut versions, &mut graph, parent);

					let diff = crate::tiny::v2_diff::read(&path)
						.with_context(|| anyhow!("Failed to parse version diff from {path:?}"))?;

					graph.add_edge(p, v, diff);
				} else {
					if let Some(root) = root {
						bail!("multiple roots present: {:?}, {version} ({path:?})", graph[root]);
					}

					let v = Self::add_node(&mut versions, &mut graph, version);

					let root_mapping = crate::tiny::v2::read(&path)
						.with_context(|| anyhow!("Failed to parse version mapping from {path:?}"))?;

					graph[v].mapping = Some(root_mapping);

					root = Some(v);
				}

				Ok(())
			}
		).context("Failed to read versions")?;

		let root = root.context("version graph does not have a root!")?;

		let mut g = VersionGraph {
			root,
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

	pub fn versions(&self) -> impl Iterator<Item=NodeIndex> + '_ {
		self.graph.node_indices()
	}

	pub fn get(&self, s: NodeIndex) -> Result<&Version> {
		Ok(&self.graph[s])
	}

	pub fn get_node(&self, string: &str) -> Option<NodeIndex> {
		self.versions.get(string).cloned()
	}

	pub fn get_diffs_from_root(&self, to: NodeIndex) -> Result<Vec<(NodeIndex, NodeIndex, &Diffs)>> {
		let mut diffs = Vec::new();

		petgraph::algo::astar(
			&self.graph,
			self.root.clone(),
			|n| n == to,
			|_| 1,
			|_| 0
		)
			.ok_or_else(|| anyhow!("there is no path in between {:?} and {to:?}", &self.root))?
			.1
			.into_iter()
			.try_fold(None, |acc, item| {
				if let Some(last) = acc {
					if let Some(edge) = self.graph.find_edge(last, item.clone()) {
						diffs.push((last, item, &self.graph[edge]));
					} else {
						bail!("there is no edge between {last:?} and {item:?}");
					}
				}
				Ok(Some(item))
			})?;

		Ok(diffs)
	}

	pub fn apply_diffs(&self, to: NodeIndex) -> Result<Mappings> {
		let diffs = self.get_diffs_from_root(to)?;

		let mut m = self.graph[self.root].mapping
			.clone()
			.context("No mapping for root node")?;

		for (diff_from, diff_to, diff) in diffs {
			diff.apply_to(&mut m)
				.with_context(|| anyhow!("Failed to apply diff (from version {} to version {}) to mappings, for version {}",
					self.graph[diff_from].name, self.graph[diff_to].name, self.graph[to].name
				))?;
		}

		Ok(m)
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
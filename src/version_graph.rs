use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use petgraph::{Direction, Graph};
use petgraph::graph::NodeIndex;
use crate::reader::tiny_v2::Mapping;
use crate::reader::tiny_v2_diff::Diff;


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

#[derive(Debug)]
pub struct VersionGraph {
	format: Format,

	root_node: NodeIndex,
	root_mapping: VersionMapping,
	versions: HashMap<String, NodeIndex>,

	pub graph: Graph<Version, VersionDiff>,
}

impl VersionGraph {
	fn add_node(versions: &mut HashMap<String, NodeIndex>, graph: &mut Graph<Version, VersionDiff>, version: String, format: &Format) -> NodeIndex {
		versions.entry(version.clone())
			.or_insert_with(|| {
				graph.add_node(Version::new(version, format.clone()))
			})
			.clone()
	}

	pub fn resolve(dir: &Path) -> Result<VersionGraph> {
		let mut graph: Graph<Version, Diffs> = Graph::new();

		let mut root = None;

		let format = Format::TinyV2;
		let mut versions = HashMap::new();

		Self::iterate_versions(
			format.clone(),
			dir,
			|parent, version, path| {
				if let Some(parent) = parent {
					let v = Self::add_node(&mut versions, &mut graph, version, &format);
					let p = Self::add_node(&mut versions, &mut graph, parent, &format);

					graph.add_edge(p, v, VersionDiff::create(path.clone())
						.with_context(|| anyhow!("Failed to parse version diff from {:?}", path))?);
				} else {
					if let Some((ref root_path, root)) = root {
						bail!("multiple roots present: {:?} ({root_path:?}), {version} ({path:?})", graph[root]);
					}

					let v = Self::add_node(&mut versions, &mut graph, version, &format);
					root = Some((path, v));
				}

				Ok(())
			}
		).context("Failed to read versions")?;

		let (root_path, root_node) = root.context("version graph does not have a root!")?;

		let root_mapping = VersionMapping::create(root_path)?;

		let mut g = VersionGraph {
			format,
			root_node,
			root_mapping,
			versions,

			graph,
		};

		// validate graph, populate depth
		g.walk(g.root_node.clone(), |g, p| {
			let mut depth = 0;

			for v in p {
				if g[v].depth < depth {
					g[v].depth = depth;
				}

				depth += 1;
			}
		})?;

		Ok(g)
	}

	fn iterate_versions<F>(format: Format, path: &Path, mut operation: F) -> Result<()>
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

	fn walk<P>(&mut self, start: NodeIndex, mut path_visitor: P) -> Result<()>
	where
		P: FnMut(&mut Graph<Version, VersionDiff>, Vec<NodeIndex>) -> (),
	{
		let mut walkers = vec![
			GraphWalker::from(start)
		];

		while !walkers.is_empty() {
			let curr = walkers.remove(0);

			for v in self.graph.neighbors_directed(curr.head, Direction::Outgoing) {
				walkers.push(curr.walk_to(&self.graph, v)?);
			}

			if self.graph.neighbors_directed(curr.head, Direction::Outgoing).next().is_none() {
				path_visitor(&mut self.graph, curr.path);
			}
		}

		Ok(())
	}
}

#[derive(Debug, Clone)]
struct GraphWalker {
	path: Vec<NodeIndex>,
	head: NodeIndex,
}

impl GraphWalker {
	fn walk_to(&self, graph: &Graph<Version, VersionDiff>, v: NodeIndex) -> Result<GraphWalker> {
		if let Some(other_index) = self.path.iter().position(|x| x == &v) {
			let l = self.path[other_index..]
				.into_iter()
				.cloned()
				.map(|x| &graph[x]);

			let v = &graph[v];

			bail!("found a loop in the version graph: ({other_index}) {:?} + {:?}", l, v);
		}

		Ok(GraphWalker {
			path: {
				let mut path = self.path.clone();
				path.push(v.clone());
				path
			},
			head: v,
		})
	}
}

impl From<NodeIndex> for GraphWalker {
	fn from(head: NodeIndex) -> Self {
		GraphWalker {
			path: Vec::new(),
			head,
		}
	}
}

#[derive(Debug)]
pub struct Version {
	version: String,
	format: Format,

	depth: usize,
}


impl Version {
	fn new(version: String, format: Format) -> Version {
		Version {
			version, format,

			depth: 0,
		}
	}
}


impl PartialEq for Version {
	fn eq(&self, other: &Self) -> bool {
		self.version == other.version
	}
}

pub struct VersionDiff {
	path: PathBuf,
	diff: Diff,
}

impl Debug for VersionDiff {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.path.fmt(f)
	}
}

impl VersionDiff {
	fn create(path: PathBuf) -> Result<VersionDiff> {
		Ok(VersionDiff {
			diff: crate::reader::tiny_v2_diff::read(&path)?,
			path,
		})
	}
}

pub struct VersionMapping {
	path: PathBuf,
	mapping: Mapping,
}

impl Debug for VersionMapping {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.path.fmt(f)
	}
}

impl VersionMapping {
	fn create(path: PathBuf) -> Result<VersionMapping> {
		Ok(VersionMapping {
			mapping: crate::reader::tiny_v2::read(&path)?,
			path,
		})
	}
}
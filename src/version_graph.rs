use std::collections::VecDeque;
use anyhow::{anyhow, bail, Context, Result};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use indexmap::IndexMap;
use petgraph::{Direction, Graph};
use petgraph::graph::NodeIndex;
use quill::tree::mappings::Mappings;
use quill::tree::mappings_diff::MappingsDiff;
use crate::{Intermediary, Named};
use crate::download::versions_manifest::MinecraftVersion;

const VERSION_SHORTCUTS: [(&str, &str); 76] = [
	//("a1.0.5", "a1.0.5-2149"),
	//("a1.0.13_01", "a1.0.13_01-1444"),
	//("a1.0.14", "a1.0.14-1659"),
	//("a1.1.0", "a1.1.0-131933"),
	//("a1.2.3_01", "a1.2.3_01-0958"),
	//("server-a0.2.5", "server-a0.2.5-1004"),
	("a1.0.15", "a1.0.15~server-a0.1.0"),
	("server-a0.1.0", "a1.0.15~server-a0.1.0"),
	("a1.0.16", "a1.0.16~server-a0.1.1-1707"),
	("server-a0.1.1", "a1.0.16~server-a0.1.1-1707"),
	("a1.0.16_01", "a1.0.16_01~server-a0.1.2_01"),
	("server-a0.1.2_01", "a1.0.16_01~server-a0.1.2_01"),
	("a1.0.16_02", "a1.0.16_02~server-a0.1.3"),
	("server-a0.1.3", "a1.0.16_02~server-a0.1.3"),
	("a1.0.17_02", "a1.0.17_02~server-a0.1.4"),
	("server-a0.1.4", "a1.0.17_02~server-a0.1.4"),
	("a1.1.1", "a1.1.1~server-a0.2.1"),
	("server-a0.2.1", "a1.1.1~server-a0.2.1"),
	("a1.2.0", "a1.2.0~server-a0.2.2"),
	("server-a0.2.2", "a1.2.0~server-a0.2.2"),
	("a1.2.0_01", "a1.2.0_01~server-a0.2.2_01"),
	("server-a0.2.2_01", "a1.2.0_01~server-a0.2.2_01"),
	("a1.2.1_01", "a1.2.1_01~server-a0.2.3"),
	("server-a0.2.3", "a1.2.1_01~server-a0.2.3"),
	("a1.2.3", "a1.2.3~server-a0.2.5-0923"),
	("server-a0.2.5", "a1.2.3~server-a0.2.5-0923"),
	("a1.2.3_02", "a1.2.3_02~server-a0.2.5_01"),
	("server-a0.2.5_01", "a1.2.3_02~server-a0.2.5_01"),
	("a1.2.3_04", "a1.2.3_04~server-a0.2.5_02"),
	("server-a0.2.5_02", "a1.2.3_04~server-a0.2.5_02"),
	("a1.2.3_05", "a1.2.3_05~server-a0.2.6"),
	("server-a0.2.6", "a1.2.3_05~server-a0.2.6"),
	("a1.2.4_01", "a1.2.4_01~server-a0.2.6_02"),
	("server-a0.2.6_02", "a1.2.4_01~server-a0.2.6_02"),
	("a1.2.5", "a1.2.5~server-a0.2.7"),
	("server-a0.2.7", "a1.2.5~server-a0.2.7"),
	("a1.2.6", "a1.2.6~server-a0.2.8"),
	("server-a0.2.8", "a1.2.6~server-a0.2.8"),
	("b1.1", "b1.1-1245"),
	("b1.1-client", "b1.1-1255"),
	("b1.3-client", "b1.3-1750-client"),
	("b1.3-server", "b1.3-1731-server"),
	("b1.4", "b1.4-1507"),
	("b1.4-client", "b1.4-1634"),
	("b1.8-pre1-client", "b1.8-pre1-201109081459"),
	("b1.8-pre1", "b1.8-pre1-201109091357"),
	("b1.9-pre3", "b1.9-pre3-201110061350"),
	("b1.9-pre3-client", "b1.9-pre3-201110061402"),
	("b1.9-pre4", "b1.9-pre4-201110131434"),
	("b1.9-pre4-server", "b1.9-pre4-201110131440"),
	("12w05a", "12w05a-1442"),
	("1.0", "1.0.0"),
	("1.3", "1.3-pre-07261249"),
	("1.4", "1.4-pre"),
	("1.4.1", "1.4.1-pre-10231538"),
	("1.4.3", "1.4.3-pre"),
	("13w03a", "13w03a-1647"),
	("13w05a", "13w05a-1538"),
	("13w06a", "13w06a-1636"),
	("13w16a", "13w16a-04192037"),
	("13w16b", "13w16b-04232151"),
	("13w23b", "13w23b-06080101"),
	("1.6", "1.6-pre-06251516"),
	("1.6.2", "1.6.2-091847"),
	("1.6.3", "1.6.3-pre-171231"),
	("13w36a", "13w36a-09051446"),
	("13w36b", "13w36b-09061310"),
	("13w41b", "13w41b-1523"),
	("1.7", "1.7-pre"),
	("1.7.1", "1.7.1-pre"),
	("1.7.3", "1.7.3-pre"),
	("1.7.7", "1.7.7-101331"),
	("14w04b", "14w04b-1554"),
	("14w27b", "14w27b-07021646"),
	("14w34c", "14w34c-08191549"),
	("16w50a", "16w50a-1438"),
	("1.12-pre3", "1.12-pre3-1409"),
	("2point0_red", "af-2013-red"),
	("2point0_blue", "af-2013-blue"),
	("2point0_purple", "af-2013-purple"),
	("15w14a", "af-2015"),
	("1.RV-Pre1", "af-2016"),
];

pub(crate) fn map_shortcut(version: &str) -> &str {
	for (shortcut, long) in VERSION_SHORTCUTS {
		if shortcut == version {
			return long
		}
	}
	version
}

const MAPPINGS_EXTENSION: &str = ".tiny";
const DIFF_EXTENSION: &str = ".tinydiff";


#[derive(Clone, Debug)]
struct NodeData {
	name: String,
	depth: usize,
}

#[derive(Debug)]
struct EdgeData {
	path: PathBuf,
}

/// the version names are split at `~`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Split {
	None,
	First,
	Second,
}

pub(crate) struct VersionGraph {
	root: NodeIndex,
	root_mapping: Mappings<2, (Intermediary, Named)>,

	/// A map to look up versions to build
	versions: IndexMap<String, (Split, NodeIndex)>,

	graph: Graph<NodeData, EdgeData>,
}

/// The version id in a way that can be looked up in the Ornithe API
#[derive(Clone, Copy, Debug)]
pub(crate) struct MinecraftVersionBorrowed<'a>(&'a str);

#[deprecated]
#[derive(Debug, PartialEq)]
pub(crate) enum Environment {
	Merged,
	Client,
	Server,
}

/// A version as in the version graph
///
/// This will be one like `a1.2.6~server-a0.2.8` or `1.0.0`
///
/// PartialEq/Hash behave as if you'd do them on the version string.
#[derive(Clone, Copy)]
pub(crate) struct VersionEntry<'a> {
	node_index: NodeIndex,
	node_data: &'a NodeData,
}

pub(crate) struct VersionEntryOwned {
	node_index: NodeIndex,
	node_data: NodeData,
}

impl NodeData {
	fn new(name: &str) -> NodeData {
		NodeData {
			name: name.to_owned(),
			depth: 0,
		}
	}
}

impl VersionGraph {
	pub(crate) fn is_root_then_get_mappings(&self, version: VersionEntry<'_>) -> Option<&Mappings<2, (Intermediary, Named)>> {
		if version.node_index == self.root {
			Some(&self.root_mapping)
		} else {
			None
		}
	}

	pub(crate) fn get_diff(&self, parent: VersionEntry<'_>, version: VersionEntry<'_>) -> Result<Option<MappingsDiff>> {
		let Some(edge) = self.graph.find_edge(parent.node_index, version.node_index) else {
			return Ok(None);
		};

		let path = &self.graph[edge].path;

		quill::tiny_v2_diff::read_file(path)
			.with_context(|| anyhow!("failed to parse version diff from {path:?}"))
			.map(Some)
	}

	pub(crate) fn write(&self) {
		for v in self.graph.node_indices() {
			// TODO: call write_mappings or write_diffs depending on root/not root
			/*
			#[allow(clippy::collapsible_else_if)]
			if root {
				if mappings != null && dirty {
					format.write_mappings(paths.get(this), mappings);
				}
			} else {
				if dirty {
					for version in diffs.keys() {
						let diff = diffs.get(version);
						let path = paths.get(version);

						format.write_diff(path, diff);
					}
				}
			}*/
		}
	}

	pub(crate) fn write_as_dot(&self, w: &mut impl Write) -> Result<()> {
		let dot = petgraph::dot::Dot::new(&self.graph);
		write!(w, "{:?}", dot)
			.with_context(|| anyhow!("failed to write version graph in `.dot` format"))
	}

	pub(crate) fn parents<'a>(&'a self, version: VersionEntry<'a>) -> impl Iterator<Item=VersionEntry<'a>> {
		self.graph.neighbors_directed(version.node_index, Direction::Incoming)
			.map(|node_index| VersionEntry {
				node_index,
				node_data: &self.graph[node_index],
			})
	}
	pub(crate) fn children<'a>(&'a self, version: VersionEntry<'a>) -> impl Iterator<Item=VersionEntry<'a>> {
		self.graph.neighbors_directed(version.node_index, Direction::Outgoing)
			.map(|node_index| VersionEntry {
				node_index,
				node_data: &self.graph[node_index],
			})
	}
	pub(crate) fn resolve(dir: impl AsRef<Path>) -> Result<VersionGraph> {
		let mut graph: Graph<NodeData, EdgeData> = Graph::new();

		let mut root: Option<(NodeIndex, PathBuf)> = None;

		let mut versions = IndexMap::new();

		for file in std::fs::read_dir(&dir)
			.with_context(|| anyhow!("cannot read version graph from {:?}", dir.as_ref()))?
		{
			let file = file?;

			let path = file.path();

			let file_name = file.file_name().into_string()
				.map_err(|file_name| anyhow!("failed to turn file name {file_name:?} into a string"))?;

			fn add_node(
				versions: &mut IndexMap<String, (Split, NodeIndex)>,
				graph: &mut Graph<NodeData, EdgeData>,
				version_str: &str
			) -> NodeIndex {
				if let Some((client, server)) = version_str.split_once('~') {
					let node_index = versions.entry(client.to_owned())
						.or_insert_with(|| (Split::First, graph.add_node(NodeData::new(version_str)))).1;

					versions.entry(server.to_owned())
						.or_insert((Split::Second, node_index));

					node_index
				} else {
					versions.entry(version_str.to_owned())
						.or_insert_with_key(|k| (Split::None, graph.add_node(NodeData::new(k)))).1
				}
			}

			if let Some(version_str) = file_name.strip_suffix(MAPPINGS_EXTENSION) {

				let v = add_node(&mut versions, &mut graph, version_str);

				if let Some((old_root, ref old_path)) = root {
					bail!("multiple roots present: {old_version:?} ({old_path:?}) and {version_str:?} ({path:?})", old_version = &graph[old_root].name);
				}
				root = Some((v, path));
			} else if let Some(raw_versions) = file_name.strip_suffix(DIFF_EXTENSION) {
				let Some((parent, version)) = raw_versions.split_once('#') else {
					bail!("expected there to be exactly one `#` in the diff file name {file_name:?}");
				};

				let v = add_node(&mut versions, &mut graph, version);
				let p = add_node(&mut versions, &mut graph, parent);

				let edge = EdgeData { path };

				graph.add_edge(p, v, edge);
			}
		}

		let (root, root_path) = root.context("version graph does not have a root")?;

		let root_mapping = quill::tiny_v2::read_file(&root_path)
			.with_context(|| anyhow!("failed to parse version mapping from {root_path:?}"))?
			.contract_inner_class_names("named")?;

		let mut walkers: VecDeque<_> = [ (Vec::new(), root) ].into();
		while let Some((path, head)) = walkers.pop_front() {
			if head != root {
				if graph[head].depth == 0 {
					graph[head].depth = path.len();
				} else {
					// we got two (or more) depths for a node:
					let old_depth = graph[head].depth;
					let new_depth = path.len();

					graph[head].depth = new_depth.min(old_depth);
				}
				// since all nodes are connected, after this all of them will have their depth with them
			}

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

	pub(crate) fn versions(&self) -> impl Iterator<Item=VersionEntry<'_>> {
		self.graph.node_indices()
			.map(|node_index| VersionEntry {
				node_index,
				node_data: &self.graph[node_index],
			})
	}

	pub(crate) fn get(&self, string: &str) -> Result<(Split, VersionEntry<'_>)> {
		self.versions.get(string)
			.map(|&(split, node_index)| (split, VersionEntry {
				node_index,
				node_data: &self.graph[node_index],
			}))
			.with_context(|| anyhow!("unknown version {string:?}"))
	}

	pub(crate) fn get_all(&self, iter: impl IntoIterator<Item=impl AsRef<str>>) -> Result<Vec<(Split, VersionEntry<'_>)>> {
		iter.into_iter()
			.map(|string| self.get(string.as_ref()))
			.collect()
	}

	pub(crate) fn apply_diffs(&self, target_version: VersionEntry<'_>) -> Result<Mappings<2, (Intermediary, Named)>> {
		petgraph::algo::astar(
			&self.graph,
			self.root,
			|n| n == target_version.node_index,
			|_| 1,
			|_| 0
		)
			.ok_or_else(|| anyhow!("there is no path in between {:?} and {:?}", &self.root, target_version))?
			.1
			.windows(2) // TODO: once array_windows is stable, use that
			.try_fold(self.root_mapping.clone(), |m, x| {
				let (a, b) = (x[0], x[1]);

				let from = &self.graph[a].name;
				let to = &self.graph[b].name;

				let edge = self.graph.find_edge(a, b)
					.ok_or_else(|| anyhow!("there is no edge between {a:?} ({from:?}) and {b:?} ({to:?})"))?;

				let path = &self.graph[edge].path;

				let diff = quill::tiny_v2_diff::read_file(path)
					.with_context(|| anyhow!("failed to parse version diff from {path:?}"))?;

				diff.apply_to(m, "named")
					.with_context(|| anyhow!("failed to apply diff from version {from:?} to version {to:?} to mappings, for version {:?}", target_version))
			})?
			.extend_inner_class_names("named")
	}
}

impl PartialEq<MinecraftVersionBorrowed<'_>> for MinecraftVersion {
	fn eq(&self, other: &MinecraftVersionBorrowed<'_>) -> bool {
		self.0 == other.0
	}
}

impl VersionEntry<'_> {
	/// Gets the version string (possibly ending in `-client` or `-server`).
	///
	/// Do not use this for debug! This type implements [`Debug`].
	pub(crate) fn as_str(&self) -> &str {
		&self.node_data.name
	}

	pub(crate) fn depth(&self) -> usize {
		self.node_data.depth
	}

	pub(crate) fn make_owned(&self) -> VersionEntryOwned {
		VersionEntryOwned {
			node_index: self.node_index,
			node_data: self.node_data.clone(),
		}
	}

	#[deprecated]
	pub(crate) fn get_minecraft_version(&self) -> MinecraftVersionBorrowed<'_> {
		if let Some(without) = self.as_str().strip_suffix("-client") {
			MinecraftVersionBorrowed(without)
		} else if let Some(without) = self.as_str().strip_suffix("-server") {
			MinecraftVersionBorrowed(without)
		} else {
			MinecraftVersionBorrowed(self.as_str())
		}
	}

	#[deprecated]
	pub(crate) fn get_environment(&self) -> Environment {
		if self.as_str().ends_with("-client") {
			Environment::Client
		} else if self.as_str().ends_with("-server") {
			Environment::Server
		} else {
			Environment::Merged
		}
	}
}

impl Debug for VersionEntry<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.as_str())
	}
}

impl PartialEq for VersionEntry<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.node_data.name == other.node_data.name
	}
}
impl Eq for VersionEntry<'_> { }
impl Hash for VersionEntry<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.node_data.name.hash(state)
	}
}

impl VersionEntryOwned {
	pub(crate) fn make_borrowed(&self) -> VersionEntry<'_> {
		VersionEntry {
			node_index: self.node_index,
			node_data: &self.node_data,
		}
	}
}

#[cfg(test)]
mod testing {
	use std::collections::HashSet;
	use pretty_assertions::assert_eq;
	use anyhow::Result;
	use std::path::Path;
	use quill::tree::NodeInfo;
	use crate::version_graph::{Split, VersionGraph};

	#[test]
	fn versions() -> Result<()> {
		let graph = VersionGraph::resolve(Path::new("tests/version-graph/graph"))?;

		let mut vec = Vec::new();
		graph.write_as_dot(&mut vec)?;
		let s = String::from_utf8(vec)?;
		println!("{s}");

		let versions: HashSet<_> = graph.versions().map(|x| x.as_str().to_owned()).collect();;
		assert_eq!(versions, HashSet::from([
			"1.3",
			"1.4~server-0.4",
			"1.5",
			"1.2~server-0.2",
			"1.1~server-0.1",
		].map(|x| x.to_owned())));

		fn check_version(graph: &VersionGraph, get_string: Result<&str, (&str, &str)>, expected_version: &str, expected_mappings: &str) -> Result<()> {
			let v = match get_string {
				Ok(get_string) => {
					let (split, v) = graph.get(get_string)?;
					assert_eq!(split, Split::None);
					assert_eq!(v.as_str(), expected_version);

					v
				},
				Err((get_string_a, get_string_b)) => {
					let (split, v) = graph.get(get_string_a)?;
					assert_eq!(split, Split::First);
					assert_eq!(v.as_str(), expected_version);

					let (split, v) = graph.get(get_string_b)?;
					assert_eq!(split, Split::Second);
					assert_eq!(v.as_str(), expected_version);

					v
				},
			};

			let actual = graph.apply_diffs(v)?;
			let actual = quill::tiny_v2::write_string(&actual)?;
			assert_eq!(actual, expected_mappings, "mappings didn't match for version {expected_version:?}");

			Ok(())
		}

		check_version(&graph, Ok("1.3"), "1.3", include_str!("../tests/version-graph/expected/1.3.tiny"))?;
		check_version(&graph, Err(("1.4", "server-0.4")), "1.4~server-0.4", include_str!("../tests/version-graph/expected/1.4~server-0.4.tiny"))?;
		check_version(&graph, Ok("1.5"), "1.5", include_str!("../tests/version-graph/expected/1.5.tiny"))?;
		check_version(&graph, Err(("1.2", "server-0.2")), "1.2~server-0.2", include_str!("../tests/version-graph/expected/1.2~server-0.2.tiny"))?;
		check_version(&graph, Err(("1.1", "server-0.1")), "1.1~server-0.1", include_str!("../tests/version-graph/expected/1.1~server-0.1.tiny"))?;

		Ok(())
	}
}

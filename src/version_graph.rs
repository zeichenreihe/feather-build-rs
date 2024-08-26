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

const VERSION_SHORTCUTS: [(&str, &str); 53] = [
	("a1.0.5", "a1.0.5-2149-client"),
	("a1.0.13_01", "a1.0.13_01-1444-client"),
	("a1.0.14", "a1.0.14-1659-client"),
	("a1.1.0", "a1.1.0-131933-client"),
	("a1.2.3_01", "a1.2.3_01-0958-client"),
	("a0.2.5-server", "server-a0.2.5-1004-server"),
	("server-a0.2.5", "server-a0.2.5-1004-server"),
	("b1.1-client", "b1.1-1255-client"),
	("b1.1-server", "b1.1-1245-server"),
	("b1.3-client", "b1.3-1750-client"),
	("b1.3-server", "b1.3-1731-server"),
	("b1.4-client", "b1.4-1634-client"),
	("b1.4-server", "b1.4-1507-server"),
	("b1.8-pre1-server", "b1.8-pre1-201109091357-server"),
	("b1.9-pre3-client", "b1.9-pre3-201110061402-client"),
	("b1.9-pre3-server", "b1.9-pre3-201110061350-server"),
	("b1.9-pre4-client", "b1.9-pre4-201110131434-client"),
	("b1.9-pre4-server", "b1.9-pre4-201110131440-server"),
	("12w05a", "12w05a-1442"),
	("12w05a-client", "12w05a-1442-client"),
	("12w05a-server", "12w05a-1442-server"),
	("1.0", "1.0.0"),
	("1.0-client", "1.0.0-client"),
	("1.0-server", "1.0.0-server"),
	("1.3", "1.3-pre-07261249"),
	("1.4", "1.4-pre"),
	("1.4.1", "1.4.1-pre-10231538"),
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

fn map_shortcut(version: &str) -> &str {
	for (shortcut, long) in VERSION_SHORTCUTS {
		if shortcut == version {
			return long
		}
	}
	version
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
		let string = map_shortcut(string);
		let version = Version(string.to_owned());
		self.versions.get(&version)
			.copied()
			.map(|node| &self.graph[node])
			.with_context(|| anyhow!("unknown version {string:?}"))
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
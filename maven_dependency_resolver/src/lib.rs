pub mod coord;
pub mod maven_pom;
mod maven_pom_done;
pub mod resolver;
pub mod tree;

use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::str::FromStr;
use anyhow::{anyhow, bail, Context, Error, Result};
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use crate::coord::MavenCoord;
use crate::maven_pom::MavenPom;
use crate::maven_pom_done::{get_merged_pom};
use crate::resolver::Resolver;
use crate::tree::{Forest, Tree};

/// A scope for a dependency.
///
/// Note: this type supports round trips with [Display] and [FromStr].
#[derive(Copy, Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub enum DependencyScope {
	#[default]
	#[serde(rename = "compile")]
	Compile,
	#[serde(rename = "runtime")]
	Runtime,
	#[serde(rename = "test")]
	Test,
	#[serde(rename = "system")]
	System,
	#[serde(rename = "provided")]
	Provided,
}

impl Display for DependencyScope {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		Display::fmt(match self {
			DependencyScope::Compile => "compile",
			DependencyScope::Runtime => "runtime",
			DependencyScope::Test => "test",
			DependencyScope::System => "system",
			DependencyScope::Provided => "provided",
		}, f)
	}
}

impl FromStr for DependencyScope {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"compile" => DependencyScope::Compile,
			"runtime" => DependencyScope::Runtime,
			"test" => DependencyScope::Test,
			"system" => DependencyScope::System,
			"provided" => DependencyScope::Provided,
			scope => bail!("unknown scope {scope:?}, scope is one of \"compile\", \"runtime\", \"test\", \"system\" and \"provided\""),
		})
	}
}

pub trait Downloader {
	// note: can't rewrite with async, bc of `+ Send`
	#[allow(clippy::manual_async_fn)]
	fn get_maven_pom(&self, url: &str) -> impl Future<Output = Result<Option<MavenPom>>> + Send;
}

impl MavenPom {
	fn get_parent_coord(&self) -> Option<MavenCoord> {
		self.parent.as_ref().map(|parent| MavenCoord {
			group: parent.group_id.clone(),
			artifact: parent.artifact_id.clone(),
			version: parent.version.clone(),
			classifier: None,
			type_: "pom".to_owned()
		})
	}
}

// TODO: doc
pub async fn get_maven_dependencies<'a>(downloader: &(impl Downloader + Sync), resolvers: &'a [Resolver<'_>],
		dependencies_list: &[(MavenCoord, DependencyScope)]) -> Result<Vec<FoundDependency<'a>>> {

	let mut dependencies_forest = Vec::with_capacity(dependencies_list.len());

	for (coord, sc) in dependencies_list {
		let c = get_dependencies_tree(downloader, resolvers, coord, *sc).await?;

		dependencies_forest.push(c);
	}

	let cleaned_dependencies_forest = clean_up_dependencies(dependencies_forest);

	Ok(Forest::into_breadth_first(cleaned_dependencies_forest).collect())
}

/// Note that gradle, other than maven, does select the highest of the dependencies found, and not the "nearest" one.
// TODO? implement a gradle like filtering as well?
fn clean_up_dependencies(mut forest: Vec<Tree<FoundDependency<'_>>>) -> Vec<Tree<FoundDependency<'_>>> {

	let mut set = forest.iter()
		.flat_map(Tree::breadth_first)
		.map(|dep| dep.coord.dependency_collision_id())
		.collect::<HashSet<_>>();

	// note that this keeps the first dependency found, and removes all later ones with the same collision id
	Forest::breadth_first_retain(&mut forest, |dep| {
		// retain if it was contained
		set.remove(&dep.coord.dependency_collision_id())
	});

	forest
}

/// A resolved dependency.
///
/// [FoundDependency] implements [TryFrom<&str>]. Format is `group:artifact[:type[:classifier]]:version:scope @ url`.
/// This format is adopted from [MavenCoord]. Note that [TryFrom<&str>] is used instead of [FromStr] because the latter
/// doesn't allow us to specify the lifetime.
///
/// The [Display] implementation allows round trips with [TryFrom<&str>]. Note: round trips remove the repositories name.
// TODO: tests for both TryFrom<&str>, and Display...
#[derive(Debug, PartialEq)]
pub struct FoundDependency<'a> {
	pub resolver: Resolver<'a>,
	pub coord: MavenCoord,
	pub scope: DependencyScope,
}

impl FoundDependency<'_> {
	pub fn make_url(&self) -> String {
		self.coord.make_url(&self.resolver)
	}
}

impl Display for FoundDependency<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{coord}:{scope} @ {url}",
			coord = self.coord,
			scope = self.scope,
			url = self.resolver.maven,
		)
	}
}

impl<'a> TryFrom<&'a str> for FoundDependency<'a> {
	type Error = Error;

	fn try_from(value: &'a str) -> std::result::Result<Self, Self::Error> {
		let (left, url) = value.split_once(" @ ")
			.with_context(|| anyhow!("expected \" @ \" to separate the url from the main part: {value:?}"))?;
		let (coord, scope) = left.rsplit_once(':')
			.with_context(|| anyhow!("expected \":\" to separate the scope from the coordinate: {value:?}"))?;

		Ok(FoundDependency {
			resolver: Resolver::new(url, url),
			coord: MavenCoord::from_str(coord).with_context(|| anyhow!("failed to parse coordinate part: {value:?}"))?,
			scope: DependencyScope::from_str(scope).with_context(|| anyhow!("failed to parse scope part: {value:?}"))?,
		})
	}
}

// TODO: recursion limiter!
#[async_recursion::async_recursion]
async fn get_dependencies_tree<'a>(downloader: &(impl Downloader + Sync), resolvers: &'a [Resolver], coord: &MavenCoord, scope: DependencyScope)
		-> Result<Tree<FoundDependency<'a>>> {

	let (resolver, pom) = get_merged_pom(downloader, resolvers, coord).await?;

	let mut tree = Tree::new(FoundDependency {
		resolver: resolver.clone(),
		coord: coord.clone(),
		scope,
	});

	for dependency in pom.dependencies {
		let is_optional = dependency.optional.unwrap_or(false);
		let dependency_scope = dependency.scope.unwrap_or(DependencyScope::Compile);

		if !is_optional {
			/// None means the `-` from the [table](https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html#dependency-scope)
			fn the_scope_table(left_column: DependencyScope, top_row: DependencyScope) -> Option<DependencyScope> {
				match (top_row, left_column) {
					(DependencyScope::Runtime, DependencyScope::Compile) => Some(DependencyScope::Runtime),
					(DependencyScope::Compile | DependencyScope::Runtime, left_column) => Some(left_column),
					(DependencyScope::Provided | DependencyScope::Test | DependencyScope::System, _) => None, // a comment says "system" is similar to "provided"
				}
			}

			// this skips all the ones with None
			if let Some(scope_after_table) = the_scope_table(scope, dependency_scope) {
				let c = get_dependencies_tree(downloader, resolvers, &dependency.coord, scope_after_table).await?; // TODO: err msg
				tree.children.push(c);
			}
		}
	}

	Ok(tree)
}

// TODO: write test with examples from
//  https://maven.apache.org/guides/introduction/introduction-to-the-pom.html
//  https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html
//  failure case: wrong model_version, parent without pom as packaging, cyclic dependencies

#[cfg(test)]
mod testing {
	use pretty_assertions::assert_eq;
	use std::collections::HashMap;
	use std::future::Future;
	use anyhow::{Context, Result};
	use crate::{Downloader, FoundDependency, get_dependencies_tree, MavenCoord, Resolver, DependencyScope};
	use crate::maven_pom::{Dependencies, Dependency, MavenPom};

	impl Downloader for HashMap<&'static str, MavenPom> {
		// note: can't rewrite with async, bc of `+ Send`
		#[allow(clippy::manual_async_fn)]
		fn get_maven_pom(&self, url: &str) -> impl Future<Output=Result<Option<MavenPom>>> + Send {
			async { Ok(self.get(url).cloned()) }
		}
	}

	impl Downloader for HashMap<&'static str, &'static str> {
		// note: can't rewrite with async, bc of `+ Send`
		#[allow(clippy::manual_async_fn)]
		fn get_maven_pom(&self, url: &str) -> impl Future<Output=Result<Option<MavenPom>>> + Send {
			async { self.get(url).map(|xml| serde_xml_rs::from_str(xml).context("maven pom")).transpose() }
		}
	}

	#[tokio::test]
	async fn simple_pom() -> Result<()> {
		const EXAMPLE_DOT_ORG: Resolver = Resolver::new("Example dot org", "invalid://maven.example.org");
		const EXAMPLE_DOT_COM: Resolver = Resolver::new("Example dot com", "invalid://maven.example.com/foo/");
		let resolvers = [ EXAMPLE_DOT_ORG.clone(), EXAMPLE_DOT_COM.clone() ];

// TODO: write a test for taking stuff from parent for examples from https://maven.apache.org/guides/introduction/introduction-to-the-pom.html#example-1

		let map = HashMap::from([
			("invalid://maven.example.org/org/example/foo/0.1/foo-0.1.pom", MavenPom {
				model_version: "4.0.0".to_string(),
				parent: None,
				group_id: Some("org.example".to_string()),
				artifact_id: "foo".to_string(),
				version: Some("0.1".to_string()),
				packaging: None,
				dependencies: Some(Dependencies {
					dependency: vec![
						Dependency {
							group_id: "com.example".to_string(),
							artifact_id: "bar".to_string(),
							version: Some("0.2".to_string()),
							type_: None,
							classifier: Some("extra".to_string()),
							scope: None,
							optional: None,
						}
					],
				}),
				dependency_management: None,
			}),
			("invalid://maven.example.com/foo/com/example/bar/0.2/bar-0.2.pom", MavenPom {
				model_version: "4.0.0".to_string(),
				parent: None,
				group_id: Some("com.example".to_string()),
				artifact_id: "bar".to_string(),
				version: Some("0.2".to_string()),
				packaging: None,
				dependencies: None,
				dependency_management: None,
			}),
		]);

		// TODO: tests!

		let map = HashMap::from([
			("invalid://maven.example.org/org/example/foo/0.1/foo-0.1.pom", "<project>
				<modelVersion>4.0.0</modelVersion>
				<groupId>org.example</groupId>
				<artifactId>foo</artifactId>
				<version>0.1</version>
				<dependencies>
					<dependency>
						<groupId>com.example</groupId>
						<artifactId>bar</artifactId>
						<version>0.2</version>
						<classifier>extra</classifier>
					</dependency>
				</dependencies>
			</project>"),
			("invalid://maven.example.com/foo/com/example/bar/0.2/bar-0.2.pom", "<project>
				<modelVersion>4.0.0</modelVersion>
				<groupId>com.example</groupId>
				<artifactId>bar</artifactId>
				<version>0.2</version>
			</project>"),
		]);

		let wanted = MavenCoord::from_group_artifact_version("org.example", "foo", "0.1");

		let x = get_dependencies_tree(&map, &resolvers, &wanted, DependencyScope::Runtime).await?;

		let dependencies = x.into_breadth_first().collect::<Vec<_>>();

		assert_eq!(dependencies, [
			FoundDependency {
				resolver: EXAMPLE_DOT_ORG.clone(),
				coord: MavenCoord {
					group: "org.example".to_string(),
					artifact: "foo".to_string(),
					version: "0.1".to_string(),
					classifier: None,
					type_: "jar".to_string(),
				},
				scope: DependencyScope::Runtime,
			},
			FoundDependency {
				resolver: EXAMPLE_DOT_COM.clone(),
				coord: MavenCoord {
					group: "com.example".to_string(),
					artifact: "bar".to_string(),
					version: "0.2".to_string(),
					classifier: Some("extra".to_string()),
					type_: "jar".to_string(),
				},
				scope: DependencyScope::Runtime,
			},
		]);

		Ok(())
	}
}


#[cfg(test)]
mod testing2 {
	use pretty_assertions::assert_eq;
	use crate::coord::MavenCoord;
	use crate::{clean_up_dependencies, FoundDependency, DependencyScope};
	use crate::resolver::Resolver;
	use crate::tree::helper::{l, t};

	fn dep(artifact: &str, version: &str) -> FoundDependency<'static> {
		FoundDependency {
			resolver: Resolver::new("foo", "bar"),
			coord: MavenCoord::from_group_artifact_version("org.example", artifact, version),
			scope: DependencyScope::Compile,
		}
	}

	// example from https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html#transitive-dependencies
	#[test]
	fn mediation_example() {
		// A is the root node, we don't have this here
		let input = vec![
			t(dep("B", "1"), [
				t(dep("C", "1"), [
					l(dep("D", "2.0")),
				]),
			]),
			t(dep("E", "1"), [
				l(dep("D", "1.0")),
			]),
		];

		let expected = vec![
			t(dep("B", "1"), [
				l(dep("C", "1")),
			]),
			t(dep("E", "1"), [
				l(dep("D", "1.0")),
			]),
		];

		assert_eq!(clean_up_dependencies(input), expected);
	}

	// example from https://maven.apache.org/guides/introduction/introduction-to-dependency-mechanism.html#transitive-dependencies
	#[test]
	fn mediation_example_override() {
		// A is the root node, we don't have this here
		let input = vec![
			t(dep("B", "1"), [
				t(dep("C", "1"), [
					l(dep("D", "2.0")),
				]),
			]),
			t(dep("E", "1"), [
				l(dep("D", "1.0")),
			]),
			l(dep("D", "2.0")),
		];

		let expected = vec![
			t(dep("B", "1"), [
				l(dep("C", "1")),
			]),
			l(dep("E", "1")),
			l(dep("D", "2.0")),
		];

		assert_eq!(clean_up_dependencies(input), expected);
	}

	#[test]
	fn mediation_first_on_same_depth_wins() {
		// A is the root node, we don't have this here
		let input = vec![
			t(dep("B", "1"), [
				l(dep("C", "1.0")),
			]),
			t(dep("D", "1"), [
				l(dep("C", "2.0")),
			]),
		];

		let expected = vec![
			t(dep("B", "1"), [
				l(dep("C", "1.0")),
			]),
			l(dep("D", "1")),
		];

		assert_eq!(clean_up_dependencies(input), expected);
	}
}


use anyhow::{anyhow, bail, Context, Result};
use crate::coord::{MavenCoord, Types};
use crate::maven_pom::{Dependencies, DependencyManagement, MavenPom};
use crate::{Downloader, DependencyScope};
use crate::resolver::{Resolver, try_get_pom_for};

/// A maven pom after inheriting from the parent.
#[derive(Debug, Clone)]
pub(crate) struct MavenPomDone {
	pub(crate) coord: MavenCoord,

	pub(crate) dependency_management: Vec<DependencyDone>,
	pub(crate) dependencies: Vec<DependencyDone>,
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyDone {
	// TODO: version could be a range...
	pub(crate) coord: MavenCoord,

	pub(crate) scope: Option<DependencyScope>,
	pub(crate) optional: Option<bool>,
}

/// Gets an "effective pom".
///
/// See also the maven goal `help:effective-pom` (which can be run with `mvn help:effective-pom`).
///
/// This includes inheriting from the parent poms and resolving `<scope>import</scope>` in `<dependencyManagement>`.
#[async_recursion::async_recursion]
pub(crate) async fn get_merged_pom<'a>(downloader: &(impl Downloader + Sync), resolvers: &'a [Resolver], coord: &MavenCoord)
		-> Result<(&'a Resolver<'a>, MavenPomDone)> {

	let (resolver, pom) = try_get_pom_for(downloader, resolvers, coord).await?;

	let mut poms_stack = Vec::new();

	let mut to_get = pom.get_parent_coord();
	while let Some(coord) = to_get.take() {
		let (_, pom) = try_get_pom_for(downloader, resolvers, &coord).await?;

		to_get = pom.get_parent_coord();

		poms_stack.push(pom);
	}

	let mut parent = None;
	for pom in poms_stack.into_iter().rev() {

		let merged = merge_parent(downloader, resolvers, parent, pom).await
			.with_context(|| anyhow!("merging parent and child for child from {resolver:?} and {coord}"))?;

		parent = Some(merged);
	}

	let merged = merge_parent(downloader, resolvers, parent, pom).await
		.with_context(|| anyhow!("merging parent and child for child from {resolver:?} and {coord}"))?;

	Ok((resolver, merged))
}


#[async_recursion::async_recursion]
async fn merge_parent(downloader: &(impl Downloader + Sync), resolvers: &[Resolver], parent: Option<MavenPomDone>, child: MavenPom)
		-> Result<MavenPomDone> {
	if let Some(parent) = parent {
		let coord = MavenCoord {
			group: child.group_id.unwrap_or(parent.coord.group),
			artifact: child.artifact_id,
			version: child.version.unwrap_or(parent.coord.version),
			// TODO: check this one
			classifier: None,
			type_: {
				if parent.coord.type_ != "pom" {
					bail!("parents `type` (determined from `packaging`) must be `pom`, got {:?}", parent.coord.type_);
				}
				Types::packaging_to_type(child.packaging.as_deref().unwrap_or("jar")).to_owned()
			},
		};

		let dependency_management = make_dependency_management(downloader, resolvers,
			child.dependency_management, Some(parent.dependency_management)
		).await
			.with_context(|| anyhow!("while creating `dependency_management` for {coord} (with a real parent)"))?;

		let dependencies = make_dependencies(
			&dependency_management, child.dependencies, Some(parent.dependencies)
		)
			.with_context(|| anyhow!("while creating `dependencies` for {coord} (with a real parent)"))?;

		Ok(MavenPomDone { coord, dependency_management, dependencies })
	} else {
		// inherit from super pom from https://maven.apache.org/ref/3.9.8/maven-model-builder/super-pom.html

		let coord = MavenCoord {
			group: child.group_id.context("no group id for pom (parent is super pom)")?,
			artifact: child.artifact_id,
			version: child.version.context("no version for pom (parent is super pom)")?,
			// TODO: check this one
			classifier: None,
			type_: Types::packaging_to_type(child.packaging.as_deref().unwrap_or("jar")).to_owned(),
		};

		let dependency_management = make_dependency_management(downloader, resolvers,
			child.dependency_management, None
		).await
			.with_context(|| anyhow!("while creating `dependency_management` for {coord} (parent is super pom)"))?;

		let dependencies = make_dependencies(
			&dependency_management,
			child.dependencies,
			None
		)
			.with_context(|| anyhow!("while creating `dependencies` for {coord} (parent is super pom)"))?;

		Ok(MavenPomDone { coord, dependency_management, dependencies })
	}
}


#[async_recursion::async_recursion]
async fn make_dependency_management(downloader: &(impl Downloader + Sync), resolvers: &[Resolver],
	child_dependency_management: Option<DependencyManagement>,
	parent_dependency_management: Option<Vec<DependencyDone>>,
) -> Result<Vec<DependencyDone>> {
	// TODO: ok so merging both dependencies and dependency_management works like this:
	//  we take the set of {group_id, artifact_id, type, classifier} and try to get from parent
	//  if we get something there, use the override trick, if we get nothing, don't error out instead just take
	//  our current values. after that append all the ones from the parent that we didn't get before
	let mut vec = Vec::new();

	for x in child_dependency_management.and_then(|x| x.dependencies).map_or_else(Vec::new, |x| x.dependency) {

		let group = x.group_id;
		let artifact = x.artifact_id;
		let version = x.version.with_context(|| anyhow!("no version for dependency {group} {artifact}"))?;
		let type_ = x.type_.unwrap_or_else(|| String::from("jar"));
		let classifier = x.classifier.or_else(|| Types::type_to_classifier(&type_).map(|x| x.to_owned()));
		let coord = MavenCoord { group, artifact, version, classifier, type_ };

		let optional = x.optional;

		let scope = match x.scope.map(|x| x.into_scope()) {
			None => None,
			Some(Some(scope)) => Some(scope),
			Some(None) => { // import scope
				// TODO: put in a recursion limiter!
				let target_pom: MavenPomDone = get_merged_pom(downloader, resolvers, &coord).await?.1;

				vec.extend(target_pom.dependency_management);

				// don't add the scope=import dependency itself
				continue;
			},
		};

		let that = DependencyDone { coord, scope, optional };

		vec.push(that);
	}

	if let Some(parent_dependency_management) = parent_dependency_management {
		// merge with parent
		vec.extend(parent_dependency_management);
	}

	Ok(vec)
}

fn make_dependencies(
	dependency_management: &[DependencyDone],
	child_dependencies: Option<Dependencies<DependencyScope>>,
	parent_dependencies: Option<Vec<DependencyDone>>,
) -> Result<Vec<DependencyDone>> {
	let parent_dependencies = parent_dependencies.unwrap_or_default();

	child_dependencies.map_or_else(Vec::new, |x| x.dependency).into_iter()
		.map(|x| {
			let group = x.group_id;
			let artifact = x.artifact_id;
			let type_ = x.type_.unwrap_or_else(|| String::from("jar"));
			let classifier = x.classifier.or_else(|| Types::type_to_classifier(&type_).map(|x| x.to_owned()));

			// try to find a dependency from dependency_management that matches and inherit from it
			if let Some(result) = dependency_management.iter()
				.find(|i| i.coord.matches_besides_version(&group, &artifact, &classifier, &type_))
			{
				// overwrite the `version` from the dependency we inherit from
				let version = x.version.unwrap_or_else(|| result.coord.version.clone());

				Ok(DependencyDone {
					coord: MavenCoord { group, artifact, version, classifier, type_ },
					scope: x.scope.or(result.scope), // allow overrides
					optional: x.optional.or(result.optional),
				})
			} else {
				// if we have a version, we have a full dependency and can take that
				if let Some(version) = x.version {
					Ok(DependencyDone {
						coord: MavenCoord { group, artifact, version, classifier, type_ },
						scope: x.scope,
						optional: x.optional,
					})
				} else {
					bail!("no dependency found matching {group:?} {artifact:?} with classifier {classifier:?} and type {type_:?}")
				}
			}
		})
		// TODO: also properly merge with parent? (appending the parent deps directly is wrong)
		.chain(parent_dependencies.into_iter().map(Ok))
		.collect::<Result<_>>()
}





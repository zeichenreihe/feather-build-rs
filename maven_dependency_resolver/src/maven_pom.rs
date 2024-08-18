use serde::{Deserialize, Serialize};
use crate::DependencyScope;

/// The corresponding struct to the `.pom` xml file.
///
/// See https://maven.apache.org/xsd/maven-4.0.0.xsd and
/// https://github.com/apache/maven/blob/c0012c08aaad27473770fc39ab7e39026238c7e1/api/maven-api-model/src/main/mdo/maven.mdo
/// for the specification of these fields
///
/// Some fields an `Option<bool>` (or similar) even tho the default value is `false` (or similar). This is necessary because
/// without this we wouldn't know if a value is explicitly overwritten or just the default. We couldn't distinguish between
/// a default value of `false` and an explicit overwrite of a `true` with a `false`. (In the first case we'd want the effective
/// value to be `true`, since that's inherited, and in the second case we'd want `false`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MavenPom {
	#[serde(rename = "modelVersion")]
	pub(crate) model_version: String,
	pub(crate) parent: Option<Parent>,

	#[serde(rename = "groupId")]
	pub(crate) group_id: Option<String>,
	#[serde(rename = "artifactId")]
	pub(crate) artifact_id: String,
	pub(crate) version: Option<String>,

	/// This doesn't get inherited
	pub(crate) packaging: Option<String>,

	#[serde(rename = "dependencyManagement")]
	pub(crate) dependency_management: Option<DependencyManagement>,
	pub(crate) dependencies: Option<Dependencies<DependencyScope>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Parent {
	#[serde(rename = "groupId")]
	pub(crate) group_id: String,
	#[serde(rename = "artifactId")]
	pub(crate) artifact_id: String,
	pub(crate) version: String,
	// TODO: relativePath
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct DependencyManagement {
	pub(crate) dependencies: Option<Dependencies<ManagementScope>>,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Dependencies<Scope> {
	pub(crate) dependency: Vec<Dependency<Scope>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Dependency<Scope> {
	#[serde(rename = "groupId")]
	pub(crate) group_id: String,
	#[serde(rename = "artifactId")]
	pub(crate) artifact_id: String,
	// TODO: this could be a range of versions
	pub(crate) version: Option<String>,
	#[serde(rename = "type")]
	pub(crate) type_: Option<String>,
	pub(crate) classifier: Option<String>,

	pub(crate) scope: Option<Scope>,

	// TODO: systemPath
	// TODO: exclusions

	pub(crate) optional: Option<bool>,
}


/// A copy of [DependencyScope], but with [ManagementScope::Import].
#[derive(Copy, Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub(crate) enum ManagementScope {
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

	/// Only on dependencies of type `pom` in a `dependency_management` block
	#[serde(rename = "import")]
	Import,
}

impl ManagementScope {
	/// Returns `None` for [ManagementScope::Import].
	pub(crate) fn into_scope(self) -> Option<DependencyScope> {
		match self {
			ManagementScope::Compile => Some(DependencyScope::Compile),
			ManagementScope::Runtime => Some(DependencyScope::Runtime),
			ManagementScope::Test => Some(DependencyScope::Test),
			ManagementScope::System => Some(DependencyScope::System),
			ManagementScope::Provided => Some(DependencyScope::Provided),
			ManagementScope::Import => None
		}
	}
}

use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use anyhow::{anyhow, bail, Context, Error};
use log::warn;
use crate::resolver::Resolver;

/// A `group_id`, `artifact_id`, `version`, `classifier` and `type` together.
///
/// Also known as an artifact coordinate, as described [here](https://maven.apache.org/repositories/artifacts.html).
///
/// [MavenCoord] implements [FromStr]. Format is: `group:artifact[:type[:classifier]]:version`.
// Note: this format is in some error message from gradle, probably maven as well, see
// https://stackoverflow.com/questions/25527235/how-do-i-resolve-this-buildconfig-error-in-grails
// The original also uses "extension" instead of type.
// TODO: switch to `extension` again? or both?
/// ```
/// use std::str::FromStr;
/// # use pretty_assertions::assert_eq;
/// use maven_dependency_resolver::coord::MavenCoord;
/// let a = MavenCoord::from_str("org.example:artifact:war:sources:1.0").unwrap();
/// let b = MavenCoord {
///     group: "org.example".to_owned(),
///     artifact: "artifact".to_owned(),
///     version: "1.0".to_owned(),
///     classifier: Some("sources".to_owned()),
///     type_: "war".to_owned(),
/// };
///
/// assert_eq!(a, b);
/// ```
/// [MavenCoord] also implements [Display], which exactly produces a format parsed by [FromStr], always listening the type.
/// ```
/// use std::str::FromStr;
/// # use pretty_assertions::assert_eq;
/// use maven_dependency_resolver::coord::MavenCoord;
///
/// let a = "org.example:artifact:1.0"; // notice: no type, no classifier
/// let b = "org.example:artifact:jar:1.0"; // equivalent, since default type is "jar"
/// assert_eq!(b, format!("{}", MavenCoord::from_str(a).unwrap()));
///
/// let a = "org.example:artifact:ear:1.0"; // notice: no classifier, but type
/// assert_eq!(a, format!("{}", MavenCoord::from_str(a).unwrap()));
///
/// let a = "org.example:artifact:ear:javadoc:1.0";
/// assert_eq!(a, format!("{}", MavenCoord::from_str(a).unwrap()));
/// ```
/// This means that round trips with [Display] and [FromStr] are possible.
#[derive(Debug, Clone, PartialEq)]
pub struct MavenCoord {
	pub group: String,
	pub artifact: String,
	pub version: String,
	pub classifier: Option<String>,
	pub type_: String,
}

impl MavenCoord {
	/// Takes a `group`, `artifact` and version and constructs one, with no classifier and type `jar`.
	///
	/// This is the default for any artifact that doesn't have a custom type or custom dependency.
	pub fn from_group_artifact_version(group: &str, artifact: &str, version: &str) -> MavenCoord {
		MavenCoord {
			group: group.to_owned(),
			artifact: artifact.to_owned(),
			version: version.to_owned(),
			classifier: None,
			type_: "jar".to_owned(),
		}
	}

	pub(crate) fn make_url(&self, resolver: &Resolver) -> String {
		format!("{maven}{maven_slash}{group}/{artifact}/{base_version}/{artifact}-{version}{classifier_minus}{classifier}.{extension}",
			maven = resolver.maven,
			maven_slash = if resolver.maven.ends_with('/') { "" } else { "/" },
			group = self.group.replace('.', "/"),
			artifact = self.artifact,
			base_version = self.base_version(),
			version = self.version,
			classifier_minus = if self.classifier.is_some() { "-" } else { "" },
			classifier = self.classifier.as_deref().unwrap_or(""),
			extension = Types::type_to_extension(&self.type_),
		)
	}

	/// Creates the url corresponding to the `.pom` of this artifact.
	pub(crate) fn make_pom_url(&self, resolver: &Resolver) -> String {
		format!("{maven}{maven_slash}{group}/{artifact}/{base_version}/{artifact}-{version}.pom",
			maven = resolver.maven,
			maven_slash = if resolver.maven.ends_with('/') { "" } else { "/" },
			group = self.group.replace('.', "/"),
			artifact = self.artifact,
			base_version = self.base_version(),
			version = self.version,
		)
	}

	pub(crate) fn base_version(&self) -> Cow<str> {
		to_snapshot_version(&self.version)
	}

	pub(crate) fn matches_besides_version(&self, group: &str, artifact: &str, classifier: &Option<String>, type_: &str) -> bool {
		self.group == group && self.artifact == artifact && &self.classifier == classifier && self.type_ == type_
	}

	pub(crate) fn dependency_collision_id(&self) -> DependencyCollisionId {
		DependencyCollisionId {
			group: self.group.clone(),
			artifact: self.artifact.clone(),
			classifier: self.classifier.clone(),
			type_: self.type_.clone(),
		}
	}
}

impl Display for MavenCoord {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{group}:{artifact}:{type_}{classifier_colon}{classifier}:{version}",
			group = self.group,
			artifact = self.artifact,
			type_ = self.type_,
			classifier_colon = if self.classifier.is_some() { ":" } else { "" },
			classifier = self.classifier.as_deref().unwrap_or(""),
			version = self.version,
		)
	}
}

impl FromStr for MavenCoord {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut iter = s.split(':');
		let group = iter.next().with_context(|| anyhow!("no group specified: {s:?}"))?;
		let artifact = iter.next().with_context(|| anyhow!("no artifact specified: {s:?}"))?;

		let type_or_version = iter.next().with_context(|| anyhow!("no version specified: {s:?}"))?;

		let (type_, classifier, version) = if let Some(classifier_or_version) = iter.next() {
			if let Some(version) = iter.next() {
				if iter.next().is_some() {
					bail!("there may not be more than 4 colons: {s:?}");
				}

				(Some(type_or_version), Some(classifier_or_version), version)
			} else {
				(Some(type_or_version), None, classifier_or_version)
			}
		} else {
			(None, None, type_or_version)
		};

		Ok(MavenCoord {
			group: group.to_owned(),
			artifact: artifact.to_owned(),
			version: version.to_owned(),
			classifier: classifier.map(|x| x.to_owned()),
			type_: type_.unwrap_or("jar").to_owned(),
		})
	}
	// TODO: add tests for this implementation (also failure cases)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DependencyCollisionId {
	group: String,
	artifact: String,
	classifier: Option<String>,
	type_: String,
}

fn to_snapshot_version(version: &str) -> Cow<str> {
	// the pattern is: ^(.*)-(\d{8}.\d{6})-(\d+)$
	version.rsplit_once('-')
		.filter(|(_, after_last_hyphen)| !after_last_hyphen.is_empty() && after_last_hyphen.chars().all(|x| x.is_ascii_digit()))
		.and_then(|(before_last_hyphen, _)| before_last_hyphen.rsplit_once('-'))
		.filter(|(_, between_hyphens)| {
			between_hyphens.split_once('.')
				.is_some_and(|(date, time)|
					date.len() == 8 && date.chars().all(|x| x.is_ascii_digit()) &&
					time.len() == 6 && time.chars().all(|x| x.is_ascii_digit())
				)
		})
		.map_or_else(|| Cow::Borrowed(version), |(before_prev_hyphen, _)| Cow::Owned(format!("{before_prev_hyphen}-SNAPSHOT")))
}

pub(crate) struct Types;

// - see https://maven.apache.org/ref/3.9.8/maven-core/artifact-handlers.html for the default handlers table
// - see https://github.com/apache/felix-dev/blob/bcf435d69ab0ab8e6a9f303fc3092e7731b9a1ea/tools/maven-bundle-plugin/src/main/resources/META-INF/plexus/components.xml
//   for how type "bundle" has extension "jar" and packaging "bundle"
impl Types {
	/// The value of the classifier column, where `None` means empty.
	pub(crate) fn type_to_classifier(type_: &str) -> Option<&str> {
		match type_ {
			// default handlers
			"pom" | "jar" | "maven-plugin" | "ejb" | "war" | "ear" | "rar" => None,
			"test-jar" => Some("tests"),
			"ejb-client" => Some("client"),
			"java-source" => Some("sources"),
			"javadoc" => Some("javadoc"),
			// cases by common plugins
			"bundle" => None,
			// any other case
			type_ => {
				warn!("unknown artifact type {type_:?}, assuming it has no default classifier (please inform the devs that his is unknown so thy can add it)");
				None
			},
		}
	}

	/// The value of the extension column.
	pub(crate) fn type_to_extension(type_: &str) -> &str {
		match type_ {
			// default handlers
			extension @ ("pom" | "jar" | "war" | "ear" | "rar") => extension,
			"test-jar" | "maven-plugin" | "ejb" | "ejb-client" | "java-source" | "javadoc" => "jar",
			// cases by common plugins
			"bundle" => "jar",
			// any other case
			extension => {
				warn!("unknown artifact type {extension:?}, taking it as extension (please inform the devs that this is unknown so they can add it)");
				extension
			},
		}
	}

	/// From the packaging column to the type column.
	pub(crate) fn packaging_to_type(packaging: &str) -> &str {
		match packaging {
			// default handlers
			type_ @ ("pom" | "jar" | "maven-plugin" | "ejb" | "war" | "ear" | "rar" | "java-source" | "javadoc") => type_,
			// default handlers that are non-reversible, so can't appear in `packaging`, since the packaging is used twice
			#[allow(unreachable_patterns)]
			"jar" => "test-jar",
			#[allow(unreachable_patterns)]
			"ejb" => "ejb-client",
			// cases by common plugins
			"bundle" => "bundle",
			// any other case
			type_ => {
				warn!("unknown packaging {type_:?}, taking it as type (please inform the devs that this is unknown so they can add it)");
				type_
			},
		}
	}

	/// The value of the "language" column.
	pub(crate) fn type_to_language(type_: &str) -> &str {
		match type_ {
			// default handlers
			"pom" => "none",
			"jar" | "test-jar" | "maven-plugin" | "ejb" | "ejb-client" | "war" | "ear" | "rar" | "java-source" | "javadoc" => "java",
			// cases by common plugins
			"bundle" => "java",
			// any other case
			type_ => {
				warn!("unknown artifact type {type_:?}, returning language \"none\" (please inform the devs that this is unknown so they can add it)");
				"none"
			},
		}
	}

	/// The value of the "added to classpath" column, assuming that empty means `false`.
	pub(crate) fn type_to_added_to_classpath(type_: &str) -> bool {
		match type_ {
			// default handlers
			"pom" | "war" | "ear" | "rar" | "java-source" => false,
			"jar" | "test-jar" | "maven-plugin" | "ejb" | "ejb-client" | "javadoc" => true,
			// cases by common plugins
			"bundle" => true,
			// any other case
			type_ => {
				warn!("unknown artifact type {type_:?}, returning `true` for \"added to classpath\" (please inform the devs that this is unknown so they can add it)");
				true
			},
		}
	}

	/// The value of the "includesDependencies" column, assuming that empty means `false`.
	pub(crate) fn type_to_includes_dependencies(type_: &str) -> bool {
		match type_ {
			// default handlers
			"pom" | "jar" | "test-jar" | "maven-plugin" | "ejb" | "ejb-client" | "java-source" | "javadoc" => false,
			"war" | "ear" | "rar" => true,
			// cases by common plugins
			"bundle" => false,
			// any other case
			type_ => {
				warn!("unknown artifact type {type_:?}, returning `false` for includes_dependencies (please inform the devs that this is unknown so they can add it)");
				false
			},
		}
	}
}

#[cfg(test)]
mod testing {
	use pretty_assertions::assert_eq;
	use crate::coord::to_snapshot_version;

	#[test]
	fn test_to_snapshot_version() {
		assert_eq!(to_snapshot_version("vineflower-1.10.0"), "vineflower-1.10.0");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20230713.025619-1"), "vineflower-1.10.0-SNAPSHOT");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20230909.205406-28"), "vineflower-1.10.0-SNAPSHOT");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20230909.205406-282828123456790"), "vineflower-1.10.0-SNAPSHOT");

		// failures of the pattern: ^(.*)-(\d{8}.\d{6})-(\d+)$
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20x30909.205406-28"), "vineflower-1.10.0-20x30909.205406-28");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20230909.20x406-28"), "vineflower-1.10.0-20230909.20x406-28");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20230909.205406-2x"), "vineflower-1.10.0-20230909.205406-2x");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20230909.205406x28"), "vineflower-1.10.0-20230909.205406x28");
		assert_eq!(to_snapshot_version("vineflower-1.10.0x20230909.205406x28"), "vineflower-1.10.0x20230909.205406x28");
		assert_eq!(to_snapshot_version("vineflower21.10.0x20230909.205406x28"), "vineflower21.10.0x20230909.205406x28");
		assert_eq!(to_snapshot_version("vineflower21.10.0-202309090205406028"), "vineflower21.10.0-202309090205406028");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-202309090205406028"), "vineflower-1.10.0-202309090205406028");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-20230713.025619-"), "vineflower-1.10.0-20230713.025619-");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-2023071.3025619-1"), "vineflower-1.10.0-2023071.3025619-1");
		assert_eq!(to_snapshot_version("vineflower-1.10.0-202307130.25619-1"), "vineflower-1.10.0-202307130.25619-1");
	}
}
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct MavenMetadata {
	#[serde(rename = "groupId")]
	pub(crate) group_id: String,
	#[serde(rename = "artifactId")]
	pub(crate) artifact_id: String,
	pub(crate) versioning: Versioning,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Versioning {
	pub(crate) latest: String,
	pub(crate) release: String,
	pub(crate) versions: Versions,
	#[serde(rename = "lastUpdated")]
	pub(crate) last_updated: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Versions {
	#[serde(rename = "version")]
	pub(crate) versions: Vec<String>,
}

use serde::{Deserialize, Serialize};
use crate::version_graph::Version;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VersionsManifest {
	#[serde(rename = "$schema")]
	pub(crate) schema: String,
	pub(crate) latest: Latest,
	pub(crate) versions: Vec<VersionInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Latest {
	pub(crate) old_alpha: Version,
	pub(crate) classic_server: Version,
	pub(crate) alpha_server: Version,
	pub(crate) old_beta: Version,
	pub(crate) snapshot: Version,
	pub(crate) release: Version,
	pub(crate) pending: Version,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VersionInfo {
	pub(crate) id: Version,
	#[serde(rename = "type")]
	pub(crate) version_type: VersionType,
	pub(crate) url: String,
	pub(crate) time: Option<String>,
	#[serde(rename = "releaseTime")]
	pub(crate) release_time: String,
	pub(crate) details: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VersionType(String); // TODO: enum of ???

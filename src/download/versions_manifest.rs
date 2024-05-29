use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VersionsManifest {
	#[serde(rename = "$schema")]
	pub(crate) schema: String,
	pub(crate) latest: Latest,
	pub(crate) versions: Vec<VersionInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Latest {
	pub(crate) old_alpha: MinecraftVersion,
	pub(crate) classic_server: MinecraftVersion,
	pub(crate) alpha_server: MinecraftVersion,
	pub(crate) old_beta: MinecraftVersion,
	pub(crate) snapshot: MinecraftVersion,
	pub(crate) release: MinecraftVersion,
	pub(crate) pending: MinecraftVersion,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq, Deserialize, Serialize)]
pub(crate) struct MinecraftVersion(pub(crate) String);

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VersionInfo {
	pub(crate) id: MinecraftVersion,
	#[serde(rename = "type")]
	pub(crate) version_type: VersionType,
	pub(crate) url: String,
	pub(crate) time: Option<String>,
	#[serde(rename = "releaseTime")]
	pub(crate) release_time: String,
	pub(crate) details: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum VersionType {
	#[serde(rename = "release")]
	Release,
	#[serde(rename = "snapshot")]
	Snapshot,
	#[serde(rename = "pending")]
	Pending,
	#[serde(rename = "old_beta")]
	OldBeta,
	#[serde(rename = "old_alpha")]
	OldAlpha,
	#[serde(rename = "alpha_server")]
	AlphaServer,
	#[serde(rename = "classic_server")]
	ClassicServer,
}

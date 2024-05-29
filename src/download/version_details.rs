use serde::{Deserialize, Serialize};
use crate::download::versions_manifest::MinecraftVersion;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VersionDetails {
	pub(crate) id: MinecraftVersion,
	pub(crate) client: bool,
	pub(crate) server: bool,
	#[serde(rename = "sharedMappings")]
	pub(crate) shared_mappings: bool,
	pub(crate) downloads: DownloadsInfo,
	pub(crate) libraries: Vec<String>,
	pub(crate) manifests: Vec<ManifestInfo>,
	#[serde(rename = "normalizedVersion")]
	pub(crate) normalised_version: String,
	pub(crate) previous: Vec<String>,
	pub(crate) next: Vec<String>,
	#[serde(rename = "releaseTarget")]
	pub(crate) release_target: String,
	#[serde(rename = "releaseTime")]
	pub(crate) release_time: String,
	pub(crate) protocol: ProtocolInfo,
	pub(crate) world: WorldInfo,
}


#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct DownloadsInfo {
	pub(crate) client: DownloadInfo,
	pub(crate) server: DownloadInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct DownloadInfo {
	pub(crate) sha1: String,
	pub(crate) size: usize,
	pub(crate) url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ManifestInfo {
	#[serde(rename = "assetHash")]
	pub(crate) asset_hash: String,
	#[serde(rename = "assetIndex")]
	pub(crate) asset_index: String,
	pub(crate) downloads: String,
	#[serde(rename = "downloadsId")]
	pub(crate) downloads_id: usize,
	pub(crate) hash: String,
	pub(crate) time: String,
	#[serde(rename = "type")]
	pub(crate) release_type: ReleaseType,
	pub(crate) url: String,
}


#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum ReleaseType {
	#[serde(rename = "release")]
	Release,
	#[serde(rename = "snapshot")]
	Snapshot,
	#[serde(rename = "old_alpha")]
	OldAlpha,
	#[serde(rename = "old_beta")]
	OldBeta,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ProtocolInfo {
	#[serde(rename = "type")]
	pub(crate) protocol_type: ProtocolType,
	pub(crate) version: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum ProtocolType {
	#[serde(rename = "classic")]
	Classic,
	#[serde(rename = "modern")]
	Modern,
	#[serde(rename = "netty")]
	Netty,
	#[serde(rename = "netty-snapshot")]
	NettySnapshot,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct WorldInfo {
	pub(crate) format: WorldFormat,
	pub(crate) version: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum WorldFormat {
	#[serde(rename = "alpha")]
	Alpha,
	#[serde(rename = "region")]
	Region,
	#[serde(rename = "anvil")]
	Anvil,
}
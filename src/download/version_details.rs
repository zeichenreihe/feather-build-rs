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
	pub(crate) manifests: Option<Vec<ManifestInfo>>, // TODO: missing in https://ornithemc.net/mc-versions/version/1.4.6.json
	#[serde(rename = "normalizedVersion")]
	pub(crate) normalised_version: String,
	pub(crate) previous: Vec<String>,
	pub(crate) next: Vec<String>,
	#[serde(rename = "releaseTarget")]
	pub(crate) release_target: Option<String>, // TODO: missing in https://skyrising.github.io/mc-versions/version/b1.5_01.json (url by now outdated)
	#[serde(rename = "releaseTime")]
	pub(crate) release_time: String,
	pub(crate) protocol: ProtocolInfo,
	pub(crate) world: WorldInfo,
}


#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct DownloadsInfo {
	pub(crate) client: Option<DownloadInfo>, // TODO: missing in https://skyrising.github.io/mc-versions/version/b1.5_02.json (url by now outdated)
	pub(crate) server: Option<DownloadInfo>, // TODO: missing in https://skyrising.github.io/mc-versions/version/b1.3_01.json (url by now outdated)
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
	pub(crate) asset_hash: Option<String>,// TODO: missing in https://skyrising.github.io/mc-versions/version/b1.5_01.json (url by now outdated)
	#[serde(rename = "assetIndex")]
	pub(crate) asset_index: Option<String>, // TODO: same
	pub(crate) downloads: String,
	#[serde(rename = "downloadsId")]
	pub(crate) downloads_id: usize,
	pub(crate) hash: String,
	pub(crate) time: Option<String>, // TODO: missing in https://skyrising.github.io/mc-versions/version/b1.5_01.json (url by now outdated)
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
	// TODO: For 1.8, this is None. 1.8 uses anvil. Investigate this further. (url by now outdated)
	pub(crate) version: Option<usize>,
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
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct VersionManifest {
	pub(crate) libraries: Vec<Library>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Library {
	pub(crate) name: String,
	pub(crate) downloads: LibraryDownloads,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct LibraryDownloads {
	pub(crate) artifact: Option<LibraryDownloadArtifact>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct LibraryDownloadArtifact {
	pub(crate) url: String,
}

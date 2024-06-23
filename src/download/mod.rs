use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, bail, Context, Result};
use bytes::Buf;
use reqwest::Client;
use zip::ZipArchive;
use crate::download::version_details::VersionDetails;
use crate::download::version_manifest::VersionManifest;
use crate::download::versions_manifest::VersionsManifest;
use crate::Environment;
use crate::download::maven_metadata::MavenMetadata;
use quill::tree::mappings::Mappings;
use dukebox::zip::file::FileJar;
use crate::Version;

pub(crate) mod versions_manifest;
pub(crate) mod version_manifest;
pub(crate) mod version_details;
pub(crate) mod maven_metadata;

#[derive(Debug)]
pub(crate) struct Downloader {
	client: Client,
}

struct DownloadResult<'a> {
	url: &'a str,
	path: PathBuf,
}

impl DownloadResult<'_> {
	fn parse_as_json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
		let vec = fs::read(self.path).with_context(|| anyhow!("failed to open cache file for json from {:?}", self.url))?;
		serde_json::from_slice(&vec).with_context(|| anyhow!("failed to parse json from {:?}", self.url))
	}

	fn parse_as_xml<T: serde::de::DeserializeOwned>(self) -> Result<T> {
		let body = File::open(self.path).with_context(|| anyhow!("failed to open cache file for xml from {:?}", self.url))?;
		serde_xml_rs::from_reader(body).with_context(|| anyhow!("failed to parse xml from {:?}", self.url))
	}

	fn open_as_file(self) -> Result<File> {
		File::open(self.path).with_context(|| anyhow!("failed to open cache file from {:?}", self.url))
	}

	fn into_file_jar(self) -> Result<FileJar> {
		Ok(FileJar::new(self.path))
	}
}

impl Downloader {
	pub(crate) fn new() -> Downloader {
		Downloader { client: Client::new() }
	}

	async fn download<'a>(&'a self, url: &'a str) -> Result<DownloadResult> {
		let downloads = Path::new("./download");

		let Some(url_stripped) = url.strip_prefix("https://") else {
			bail!("url doesn't start with `https://`: {url:?}");
		};

		//TODO: reevaluate possible security vulnerabilities here
		// - one thing could be something like https://evil.example.org/../../../../../../../../usr/bin/bla.jar and replaces a jar on our system
		let cache_path = downloads.join(url_stripped);

		if !cache_path.try_exists()? {
			if let Some(parent) = cache_path.parent() {
				fs::create_dir_all(parent)?;
			}

			println!("downloading {url}");

			let response = self.client.get(url).send().await?;
			if !response.status().is_success() {
				bail!("got a \"{}\" for {url:?}", response.status());
			}

			let src = response.bytes().await?;
			let mut dest = File::create(&cache_path)?;
			std::io::copy(&mut src.reader(), &mut dest)?;
		}

		Ok(DownloadResult { url, path: cache_path })
	}

	pub(crate) async fn get_jar(&self, url: &str) -> Result<FileJar> {
		self.download(url).await?
			.into_file_jar()
	}

	pub(crate) async fn get_versions_manifest(&self) -> Result<VersionsManifest> {
		self.download("https://skyrising.github.io/mc-versions/version_manifest.json").await?
			.parse_as_json().context("versions manifest")
	}

	async fn wanted_version_manifest(&self, versions_manifest: &VersionsManifest, version: &Version) -> Result<VersionManifest> {
		let minecraft_version = version.get_minecraft_version();

		let manifest_version = versions_manifest.versions.iter()
			.find(|it| it.id == minecraft_version)
			.with_context(|| anyhow!("no version data for minecraft version {version:?}"))?;

		self.download(&manifest_version.url).await?
			.parse_as_json().with_context(|| anyhow!("version manifest for version {version:?}"))
	}

	pub(crate) async fn version_details(&self, versions_manifest: &VersionsManifest, version: &Version, environment: &Environment) -> Result<VersionDetails> {
		let minecraft_version = version.get_minecraft_version();

		let manifest_version = versions_manifest.versions.iter()
			.find(|it| it.id == minecraft_version)
			.with_context(|| anyhow!("no version details for minecraft version {version:?}"))?;

		let version_details: VersionDetails = self.download(&manifest_version.details).await?
			.parse_as_json().with_context(|| anyhow!("version details for version {version:?}"))?;

		if version_details.shared_mappings {
			if &Environment::Merged != environment {
				bail!("minecraft version {:?} is only available as merged but was requested for {:?}", version, environment);
			}
		} else {
			match environment {
				Environment::Merged => {
					bail!("minecraft version {:?} cannot be merged - please select either the client or server environment!", version);
				},
				Environment::Client if !version_details.client => {
					bail!("minecraft version {:?} does not have a client jar!", version);
				},
				Environment::Server if !version_details.server => {
					bail!("minecraft version {:?} does not have a server jar", version);
				},
				_ => {},
			}
		}

		Ok(version_details)
	}

	/// Downloads the feather intermediary, calamus, for a given version.
	///
	/// The namespaces are `official` to `intermediary` (aka `calamus`) here.
	pub(crate) async fn calamus_v2(&self, version: &Version) -> Result<Mappings<2>> {
		let url = format!("https://maven.ornithemc.net/releases/net/ornithemc/calamus-intermediary/{version}/calamus-intermediary-{version}-v2.jar");

		let body = self.download(&url).await?.open_as_file()?;

		let mut zip = ZipArchive::new(body)?;

		let file = zip.by_name("mappings/mappings.tiny").with_context(|| anyhow!("cannot find mappings in zip file from {url:?}"))?;

		let mappings = quill::tiny_v2::read(file).with_context(|| anyhow!("failed to read mappings from mappings/mappings.tiny of {url:?}"))?;

		mappings.info.namespaces.check_that(["official", "intermediary"])?;

		Ok(mappings)
	}

	pub(crate) async fn mc_libs(&self, versions_manifest: &VersionsManifest, version: &Version) -> Result<Vec<FileJar>> {
		let version_file = self.wanted_version_manifest(versions_manifest, version).await?;

		let mut libs = Vec::new();

		for lib in version_file.libraries {
			if let Some(artifact) = lib.downloads.artifact {
				let jar = self.get_jar(&artifact.url).await?;

				libs.push(jar);
			}
		}

		Ok(libs)
	}

	pub(crate) async fn get_maven_metadata_xml(&self, url: &str) -> Result<MavenMetadata> {
		self.download(url).await?
			.parse_as_xml().context("maven metadata")
	}
}
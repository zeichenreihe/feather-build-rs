use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use bytes::Buf;
use reqwest::{Client, Response};
use zip::ZipArchive;
use crate::download::version_details::VersionDetails;
use crate::download::version_manifest::VersionManifest;
use crate::download::versions_manifest::VersionsManifest;
use crate::{Environment, Jar};
use crate::download::maven_metadata::MavenMetadata;
use mappings_rw::tree::mappings::Mappings;
use crate::Version;

pub(crate) mod versions_manifest;
pub(crate) mod version_manifest;
pub(crate) mod version_details;
pub(crate) mod maven_metadata;

#[derive(Debug)]
pub(crate) struct Downloader {
	versions_manifest: Option<VersionsManifest>,
	client: Client,
}

impl Downloader {
	pub(crate) fn new() -> Downloader {
		Downloader {
			versions_manifest: None,
			client: Client::new(),
		}
	}

	async fn get(&self, url: &str) -> Result<Response> {
		let response = self.client.get(url).send().await?;

		if response.status().is_success() {
			Ok(response)
		} else {
			bail!("got a \"{}\" for {url:?}", response.status());
		}
	}

	pub(crate) async fn get_jar(&mut self, url: &str) -> Result<Jar> {
		let downloads = Path::new("./download");

		let path = Path::new(url).strip_prefix(Path::new("https://"))
			.with_context(|| anyhow!("url doesn't start with `https://`: {url:?}"))?;

		let path = downloads.join(path);

		if !path.exists() {
			if let Some(parent) = path.parent() {
				fs::create_dir_all(parent)?;
			}

			let response = self.get(url).await?;

			let mut src = response.bytes().await?.reader();
			let mut dest = File::create(&path)?;
			std::io::copy(&mut src, &mut dest)?;
		}

		Ok(Jar::new(path))
	}

	async fn versions_manifest(&mut self) -> Result<&VersionsManifest> {
		if let Some(ref version_manifest) = self.versions_manifest {
			Ok(version_manifest)
		} else {
			let url = "https://skyrising.github.io/mc-versions/version_manifest.json";

			let body = self.get(url).await?.text().await?;

			let versions_manifest: VersionsManifest = serde_json::from_str(&body)?;

			Ok(self.versions_manifest.insert(versions_manifest))
		}
	}

	async fn wanted_version_manifest(&mut self, version: &Version) -> Result<VersionManifest> {
		let minecraft_version = version.get_minecraft_version();

		let manifest = self.versions_manifest().await?;

		let manifest_version = manifest
			.versions
			.iter()
			.find(|it| it.id == minecraft_version);

		if let Some(manifest_version) = manifest_version {
			let url = manifest_version.url.clone();

			let body = self.get(&url).await?.text().await?;

			let version_manifest: VersionManifest = serde_json::from_str(&body)
				.with_context(|| anyhow!("failed to parse version manifest for version {:?} from {:?}", version, url))?;

			Ok(version_manifest)
		} else {
			bail!("no version data for Minecraft version {:?}", version);
		}
	}
	pub(crate) async fn version_details(&mut self, version: &Version, environment: &Environment) -> Result<VersionDetails> {
		let minecraft_version = version.get_minecraft_version();

		let manifest = self.versions_manifest().await?;

		let manifest_version = manifest.versions.iter()
			.find(|it| it.id == minecraft_version);

		if let Some(manifest_version) = manifest_version {
			let url = &manifest_version.details.clone();

			let body = self.get(url).await?.text().await?;

			let version_details: VersionDetails = serde_json::from_str(&body)
				.with_context(|| anyhow!("failed to parse version details for version {:?} from {:?}", version, url))?;

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
		} else {
			bail!("no version details for minecraft version {:?}", version);
		}
	}
	/// Downloads the feather intermediary, calamus, for a given version.
	///
	/// The namespaces are `official` to `intermediary` (aka `calamus`) here.
	pub(crate) async fn calamus_v2(&mut self, version: &Version) -> Result<Mappings<2>> {
		let url = format!("https://maven.ornithemc.net/releases/net/ornithemc/calamus-intermediary/{version}/calamus-intermediary-{version}-v2.jar");

		let body = self.get(&url).await?.bytes().await?;

		let mut zip = ZipArchive::new(Cursor::new(body))?;

		let mappings = zip.by_name("mappings/mappings.tiny")
			.with_context(|| anyhow!("cannot find mappings in zip file from {:?}", url))?;

		let mappings = mappings_rw::tiny_v2::read(mappings)
			.with_context(|| anyhow!("failed to read mappings from mappings/mappings.tiny of {:?}", url))?;

		mappings.info.namespaces.check_that(["official", "intermediary"])?;

		Ok(mappings)
	}
	pub(crate) async fn mc_libs(&mut self, version: &Version) -> Result<Vec<Jar>> {
		let version_file = self.wanted_version_manifest(version).await?;

		let mut libs = Vec::new();

		for lib in version_file.libraries {
			if let Some(artifact) = lib.downloads.artifact {
				let jar = self.get_jar(&artifact.url).await?;

				libs.push(jar);
			}
		}

		Ok(libs)
	}

	pub(crate) async fn get_maven_metadata_xml(&mut self, url: &str) -> Result<MavenMetadata> {
		let body = self.get(url).await?.text().await?;

		serde_xml_rs::from_str(&body)
			.with_context(|| anyhow!("failed to parse maven metadata xml file from {url:?}"))
	}
}
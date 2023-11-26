use std::io::Cursor;
use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use zip::ZipArchive;
use crate::download::version_details::VersionDetails;
use crate::download::version_manifest::VersionManifest;
use crate::download::versions_manifest::VersionsManifest;
use crate::{Environment, Jar};
use crate::tree::mappings::Mappings;
use crate::version_graph::Version;

pub(crate) mod versions_manifest;
pub(crate) mod version_manifest;
pub(crate)mod version_details;

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
		let response = reqwest::get(url).await?;

		if response.status().is_success() {
			Ok(response)
		} else {
			bail!("Got a \"{}\" for {url:?}", response.status());
		}
	}

	pub(crate) async fn get_jar(&mut self, url: &str) -> Result<Jar> {
		// TODO: download + cache jar


		// from libs:
		// to a file given by the
		// part after the last slash into a libraries folder (ensuring that we don't overwrite a file)

		Ok(Jar)
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
		let manifest = self.versions_manifest().await?;

		let manifest_version = manifest
			.versions
			.iter()
			.find(|it| &it.id == version);

		if let Some(manifest_version) = manifest_version {
			let url = manifest_version.url.clone();

			let body = self.get(&url).await?.text().await?;

			let version_manifest: VersionManifest = serde_json::from_str(&body)
				.with_context(|| anyhow!("Failed to parse version manifest for version {:?} from {:?}", version, url))?;

			Ok(version_manifest)
		} else {
			bail!("No version data for Minecraft version {:?}", version);
		}
	}
	pub(crate) async fn version_details(&mut self, version: &Version, environment: &Environment) -> Result<VersionDetails> {
		let manifest = self.versions_manifest().await?;

		let manifest_version = manifest
			.versions
			.iter()
			.find(|it| &it.id == version);

		if let Some(manifest_version) = manifest_version {
			let url = &manifest_version.details.clone();

			let body = self.get(url).await?.text().await?;

			let version_details: VersionDetails = serde_json::from_str(&body)
				.with_context(|| anyhow!("Failed to parse version details for version {:?} from {:?}", version, url))?;

			if version_details.shared_mappings {
				if &Environment::Merged != environment {
					bail!("Minecraft version {:?} is only available as merged but was requested for {:?}", version, environment);
				}
			} else {
				match environment {
					Environment::Merged => {
						bail!("Minecraft version {:?} cannot be merged - please select either the client or server environment!", version);
					},
					Environment::Client if !version_details.client => {
						bail!("Minecraft version {:?} does not have a client jar!", version);
					},
					Environment::Server if !version_details.server => {
						bail!("Minecraft version {:?} does not have a server jar", version);
					},
					_ => {},
				}
			}

			Ok(version_details)
		} else {
			bail!("No version details for Minecraft version {:?}", version);
		}
	}
	pub(crate) async fn calamus_v2(&mut self, version: &Version) -> Result<Mappings<2>> {
		let url = format!("https://maven.ornithemc.net/releases/net/ornithemc/calamus-intermediary/{}/calamus-intermediary-{}-v2.jar", version.0, version.0);

		let body = self.get(&url).await?.bytes().await?;

		let reader = Cursor::new(body);

		let mut zip = ZipArchive::new(reader)?;

		let mappings = zip.by_name("mappings/mappings.tiny")
			.with_context(|| anyhow!("Cannot find mappings in zip file from {:?}", url))?;

		crate::reader::tiny_v2::read(mappings)
			.with_context(|| anyhow!("Failed to read mappings from mappings/mappings.tiny of {:?}", url))
	}
	pub(crate) async fn mc_libs(&mut self, version: &Version) -> Result<Vec<Jar>> {
		let version_file = self.wanted_version_manifest(version).await?;

		let mut libs = Vec::new();

		for lib in version_file.libraries {
			if let Some(artifact) = lib.downloads.artifact {
				let url = &artifact.url;

				let jar = self.get_jar(&url).await?;

				libs.push(jar);
			}
		}

		Ok(libs)
	}
}
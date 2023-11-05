
use anyhow::{bail, Result};

pub(crate) mod versions_manifest;
pub(crate) mod version_manifest;
pub(crate)mod version_details;


pub(crate) async fn get(url: &str) -> Result<String> {
	let response = reqwest::get(url)
		.await?;

	if response.status().is_success() {
		Ok(response.text().await?)
	} else {
		bail!("Got a \"{}\" for {:?}", response.status(), url);
	}
}
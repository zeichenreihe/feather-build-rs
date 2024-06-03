use std::fs::File;
use std::path::PathBuf;
use anyhow::{anyhow, Context, Result};
use zip::ZipArchive;
use crate::Jar;

#[derive(Debug)]
pub struct FileJar {
	pub(crate) path: PathBuf,
}

impl FileJar {
	pub fn new(path: PathBuf) -> FileJar {
		FileJar { path }
	}
}

impl Jar for FileJar {
	type Opened<'a> = ZipArchive<File> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		let file = File::open(&self.path)
			.with_context(|| anyhow!("could not open file {self:?}"))?
		ZipArchive::new(file)
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}
}
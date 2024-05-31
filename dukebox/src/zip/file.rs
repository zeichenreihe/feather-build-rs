use std::fs::File;
use std::path::PathBuf;
use anyhow::{anyhow, Context, Result};
use crate::zip::JarFromReader;

#[derive(Debug, Clone)]
pub struct FileJar {
	path: PathBuf,
}

impl FileJar {
	pub fn new(path: PathBuf) -> FileJar {
		FileJar { path }
	}
}

impl JarFromReader for FileJar {
	type Reader<'a> = File;

	fn open(&self) -> Result<Self::Reader<'_>> {
		File::open(&self.path)
			.with_context(|| anyhow!("failed to open jar at {:?}", self.path))
	}
}
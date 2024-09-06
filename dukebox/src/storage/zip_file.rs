use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Result};
use zip::ZipArchive;
use crate::storage::Jar;

/// A jar read from a path.
#[derive(Debug)]
pub struct FileJar {
	/// The path pointing to a zip archive, that's read as the jar.
	pub path: PathBuf,
}

impl Jar for FileJar {
	type Opened<'a> = ZipArchive<File> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		let file = File::open(&self.path)
			.with_context(|| anyhow!("could not open file {self:?}"))?;
		ZipArchive::new(file)
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}

	fn put_to_file<'a>(&'a self, suggested: &'a Path) -> Result<&'a Path> {
		// since this is only a suggestion and we already have a path
		let _ = suggested;
		Ok(&self.path)
	}
}
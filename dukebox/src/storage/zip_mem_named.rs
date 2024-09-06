use anyhow::{anyhow, Context, Result};
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::path::Path;
use zip::ZipArchive;
use crate::storage::Jar;

/// A named, in-memory jar.
#[derive(Clone)]
pub struct NamedMemJar {
	/// An arbitrary name for the jar.
	pub name: String,
	/// The data for the jar.
	///
	/// This is read as a zip archive.
	pub data: Vec<u8>,
}

/// [`Debug`] only prints the name and size, not the actual data.
impl Debug for NamedMemJar {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MemJar")
			.field("name", &self.name)
			.field("size", &self.data.len())
			.finish_non_exhaustive()
	}
}

impl Jar for NamedMemJar {
	type Opened<'a> = ZipArchive<Cursor<&'a Vec<u8>>> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		ZipArchive::new(Cursor::new(&self.data))
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}

	fn put_to_file<'a>(&'a self, suggested: &'a Path) -> Result<&'a Path> {
		std::fs::write(suggested, &self.data)
			.map(|()| suggested)
			.with_context(|| anyhow!("failed to write named ({:?}) in-memory jar to {suggested:?}", self.name))
	}
}
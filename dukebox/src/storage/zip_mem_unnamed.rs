use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use zip::ZipArchive;
use crate::storage::Jar;

/// An unnamed, in-memory jar.
#[derive(Clone)]
pub struct UnnamedMemJar {
	/// The data for the jar.
	///
	/// This is read as a zip archive.
	pub data: Vec<u8>,
}

/// [`Debug`] only prints the size, not the actual data.
impl Debug for UnnamedMemJar {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MemJar")
			.field("size", &self.data.len())
			.finish_non_exhaustive()
	}
}

impl Jar for UnnamedMemJar {
	type Opened<'a> = ZipArchive<Cursor<&'a Vec<u8>>> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		ZipArchive::new(Cursor::new(&self.data))
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}

	fn put_to_file<'a>(&'a self, suggested: &'a Path) -> Result<&'a Path> {
		std::fs::write(suggested, &self.data)
			.map(|()| suggested)
			.with_context(|| anyhow!("failed to write unnamed in-memory jar to {suggested:?}"))
	}
}

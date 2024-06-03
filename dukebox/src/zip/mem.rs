use anyhow::{anyhow, Context, Result};
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use zip::ZipArchive;
use crate::Jar;

#[derive(Clone)]
pub struct MemJar {
	name: Option<String>,
	pub(crate) data: Vec<u8>,
}

impl Debug for MemJar {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MemJar").field("name", &self.name).finish_non_exhaustive()
	}
}

impl MemJar {
	pub fn new(name: String, data: Vec<u8>) -> MemJar {
		MemJar { name: Some(name), data }
	}

	pub(crate) fn new_unnamed(data: Vec<u8>) -> MemJar {
		MemJar { name: None, data }
	}
}

impl Jar for MemJar {
	type Opened<'a> = ZipArchive<Cursor<&'a Vec<u8>>> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		ZipArchive::new(Cursor::new(&self.data))
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}
}
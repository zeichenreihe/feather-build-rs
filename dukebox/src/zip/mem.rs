use anyhow::{anyhow, Context, Result};
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use zip::ZipArchive;
use crate::Jar;

#[derive(Clone)]
pub struct NamedMemJar {
	name: String,
	pub(crate) data: Vec<u8>,
}

impl Debug for NamedMemJar {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MemJar").field("name", &self.name).finish_non_exhaustive()
	}
}

impl NamedMemJar {
	pub fn new(name: String, data: Vec<u8>) -> NamedMemJar {
		NamedMemJar { name, data }
	}
}

impl Jar for NamedMemJar {
	type Opened<'a> = ZipArchive<Cursor<&'a Vec<u8>>> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		ZipArchive::new(Cursor::new(&self.data))
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}
}


#[derive(Clone)]
pub struct UnnamedMemJar {
	pub(crate) data: Vec<u8>,
}

impl Debug for UnnamedMemJar {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MemJar").finish_non_exhaustive()
	}
}

impl UnnamedMemJar {
	pub fn new(data: Vec<u8>) -> UnnamedMemJar {
		UnnamedMemJar { data }
	}
}

impl Jar for UnnamedMemJar {
	type Opened<'a> = ZipArchive<Cursor<&'a Vec<u8>>> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		ZipArchive::new(Cursor::new(&self.data))
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}
}
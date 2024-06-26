use std::fs::File;
use anyhow::{anyhow, Context, Result};
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;
use crate::Jar;
use crate::zip::file::FileJar;
use crate::zip::mem::NamedMemJar;

#[derive(Debug)]
pub enum EnumJar {
	File(FileJar),
	Mem(NamedMemJar),
}

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

impl Jar for EnumJar {
	type Opened<'a> = ZipArchive<Box<dyn ReadSeek + 'a>> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		let reader: Box<dyn ReadSeek> = match self {
			EnumJar::File(file) => Box::new(File::open(&file.path)
				.with_context(|| anyhow!("could not open file {file:?}"))?),
			EnumJar::Mem(mem) => Box::new(Cursor::new(&mem.data)),
		};

		ZipArchive::new(reader)
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}
}

impl From<FileJar> for EnumJar {
	fn from(value: FileJar) -> Self {
		EnumJar::File(value)
	}
}

impl From<NamedMemJar> for EnumJar {
	fn from(value: NamedMemJar) -> Self {
		EnumJar::Mem(value)
	}
}
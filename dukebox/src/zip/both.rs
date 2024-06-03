use std::fs::File;
use anyhow::{anyhow, Context, Result};
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;
use crate::Jar;
use crate::zip::file::FileJar;
use crate::zip::mem::MemJar;

#[derive(Debug)]
pub enum EnumJarFromReader {
	File(FileJar),
	Mem(MemJar),
}

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

impl Jar for EnumJarFromReader {
	type Opened<'a> = ZipArchive<Box<dyn ReadSeek + 'a>> where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		let reader: Box<dyn ReadSeek> = match self {
			EnumJarFromReader::File(file) => Box::new(File::open(&file.path)
				.with_context(|| anyhow!("could not open file {file:?}"))?),
			EnumJarFromReader::Mem(mem) => Box::new(Cursor::new(&mem.data)),
		};

		ZipArchive::new(reader)
			.with_context(|| anyhow!("failed to read zip archive from {self:?}"))
	}
}

impl From<FileJar> for EnumJarFromReader {
	fn from(value: FileJar) -> Self {
		EnumJarFromReader::File(value)
	}
}

impl From<MemJar> for EnumJarFromReader {
	fn from(value: MemJar) -> Self {
		EnumJarFromReader::Mem(value)
	}
}
use anyhow::Result;
use std::io::{Read, Seek};
use crate::zip::JarFromReader;
use crate::zip::file::FileJar;
use crate::zip::mem::MemJar;

#[derive(Debug, Clone)]
pub enum EnumJarFromReader {
	File(FileJar),
	Mem(MemJar),
}

pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

impl JarFromReader for EnumJarFromReader {
	type Reader<'a> = Box<dyn ReadSeek + 'a>;

	fn open(&self) -> Result<Self::Reader<'_>> {
		Ok(match self {
			EnumJarFromReader::File(file) => {
				Box::new(file.open()?)
			},
			EnumJarFromReader::Mem(mem) => {
				Box::new(mem.open()?)
			},
		})
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
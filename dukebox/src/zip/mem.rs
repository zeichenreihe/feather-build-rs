use anyhow::Result;
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use crate::zip::JarFromReader;

#[derive(Clone)]
pub struct MemJar {
	name: Option<String>,
	data: Vec<u8>
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

impl JarFromReader for MemJar {
	type Reader<'a> = Cursor<&'a Vec<u8>>;

	fn open(&self) -> Result<Self::Reader<'_>> {
		Ok(Cursor::new(&self.data))
	}
}
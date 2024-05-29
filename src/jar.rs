pub(crate) mod merge;

use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::PathBuf;
use anyhow::Result;
use zip::ZipArchive;
use class_file::visitor::MultiClassVisitor;

#[derive(Clone)]
pub(crate) enum Jar {
	File {
		path: PathBuf,
	},
	Mem {
		name: Option<String>,
		data: Vec<u8>,
	},
}

impl Debug for Jar {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Jar::File { path } => f.debug_struct("Jar::File").field("path", path).finish(),
			Jar::Mem { name, .. } => f.debug_struct("Jar::Mem").field("name", name).finish_non_exhaustive(),
		}
	}
}

impl Jar {
	pub(crate) fn new(path: PathBuf) -> Jar {
		Jar::File { path }
	}
	pub(crate) fn new_mem(name: String, data: Vec<u8>) -> Jar {
		let name = Some(name);
		Jar::Mem { name, data }
	}
	pub(crate) fn new_mem_unnamed(data: Vec<u8>) -> Jar {
		Jar::Mem { name: None, data }
	}

	pub(crate) fn read_into<V: MultiClassVisitor>(&self, visitor: V) -> Result<V> {
		fn action<V: MultiClassVisitor>(reader: impl Read + Seek, mut visitor: V) -> Result<V> {
			let mut zip = ZipArchive::new(reader)?;

			for index in 0..zip.len() {
				let mut file = zip.by_index(index)?;
				if file.name().ends_with(".class") {

					let mut vec = Vec::new();
					file.read_to_end(&mut vec)?;
					let mut reader = Cursor::new(vec);

					visitor = class_file::read_class_multi(&mut reader, visitor)?;
				}
			}

			Ok(visitor)
		}

		match self {
			Jar::File { path } => {
				let reader = File::open(path)?;
				action(reader, visitor)
			},
			Jar::Mem { data, .. } => {
				let reader = Cursor::new(data);
				action(reader, visitor)
			}
		}
	}

	pub(crate) fn for_each_class(&self, f: impl FnMut(Cursor<Vec<u8>>) -> Result<()>) -> Result<()> {
		fn action(reader: impl Read + Seek, mut f: impl FnMut(Cursor<Vec<u8>>) -> Result<()>) -> Result<()> {
			let mut zip = ZipArchive::new(reader)?;

			for index in 0..zip.len() {
				let mut file = zip.by_index(index)?;
				if file.name().ends_with(".class") {

					let mut vec = Vec::new();
					file.read_to_end(&mut vec)?;
					let reader = Cursor::new(vec);

					f(reader)?;
				}
			}

			Ok(())
		}

		match self {
			Jar::File { path } => {
				let reader = File::open(path)?;
				action(reader, f)
			},
			Jar::Mem { data, .. } => {
				let reader = Cursor::new(data);
				action(reader, f)
			},
		}
	}
}
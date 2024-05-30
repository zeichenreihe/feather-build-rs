pub(crate) mod merge;

use std::convert::Infallible;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::ops::ControlFlow;
use std::path::PathBuf;
use anyhow::{anyhow, Context, Result};
use indexmap::{IndexMap, IndexSet};
use zip::ZipArchive;
use duke::tree::class::{ClassAccess, ClassName};
use duke::tree::version::Version;
use duke::visitor::MultiClassVisitor;
use quill::remapper::JarSuperProv;

pub(crate) trait Jar {
	type Reader<'a>: Read + Seek + 'a where Self: 'a;

	fn open(&self) -> Result<Self::Reader<'_>>;

	fn read_into<V: MultiClassVisitor>(&self, mut visitor: V) -> Result<V> {
		let reader = self.open()?;
		let mut zip = ZipArchive::new(reader)?;

		for index in 0..zip.len() {
			let mut file = zip.by_index(index)?;
			if file.name().ends_with(".class") {
				let mut vec = Vec::new();
				file.read_to_end(&mut vec)?;
				let mut reader = Cursor::new(vec);

				visitor = duke::read_class_multi(&mut reader, visitor)?;
			}
		}

		Ok(visitor)
	}

	fn for_each_class(&self, mut f: impl FnMut(Cursor<Vec<u8>>) -> Result<()>) -> Result<()> {
		let reader = self.open()?;
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

	fn get_super_classes_provider(&self) -> Result<JarSuperProv> {
		struct MyJarSuperProv(JarSuperProv);
		impl MultiClassVisitor for MyJarSuperProv {
			type ClassVisitor = Infallible;
			type ClassResidual = Infallible;

			fn visit_class(mut self, _version: Version, _access: ClassAccess, name: ClassName, super_class: Option<ClassName>, interfaces: Vec<ClassName>)
				-> Result<ControlFlow<Self, (Self::ClassResidual, Self::ClassVisitor)>>
			{
				let mut set = IndexSet::new();
				if let Some(super_class) = super_class {
					set.insert(super_class);
				}
				for interface in interfaces {
					set.insert(interface);
				}
				self.0.super_classes.insert(name, set);
				Ok(ControlFlow::Break(self))
			}

			fn finish_class(_this: Self::ClassResidual, _class_visitor: Self::ClassVisitor) -> Result<Self> {
				unreachable!()
			}
		}

		Ok(self.read_into(MyJarSuperProv(JarSuperProv { super_classes: IndexMap::new() }))?.0)
	}
}

#[derive(Debug, Clone)]
pub(crate) struct FileJar {
	path: PathBuf,
}

impl FileJar {
	pub(crate) fn new(path: PathBuf) -> FileJar {
		FileJar { path }
	}
}

impl Jar for FileJar {
	type Reader<'a> = File;

	fn open(&self) -> Result<Self::Reader<'_>> {
		File::open(&self.path)
			.with_context(|| anyhow!("failed to open jar at {:?}", self.path))
	}
}

#[derive(Clone)]
pub(crate) struct MemJar {
	name: Option<String>,
	data: Vec<u8>
}

impl Debug for MemJar {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MemJar").field("name", &self.name).finish_non_exhaustive()
	}
}

impl MemJar {
	pub(crate) fn new(name: String, data: Vec<u8>) -> MemJar {
		MemJar { name: Some(name), data }
	}

	pub(crate) fn new_unnamed(data: Vec<u8>) -> MemJar {
		MemJar { name: None, data }
	}
}

impl Jar for MemJar {
	type Reader<'a> = Cursor<&'a Vec<u8>>;

	fn open(&self) -> Result<Self::Reader<'_>> {
		Ok(Cursor::new(&self.data))
	}
}

#[derive(Debug, Clone)]
pub(crate) enum EnumJar {
	File(FileJar),
	Mem(MemJar),
}

pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

impl Jar for EnumJar {
	type Reader<'a> = Box<dyn ReadSeek + 'a>;

	fn open(&self) -> Result<Self::Reader<'_>> {
		Ok(match self {
			EnumJar::File(file) => {
				Box::new(file.open()?)
			},
			EnumJar::Mem(mem) => {
				Box::new(mem.open()?)
			},
		})
	}
}

impl From<FileJar> for EnumJar {
	fn from(value: FileJar) -> Self {
		EnumJar::File(value)
	}
}

impl From<MemJar> for EnumJar {
	fn from(value: MemJar) -> Self {
		EnumJar::Mem(value)
	}
}

pub(crate) mod merge;

use std::convert::Infallible;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::ops::ControlFlow;
use std::path::PathBuf;
use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use zip::ZipArchive;
use class_file::tree::class::{ClassAccess, ClassName};
use class_file::tree::version::Version;
use class_file::visitor::MultiClassVisitor;
use mappings_rw::remapper::JarSuperProv;

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

impl Jar {
	pub(crate) fn get_super_classes_provider(&self) -> Result<JarSuperProv> {
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
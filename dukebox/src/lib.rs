pub mod merge;

use std::convert::Infallible;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::ops::ControlFlow;
use std::path::PathBuf;
use anyhow::{anyhow, Context, Result};
use indexmap::{IndexMap, IndexSet};
use zip::{DateTime, ZipArchive};
use duke::tree::class::{ClassAccess, ClassFile, ClassName};
use duke::tree::version::Version;
use duke::visitor::MultiClassVisitor;
use quill::remapper::JarSuperProv;

pub trait Jar {
	type Entry<'a>: JarEntry where Self: 'a;
	type Iter<'a>: Iterator<Item=Self::Entry<'a>> where Self: 'a;

	fn entries<'a: 'b, 'b>(&'a self) -> Result<Self::Iter<'b>>;

	fn read_classes_into<V: MultiClassVisitor>(&self, mut visitor: V) -> Result<V> {
		for entry in self.entries()? {
			if !entry.is_dir() && entry.is_class() {
				visitor = entry.visit_as_class(visitor)?;
			}
		}
		Ok(visitor)
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

		Ok(self.read_classes_into(MyJarSuperProv(JarSuperProv { super_classes: IndexMap::new() }))?.0)
	}
}

pub trait JarEntry {
	fn is_dir(&self) -> bool;
	fn name(&self) -> &str;

	fn is_class(&self) -> bool {
		!self.is_dir() && self.name().ends_with(".class")
	}
	fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V>;

	fn get_vec(&self) -> Vec<u8>;

	fn attrs(&self) -> BasicFileAttributes;
}

#[derive(Clone, Debug)]
struct BasicFileAttributes {
	mtime: DateTime,
	atime: (),
	ctime: (),
}

pub(crate) trait JarFromReader {
	type Reader<'a>: Read + Seek + 'a where Self: 'a;

	fn open(&self) -> Result<Self::Reader<'_>>;

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
}

impl<T: JarFromReader> Jar for T {
	type Entry<'a> = ZipFileEntry where Self: 'a;
	type Iter<'a> = std::vec::IntoIter<ZipFileEntry> where Self: 'a;

	fn entries<'a: 'b, 'b>(&'a self) -> Result<Self::Iter<'b>> {
		let reader = self.open()?;
		let mut zip = ZipArchive::new(reader)?;

		let mut out = Vec::with_capacity(zip.len());
		for index in 0..zip.len() {
			let mut file = zip.by_index(index)?;
			out.push(ZipFileEntry {
				is_dir: file.is_dir(),
				name: file.name().to_owned(),
				vec: { let mut vec = Vec::new(); file.read_to_end(&mut vec)?; vec },
				attrs: BasicFileAttributes { // TODO: implement reading the more exact file modification times from the extra data of the zip file
					mtime: file.last_modified(),
					atime: (),
					ctime: (),
				},
			});
		}

		Ok(out.into_iter())
	}

	fn read_classes_into<V: MultiClassVisitor>(&self, mut visitor: V) -> Result<V> {
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
}

pub struct ZipFileEntry {
	is_dir: bool,
	name: String,
	vec: Vec<u8>,
	attrs: BasicFileAttributes,
}

impl JarEntry for ZipFileEntry {
	fn is_dir(&self) -> bool {
		self.is_dir
	}

	fn name(&self) -> &str {
		&self.name
	}

	fn visit_as_class<V: MultiClassVisitor>(mut self, visitor: V) -> Result<V> {
		let mut reader = Cursor::new(&self.vec);

		duke::read_class_multi(&mut reader, visitor)
	}

	fn get_vec(&self) -> Vec<u8> {
		self.vec.clone()
	}

	fn attrs(&self) -> BasicFileAttributes {
		self.attrs.clone()
	}
}

#[derive(Debug, Clone)]
pub struct FileJar {
	path: PathBuf,
}

impl FileJar {
	pub fn new(path: PathBuf) -> FileJar {
		FileJar { path }
	}
}

impl JarFromReader for FileJar {
	type Reader<'a> = File;

	fn open(&self) -> Result<Self::Reader<'_>> {
		File::open(&self.path)
			.with_context(|| anyhow!("failed to open jar at {:?}", self.path))
	}
}

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

struct ParsedJar {
	entries: Vec<ParsedJarEntry>,
}

impl Jar for ParsedJar {
	type Entry<'a> = &'a ParsedJarEntry;
	type Iter<'a> = std::slice::Iter<'a, ParsedJarEntry> where Self: 'a;

	fn entries<'a: 'b, 'b>(&'a self) -> Result<Self::Iter<'b>> {
		Ok(self.entries.iter())
	}
}

enum ParsedJarEntry {
	Class {
		name: String,
		class: ClassFile,
	},
	ClassAsVec {
		name: String,
		data: Vec<u8>,
	},
	Other {
		name: String,
		data: Vec<u8>,
	},
	Dir {
		name: String,
	},
}

impl JarEntry for &ParsedJarEntry {
	fn is_dir(&self) -> bool {
		matches!(self, ParsedJarEntry::Dir { .. })
	}

	fn name(&self) -> &str {
		match self {
			ParsedJarEntry::Class { name, .. } => name,
			ParsedJarEntry::ClassAsVec { name, .. } => name,
			ParsedJarEntry::Other { name, .. } => name,
			ParsedJarEntry::Dir { name, .. } => name,
		}
	}

	fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V> {
		match self {
			ParsedJarEntry::Class { class, .. } => {
				class.clone().accept(visitor)
			},
			ParsedJarEntry::ClassAsVec { data, .. } => {
				duke::read_class_multi(&mut Cursor::new(data), visitor)
			},
			_ => Ok(visitor),
		}
	}

	fn get_vec(&self) -> Vec<u8> {
		todo!()
	}

	fn attrs(&self) -> BasicFileAttributes {
		todo!()
	}
}

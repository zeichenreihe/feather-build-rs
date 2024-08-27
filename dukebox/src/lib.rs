use std::convert::Infallible;
use std::fmt::{Debug, Formatter};
use std::ops::ControlFlow;
use std::path::Path;
use anyhow::{Result};
use indexmap::{IndexMap, IndexSet};
use ::zip::{DateTime, ExtraField};
use ::zip::write::{ExtendedFileOptions, FileOptions};
use duke::tree::class::{ClassAccess, ClassFile, ClassName};
use duke::tree::version::Version;
use duke::visitor::MultiClassVisitor;
use quill::remapper::JarSuperProv;
use crate::lazy_duke::ClassRepr;

pub mod merge;
pub mod remap;
pub mod parsed;
pub mod zip;
pub mod lazy_duke;

pub trait Jar {
	type Opened<'a>: OpenedJar where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>>;

	fn put_to_file<'a>(&'a self, suggested: &'a Path) -> Result<&'a Path>;

	fn get_super_classes_provider(&self) -> Result<JarSuperProv> {
		self.open()?.get_super_classes_provider()
	}
}

pub trait OpenedJar {
	type EntryKey: Copy;

	type Entry<'a>: JarEntry where Self: 'a;

	fn entry_keys(&self) -> impl Iterator<Item=Self::EntryKey> + 'static;

	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>>;

	fn names(&self) -> impl Iterator<Item=(Self::EntryKey, &'_ str)>;

	fn by_name(&mut self, name: &str) -> Result<Option<Self::Entry<'_>>>;

	fn read_classes_into<V: MultiClassVisitor>(&mut self, mut visitor: V) -> Result<V> {
		let keys = self.entry_keys();
		for key in keys {
			let entry = self.by_entry_key(key)?;

			if let JarEntryEnum::Class(class) = entry.to_jar_entry_enum()? {
				visitor = class.visit(visitor)?;
			}
		}

		Ok(visitor)
	}

	fn get_super_classes_provider(&mut self) -> Result<JarSuperProv> {
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
	fn name(&self) -> &str;

	fn attrs(&self) -> BasicFileAttributes;

	type Class: IsClass;
	type Other: IsOther;
	fn to_jar_entry_enum(self) -> Result<JarEntryEnum<Self::Class, Self::Other>>;
}

pub enum JarEntryEnum<Class, Other> {
	Dir,
	Class(Class),
	Other(Other),
}

impl<Class, Other> JarEntryEnum<Class, Other> {
	pub(crate) fn map_both<NewClass, NewOther>(
		self,
		class_f: impl FnOnce(Class) -> NewClass,
		other_f: impl FnOnce(Other) -> NewOther,
	) -> JarEntryEnum<NewClass, NewOther> {
		use JarEntryEnum::*;
		match self {
			Dir => Dir,
			Class(class) => Class(class_f(class)),
			Other(other) => Other(other_f(other)),
		}
	}

	pub(crate) fn try_map_both<NewClass, NewOther>(
		self,
		class_f: impl FnOnce(Class) -> Result<NewClass>,
		other_f: impl FnOnce(Other) -> Result<NewOther>,
	) -> Result<JarEntryEnum<NewClass, NewOther>> {
		use JarEntryEnum::*;
		Ok(match self {
			Dir => Dir,
			Class(class) => Class(class_f(class)?),
			Other(other) => Other(other_f(other)?),
		})
	}
}

impl<Class, Other> Debug for JarEntryEnum<Class, Other> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		use JarEntryEnum::*;
		match self {
			Dir => write!(f, "Dir"),
			Class(_) => write!(f, "Class"),
			Other(_) => write!(f, "Other"),
		}
	}
}

pub trait IsClass {
	fn read(self) -> Result<ClassFile>;

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M>;

	type Written<'a>: AsRef<[u8]> where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>>;

	// TODO: remove?
	fn into_class_repr(self) -> ClassRepr;
}

impl IsClass for ClassFile {
	fn read(self) -> Result<ClassFile> {
		Ok(self)
	}

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M> {
		self.accept(visitor)
	}

	type Written<'a> = Vec<u8> where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>> {
		let mut buf = Vec::new();
		duke::write_class(&mut buf, self)?;
		Ok(buf)
	}

	fn into_class_repr(self) -> ClassRepr {
		ClassRepr::Parsed { class: self }
	}
}

pub trait IsOther {
	fn get_data(&self) -> &[u8];
	fn get_data_owned(self) -> Vec<u8>;
}

#[derive(Clone, Copy, Debug)]
pub struct BasicFileAttributes {
	last_modified: Option<DateTime>,
	mtime: Option<u32>,
	atime: Option<u32>,
	ctime: Option<u32>,
}

impl BasicFileAttributes {
	fn new<'a>(last_modified: Option<DateTime>, extra_data_fields: impl Iterator<Item=&'a ExtraField>) -> BasicFileAttributes {
		let extended_timestamp = extra_data_fields
			.filter_map(|extra_field| match extra_field {
				ExtraField::ExtendedTimestamp(x) => Some(x),
				#[allow(unreachable_patterns)]
				_ => None,
			})
			.next();

		let mtime = extended_timestamp.and_then(|x| x.mod_time());
		let atime = extended_timestamp.and_then(|x| x.ac_time());
		let ctime = extended_timestamp.and_then(|x| x.cr_time());

		BasicFileAttributes { last_modified, mtime, atime, ctime }
	}

	pub fn new_empty() -> BasicFileAttributes {
		BasicFileAttributes {
			last_modified: None,
			mtime: None,
			atime: None,
			ctime: None,
		}
	}

	fn to_file_options<'k>(self) -> FileOptions<'k, ExtendedFileOptions> {
		let mut file_options = FileOptions::default();

		if let Some(last_modified) = self.last_modified {
			file_options = file_options.last_modified_time(last_modified);
		}
		// TODO: awaiting lib support: set the ctime, atime, mtime to the ones from self

		file_options
	}
}
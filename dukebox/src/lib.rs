use std::convert::Infallible;
use std::fmt::Debug;
use std::ops::ControlFlow;
use anyhow::{Result};
use indexmap::{IndexMap, IndexSet};
use ::zip::{DateTime, ExtraField};
use ::zip::write::{ExtendedFileOptions, FileOptions};
use duke::tree::class::{ClassAccess, ClassName};
use duke::tree::version::Version;
use duke::visitor::MultiClassVisitor;
use quill::remapper::JarSuperProv;
use crate::parsed::ParsedJarEntry;

pub mod merge;
mod parsed;
pub mod zip;
mod lazy_duke;

pub trait Jar {
	type Opened<'a>: OpenedJar where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>>;

	fn get_super_classes_provider(&self) -> Result<JarSuperProv> {
		self.open()?.get_super_classes_provider()
	}
}

pub trait OpenedJar {
	type Entry<'a>: JarEntry where Self: 'a;

	type EntryKey;
	type EntryKeyIter: Iterator<Item=Self::EntryKey>;

	fn entry_keys(&self) -> Self::EntryKeyIter;

	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>>;


	type Name<'a>: AsRef<str> where Self: 'a;
	type NameIter<'a>: Iterator<Item=Self::Name<'a>> where Self: 'a;

	fn names(&self) -> Self::NameIter<'_>;
	fn by_name(&mut self, name: &str) -> Result<Option<Self::Entry<'_>>>;


	fn read_classes_into<V: MultiClassVisitor>(&mut self, mut visitor: V) -> Result<V> {
		let keys = self.entry_keys();
		for key in keys {
			let entry = self.by_entry_key(key)?;

			if entry.is_class() {
				visitor = entry.visit_as_class(visitor)?;
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
	fn is_dir(&self) -> bool;
	fn name(&self) -> &str;

	fn is_class(&self) -> bool {
		!self.is_dir() && self.name().ends_with(".class")
	}
	fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V>;

	fn attrs(&self) -> BasicFileAttributes;

	fn to_parsed_jar_entry(self) -> Result<ParsedJarEntry>;
}

#[derive(Clone, Debug)]
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
				_ => None,
			})
			.next();

		let mtime = extended_timestamp.and_then(|x| x.mod_time()).copied();
		let atime = extended_timestamp.and_then(|x| x.ac_time()).copied();
		let ctime = extended_timestamp.and_then(|x| x.cr_time()).copied();

		BasicFileAttributes { last_modified, mtime, atime, ctime }
	}

	fn to_file_options<'k>(self) -> FileOptions<'k, ExtendedFileOptions> {
		// TODO: set the ctime, atime, mtime to the ones from self

		FileOptions::default().last_modified_time(self.last_modified.unwrap())
	}
}
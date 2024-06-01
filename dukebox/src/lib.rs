use std::convert::Infallible;
use std::fmt::Debug;
use std::ops::ControlFlow;
use anyhow::{Result};
use indexmap::{IndexMap, IndexSet};
use ::zip::DateTime;
use duke::tree::class::{ClassAccess, ClassName};
use duke::tree::version::Version;
use duke::visitor::MultiClassVisitor;
use quill::remapper::JarSuperProv;
use crate::parsed::ParsedJarEntry;

pub mod merge;
mod parsed;
pub mod zip;


pub trait Jar {
	type Entry<'a>: JarEntry where Self: 'a;
	type Iter<'a>: Iterator<Item=Self::Entry<'a>> where Self: 'a;

	fn entries<'a: 'b, 'b>(&'a self) -> Result<Self::Iter<'b>>;

	fn by_name(&self, name: &str) -> Result<Self::Entry<'_>>;

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

	fn attrs(&self) -> BasicFileAttributes;

	fn to_parsed_jar_entry(self) -> Result<ParsedJarEntry>;
}

#[derive(Clone, Debug)]
pub struct BasicFileAttributes {
	mtime: DateTime,
	atime: (),
	ctime: (),
}

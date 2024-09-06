use std::convert::Infallible;
use std::ops::ControlFlow;
use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use duke::tree::class::{ClassAccess, ClassName};
use duke::tree::version::Version;
use duke::visitor::MultiClassVisitor;
use quill::remapper::JarSuperProv;
use crate::storage::{IsClass, JarEntry, JarEntryEnum};

/// Represents an opened jar.
///
/// An opened jar can be read.
///
/// Each opened jar has an [`EntryKey`][OpenedJar::EntryKey] type (most implementations use `usize`)
/// that's used for uniquely identifying each entry. You can retrieve an iterator over these entry
/// keys with [`entry_keys`][OpenedJar::entry_keys], and use the entry key to get a [`JarEntry`] with
/// the [`by_entry_key`][OpenedJar::by_entry_key] method.
///
/// With the [`names`][OpenedJar::names] and [`by_name`][OpenedJar::by_name] methods, an opened jar
/// supports lookup by file name. Note that [`names`][OpenedJar::names] also returns the corresponding
/// [`EntryKey`][OpenedJar::EntryKey]s, which avoids slow string lookup.
pub trait OpenedJar {
	type EntryKey: Copy;

	type Entry<'a>: JarEntry where Self: 'a;

	fn entry_keys(&self) -> impl Iterator<Item=Self::EntryKey> + 'static;

	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>>;

	fn names(&self) -> impl Iterator<Item=(Self::EntryKey, &'_ str)>;

	fn by_name(&mut self, name: &str) -> Result<Option<Self::Entry<'_>>>;

	/// Visits all the classes into the multi class visitor.
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

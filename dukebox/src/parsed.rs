use std::fs::File;
use std::io::{Cursor, Seek, Write};
use std::ops::Range;
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use zip::write::FileOptions;
use zip::ZipWriter;
use duke::visitor::MultiClassVisitor;
use crate::{BasicFileAttributes, Jar, JarEntry, OpenedJar};
use crate::lazy_duke::ClassRepr;
use crate::zip::mem::UnnamedMemJar;

#[derive(Debug, Default)]
pub struct ParsedJar {
	pub entries: IndexMap<String, ParsedJarEntry>,
}

impl Jar for ParsedJar {
	type Opened<'a> = &'a ParsedJar where Self: 'a;

	fn open(&self) -> Result<Self::Opened<'_>> {
		Ok(self)
	}

	fn put_to_file<'a>(&'a self, suggested: &'a Path) -> Result<&'a Path> {
		let writer = File::create(suggested)
			.with_context(|| anyhow!("failed to open {suggested:?} for writing \"parsed\" jar"))?;

		self.write(writer)
			.with_context(|| anyhow!("failed to write \"parsed\" jar to {suggested:?}"))?;

		Ok(suggested)
	}
}

impl<'this> OpenedJar for &'this ParsedJar {
	type Entry<'a> = (&'a String, &'a ParsedJarEntry) where Self: 'a;

	type EntryKey = usize;
	type EntryKeyIter = Range<usize>;

	fn entry_keys(&self) -> Self::EntryKeyIter {
		0..self.entries.len()
	}
	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>> {
		self.entries.get_index(key)
			.with_context(|| anyhow!("no entry for index {key:?}"))
	}

	type NameIter<'a> = std::vec::IntoIter<(&'a str, usize)> where Self: 'a;

	fn names(&self) -> Self::NameIter<'_> {
		(0..self.entries.len()).map(|x| (self.entries.get_index(x).unwrap().0.as_str(), x)).collect::<Vec<_>>().into_iter()
	}

	fn by_name(&mut self, name: &str) -> Result<Option<Self::Entry<'_>>> {
		Ok(self.entries.get_key_value(name))
	}
}

impl ParsedJar {
	pub(crate) fn from_jar(jar: &impl Jar) -> Result<ParsedJar> {
		let mut jar = jar.open()?;

		let mut result = ParsedJar {
			entries: IndexMap::new(),
		};

		for key in jar.entry_keys() {
			let entry = jar.by_entry_key(key)?;

			let path = entry.name().to_owned();
			let entry = entry.to_parsed_jar_entry()?;

			result.entries.insert(path, entry);
		}

		Ok(result)
	}

	fn add_dirs_to<W: Write + Seek>(path: &str, mut zip_out: ZipWriter<W>) -> Result<()> {
		let mut x = path;
		while let Some((left, _)) = x.rsplit_once('/') {
			if !left.is_empty() {
				zip_out.add_directory(left, FileOptions::<()>::default())?;
			}
			x = left;
		}
		Ok(())
	}

	fn write<W: Write + Seek>(&self, writer: W) -> Result<W> {
		let mut zip_out = ZipWriter::new(writer);

		for (name, entry) in &self.entries {
			let name = name.as_str();
			match entry {
				ParsedJarEntry::Class { attr, class } => {
					let data = class.write()?;

					zip_out.start_file(name, attr.to_file_options())?;
					zip_out.write_all(&data)?;
				},
				ParsedJarEntry::Other { attr, data } => {
					zip_out.start_file(name, attr.to_file_options())?;
					zip_out.write_all(data)?;
				},
				ParsedJarEntry::Dir { attr } => {
					zip_out.add_directory(name, attr.to_file_options())?;
				},
			}
		}

		Ok(zip_out.finish()?)
	}

	pub fn to_mem(self) -> Result<UnnamedMemJar> {
		let vec = self.write(Cursor::new(Vec::new()))?
			.into_inner();

		Ok(UnnamedMemJar::new(vec))
	}
}

#[derive(Debug, Clone)]
pub enum ParsedJarEntry {
	Class {
		attr: BasicFileAttributes,
		class: ClassRepr,
	},
	Other {
		attr: BasicFileAttributes,
		data: Vec<u8>,
	},
	Dir {
		attr: BasicFileAttributes,
	},
}

impl JarEntry for (&String, &ParsedJarEntry) {
	fn is_dir(&self) -> bool {
		matches!(self.1, ParsedJarEntry::Dir { .. })
	}

	fn name(&self) -> &str {
		self.0
	}

	fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V> {
		match self.1 {
			ParsedJarEntry::Class { class, .. } => {
				class.clone().visit_as_class(visitor)
			},
			_ => Ok(visitor),
		}
	}

	fn attrs(&self) -> BasicFileAttributes {
		match self.1 {
			ParsedJarEntry::Class { attr, .. } => *attr,
			ParsedJarEntry::Other { attr, .. } => *attr,
			ParsedJarEntry::Dir { attr, .. } => *attr,
		}
	}

	fn to_parsed_jar_entry(self) -> Result<ParsedJarEntry> {
		Ok(self.1.clone())
	}
}
use std::io::{Cursor, Write};
use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use zip::write::FileOptions;
use zip::ZipWriter;
use duke::tree::class::ClassFile;
use duke::visitor::MultiClassVisitor;
use crate::{BasicFileAttributes, Jar, JarEntry};
use crate::zip::mem::MemJar;

pub struct ParsedJar {
	pub(crate) entries: IndexMap<String, ParsedJarEntry>,
}

impl Jar for ParsedJar {
	type Entry<'a> = (&'a String, &'a ParsedJarEntry);
	type Iter<'a> = indexmap::map::Iter<'a, String, ParsedJarEntry> where Self: 'a;

	fn entries<'a: 'b, 'b>(&'a self) -> Result<Self::Iter<'b>> {
		Ok(self.entries.iter())
	}

	fn by_name(&self, name: &str) -> Result<Self::Entry<'_>> {
		self.entries.get_key_value(name)
			.with_context(|| anyhow!("no jar entry for name {name:?}"))
	}
}

impl ParsedJar {
	pub(crate) fn put(&mut self, name: String, entry: ParsedJarEntry) -> Result<()> {
		self.entries.insert(name, entry);
		Ok(())
	}

	pub(crate) fn from_jar(jar: &impl Jar) -> Result<ParsedJar> {
		let mut result = ParsedJar {
			entries: IndexMap::new(),
		};

		for entry in jar.entries()? {
			let path = entry.name().to_owned();
			let entry = entry.to_parsed_jar_entry()?;

			result.entries.insert(path, entry);
		}

		Ok(result)
	}

	pub fn to_mem(self) -> Result<MemJar> {
		let mut zip_out = ZipWriter::new(Cursor::new(Vec::new()));

		for entry in self.entries {
			let path = entry.name();
			let attr = entry.attrs();

			let mut x = path;
			while let Some((left, _)) = x.rsplit_once('/') {
				if !left.is_empty() {
					zip_out.add_directory(left, FileOptions::default())?;
				}
				x = left;
			}

			let data: Option<Vec<u8>> = match entry.1.clone() {
				ParsedJarEntry::Class { class, .. } => Some(class.write()?),
				ParsedJarEntry::Other { data, .. } => Some(data),
				ParsedJarEntry::Dir { .. } => None,
			};

			if let Some(data) = data {
				zip_out.start_file(path, FileOptions::default().last_modified_time(attr.mtime))?;
				// TODO: set the files ctime, atime, mtime to the ones from the file read
				zip_out.write_all(&data)?;
			}
		}

		let vec = zip_out.finish()?.into_inner();

		Ok(MemJar::new_unnamed(vec))
	}
}

#[derive(Clone)]
pub enum ParsedJarEntry {
	Class {
		class: ClassRepr,
		attr: BasicFileAttributes,
	},
	Other {
		data: Vec<u8>,
		attr: BasicFileAttributes,
	},
	Dir {
		attr: BasicFileAttributes,
	},
}

impl JarEntry for (String, ParsedJarEntry) {
	fn is_dir(&self) -> bool {
		matches!(self.1, ParsedJarEntry::Dir { .. })
	}

	fn name(&self) -> &str {
		&self.0
	}

	fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V> {
		match self.1 {
			ParsedJarEntry::Class { class, .. } => {
				class.visit_as_class(visitor)
			},
			_ => Ok(visitor),
		}
	}

	fn attrs(&self) -> BasicFileAttributes {
		match &self.1 {
			ParsedJarEntry::Class { attr, .. } => attr.clone(),
			ParsedJarEntry::Other { attr, .. } => attr.clone(),
			ParsedJarEntry::Dir { attr, .. } => attr.clone(),
		}
	}

	fn to_parsed_jar_entry(self) -> Result<ParsedJarEntry> {
		Ok(self.1)
	}
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
			ParsedJarEntry::Class { attr, .. } => attr.clone(),
			ParsedJarEntry::Other { attr, .. } => attr.clone(),
			ParsedJarEntry::Dir { attr, .. } => attr.clone(),
		}
	}

	fn to_parsed_jar_entry(self) -> Result<ParsedJarEntry> {
		Ok(self.1.clone())
	}
}

#[derive(Clone)]
pub(crate) enum ClassRepr {
	Parsed {
		class: ClassFile,
	},
	Vec {
		data: Vec<u8>,
	}
}

impl ClassRepr {
	pub(crate) fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V> {
		match self {
			ClassRepr::Parsed { class } => {
				class.clone().accept(visitor)
			},
			ClassRepr::Vec { data } => {
				duke::read_class_multi(&mut Cursor::new(data), visitor)
			},
		}
	}

	pub(crate) fn read(self) -> Result<ClassFile> {
		match self {
			ClassRepr::Parsed { class } => Ok(class),
			ClassRepr::Vec { data } => duke::read_class(&mut Cursor::new(data)),
		}
	}

	pub(crate) fn write(self) -> Result<Vec<u8>> {
		match self {
			ClassRepr::Parsed { class } => {
				let mut buf = Vec::new();
				duke::write_class(&mut buf, &class)?;
				Ok(buf)
			},
			ClassRepr::Vec { data } => Ok(data),
		}
	}

	pub(crate) fn action(self, f: impl FnOnce(ClassFile) -> Result<ClassFile>) -> Result<ClassRepr> {
		let class = self.read()?;
		let class = f(class)?;
		Ok(ClassRepr::Parsed { class })
	}

	pub(crate) fn edit(self, f: impl FnOnce(&mut ClassFile)) -> Result<ClassRepr> {
		self.action(|mut class| {
			f(&mut class);
			Ok(class)
		})
	}
}

impl From<ClassFile> for ClassRepr {
	fn from(class: ClassFile) -> Self {
		ClassRepr::Parsed { class }
	}
}
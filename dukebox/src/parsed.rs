use std::fs::File;
use std::io::{Cursor, Seek, Write};
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use zip::write::FileOptions;
use zip::{DateTime, ZipWriter};
use crate::{BasicFileAttributes, IsClass, IsOther, Jar, JarEntry, JarEntryEnum, OpenedJar};
use crate::lazy_duke::ClassRepr;
use crate::zip::mem::UnnamedMemJar;

#[derive(Debug, Default)]
pub struct ParsedJar<Class, Other> {
	pub entries: IndexMap<String, ParsedJarEntry<Class, Other>>,
}

#[derive(Debug)]
pub struct ParsedJarEntry<Class, Other> {
	pub attr: BasicFileAttributes,
	pub content: JarEntryEnum<Class, Other>,
}

impl<Class, Other> Jar for ParsedJar<Class, Other>
where
	Class: IsClass,
	Other: IsOther,
	for<'a> &'a Class: IsClass,
	for<'a> &'a Other: IsOther,
{
	type Opened<'a> = &'a ParsedJar<Class, Other> where Self: 'a;

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

impl<'this, Class, Other> OpenedJar for &'this ParsedJar<Class, Other>
where
	for<'a> &'a Class: IsClass,
	for<'a> &'a Other: IsOther,
{
	type EntryKey = usize;

	type Entry<'a> = (&'a String, &'a ParsedJarEntry<Class, Other>) where Self: 'a;

	fn entry_keys(&self) -> impl Iterator<Item=Self::EntryKey> + 'static {
		0..self.entries.len()
	}

	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>> {
		self.entries.get_index(key)
			.with_context(|| anyhow!("no entry for index {key:?}"))
	}

	fn names(&self) -> impl Iterator<Item=(Self::EntryKey, &'_ str)> {
		self.entries.keys().map(|x| x.as_str()).enumerate()
	}

	fn by_name(&mut self, name: &str) -> Result<Option<Self::Entry<'_>>> {
		Ok(self.entries.get_key_value(name))
	}
}

impl<Class, Other> ParsedJar<Class, Other>
	where
		Class: IsClass,
		Other: IsOther,
{
	pub(crate) fn from_jar(jar: &impl Jar) -> Result<ParsedJar<ClassRepr, Vec<u8>>> {
		let mut jar = jar.open()?;

		let mut result = ParsedJar {
			entries: IndexMap::new(),
		};

		for key in jar.entry_keys() {
			let entry = jar.by_entry_key(key)?;

			let name = entry.name().to_owned();

			let entry = ParsedJarEntry {
				attr: entry.attrs(),
				content: entry.to_jar_entry_enum()?
					.map_both(|class| class.into_class_repr(), |other| other.get_data_owned()),
			};

			result.entries.insert(name, entry);
		}

		Ok(result)
	}

	fn add_dirs_to<W: Write + Seek>(path: &str, mut zip_out: ZipWriter<W>) -> Result<()> {
		let mut x = path;
		while let Some((left, _)) = x.rsplit_once('/') {
			if !left.is_empty() {
				let options = FileOptions::<()>::default()
					.last_modified_time(DateTime::default()); // otherwise we'd get the current time
				// TODO: awaiting lib support: set the ctime, atime, mtime to the same value
				zip_out.add_directory(left, options)?;
			}
			x = left;
		}
		Ok(())
	}

	fn write<W: Write + Seek>(&self, writer: W) -> Result<W> {
		let mut zip_out = ZipWriter::new(writer);

		for (name, entry) in &self.entries {
			use JarEntryEnum::*;
			match &entry.content {
				Dir => zip_out.add_directory(name.as_str(), entry.attr.to_file_options())?,
				Class(class) => {
					let data = class.write()?;

					zip_out.start_file(name.as_str(), entry.attr.to_file_options())?;
					zip_out.write_all(data.as_ref())?;
				},
				Other(data) => {
					zip_out.start_file(name.as_str(), entry.attr.to_file_options())?;
					zip_out.write_all(data.get_data())?;
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

impl<'name, 'entry, Class, Other> JarEntry for (&'name String, &'entry ParsedJarEntry<Class, Other>)
where
	for<'a> &'a Class: IsClass,
	for<'a> &'a Other: IsOther,
{
	fn name(&self) -> &str {
		self.0
	}

	fn attrs(&self) -> BasicFileAttributes {
		self.1.attr
	}

	type Class = &'entry Class;
	type Other = &'entry Other;
	fn to_jar_entry_enum(self) -> Result<JarEntryEnum<Self::Class, Self::Other>> {
		use JarEntryEnum::*;
		Ok(match &self.1.content {
			Dir => Dir,
			Class(class) => Class(class),
			Other(other) => Other(other),
		})
	}
}
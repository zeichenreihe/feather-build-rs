use anyhow::{anyhow, Context, Result};
use std::io::{Cursor, Read, Seek};
use std::ops::Range;
use zip::read::ZipFile;
use zip::result::ZipError;
use zip::ZipArchive;
use duke::visitor::MultiClassVisitor;
use crate::{BasicFileAttributes, JarEntry, OpenedJar};
use crate::parsed::ParsedJarEntry;

pub mod mem;
pub mod file;
pub mod both;

impl<R: Read + Seek> OpenedJar for ZipArchive<R> {
	type Entry<'a> = ZipFile<'a> where Self: 'a;


	type EntryKey = usize;
	type EntryKeyIter = Range<usize>;

	fn entry_keys(&self) -> Self::EntryKeyIter {
		0..self.len()
	}

	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>> {
		self.by_index(key).context("")
	}


	type Name<'a> = &'a str where Self: 'a;
	type NameIter<'a> = std::vec::IntoIter<&'a str> where Self: 'a;

	fn names(&self) -> Self::NameIter<'_> {
		self.file_names().collect::<Vec<_>>().into_iter()
	}

	fn by_name(&mut self, name: &str) -> Result<Option<Self::Entry<'_>>> {
		match self.by_name(name) {
			Ok(file) => Ok(Some(file)),
			Err(e) => match e {
				ZipError::FileNotFound => Ok(None),
				e => Err(anyhow!("could not get file {name} from zip: {e}")),
			}
		}
	}
}

impl JarEntry for ZipFile<'_> {
	fn is_dir(&self) -> bool {
		ZipFile::is_dir(self)
	}

	fn name(&self) -> &str {
		ZipFile::name(self)
	}

	fn visit_as_class<V: MultiClassVisitor>(mut self, visitor: V) -> Result<V> {
		let mut vec = Vec::new();
		self.read_to_end(&mut vec)?;

		let mut reader = Cursor::new(vec);

		duke::read_class_multi(&mut reader, visitor)
	}

	fn attrs(&self) -> BasicFileAttributes {
		BasicFileAttributes::new(self.last_modified(), self.extra_data_fields())
	}

	fn to_vec(mut self) -> Result<Vec<u8>> {
		let mut vec = Vec::new();
		self.read_to_end(&mut vec)?;
		Ok(vec)
	}

	fn to_parsed_jar_entry(mut self) -> Result<ParsedJarEntry> {
		let attr = self.attrs();

		Ok(if self.is_dir() {
			ParsedJarEntry::Dir { attr }
		} else {
			let mut data = Vec::new();
			self.read_to_end(&mut data)?;

			if self.is_class() {
				ParsedJarEntry::Class { attr, class: data.into() }
			} else {
				ParsedJarEntry::Other { attr, data }
			}
		})
	}
}

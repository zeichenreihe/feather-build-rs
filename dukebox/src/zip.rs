use anyhow::{anyhow, Context, Result};
use std::io::{Cursor, Read, Seek};
use zip::read::ZipFile;
use zip::result::ZipError;
use zip::ZipArchive;
use duke::tree::class::ClassFile;
use duke::visitor::MultiClassVisitor;
use crate::{BasicFileAttributes, IsClass, IsOther, JarEntry, JarEntryEnum, OpenedJar};
use crate::lazy_duke::ClassRepr;

pub mod mem;
pub mod file;

impl<R: Read + Seek> OpenedJar for ZipArchive<R> {
	type EntryKey = usize;

	type Entry<'a> = ZipFile<'a> where Self: 'a;

	fn entry_keys(&self) -> impl Iterator<Item=Self::EntryKey> + 'static {
		0..self.len()
	}

	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>> {
		self.by_index(key).context("")
	}

	fn names(&self) -> impl Iterator<Item=(Self::EntryKey, &'_ str)> {
		//TODO: suggest to `zip` crate to expose the `files` map of the `ZipArchive`, because I want to have both the names and the
		// zip file indices to improve performance (as then I don't need to get with a string from a map!)

		// This unwrap is fine, as all the indices are within bounds
		#[allow(clippy::unwrap_used)]
		(0..self.len()).map(|x| (x, self.name_for_index(x).unwrap()))
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
	fn name(&self) -> &str {
		ZipFile::name(self)
	}

	fn attrs(&self) -> BasicFileAttributes {
		BasicFileAttributes::new(self.last_modified(), self.extra_data_fields())
	}

	type Class = FromZipClass;
	type Other = FromZipOther;
	fn to_jar_entry_enum(mut self) -> Result<JarEntryEnum<Self::Class, Self::Other>> {
		Ok(if self.is_dir() {
			JarEntryEnum::Dir
		} else {
			let mut data = Vec::new();
			self.read_to_end(&mut data)?;

			if !self.is_dir() && self.name().ends_with(".class") {
				JarEntryEnum::Class(FromZipClass { inner: data })
			} else {
				JarEntryEnum::Other(FromZipOther { inner: data })
			}
		})
	}
}

pub struct FromZipClass {
	inner: Vec<u8>,
}

impl IsClass for FromZipClass {
	fn read(self) -> Result<ClassFile> {
		duke::read_class(&mut Cursor::new(self.inner))
	}

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M> {
		duke::read_class_multi(&mut Cursor::new(self.inner), visitor)
	}

	type Written<'a> = &'a [u8] where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>> {
		Ok(&self.inner)
	}

	fn into_class_repr(self) -> ClassRepr {
		ClassRepr::Vec { data: self.inner }
	}
}

pub struct FromZipOther {
	inner: Vec<u8>,
}

impl IsOther for FromZipOther {
	fn get_data(&self) -> &[u8] {
		&self.inner
	}
	fn get_data_owned(self) -> Vec<u8> {
		self.inner
	}
}

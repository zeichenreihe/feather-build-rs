use anyhow::{anyhow, Context, Result};
use std::io::{Read, Seek};
use log::info;
use zip::read::ZipFile;
use zip::result::ZipError;
use zip::{ExtraField, ZipArchive};
use crate::storage::{BasicFileAttributes, JarEntry, JarEntryEnum, OpenedJar, VecClass};

impl<R: Read + Seek> OpenedJar for ZipArchive<R> {
	type EntryKey = usize;

	type Entry<'a> = ZipFile<'a> where Self: 'a;

	fn entry_keys(&self) -> impl Iterator<Item=Self::EntryKey> + 'static {
		0..self.len()
	}

	fn by_entry_key(&mut self, key: Self::EntryKey) -> Result<Self::Entry<'_>> {
		self.by_index(key).context("") // TODO: msg
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
		let last_modified = self.last_modified();
		let extended_timestamp = self.extra_data_fields()
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

	type Class = VecClass;
	type Other = Vec<u8>;
	fn to_jar_entry_enum(mut self) -> Result<JarEntryEnum<Self::Class, Self::Other>> {
		Ok(if self.is_dir() {
			JarEntryEnum::Dir
		} else {
			let data = {
				let capacity = self.size()
					.try_into()
					.unwrap_or_else(|x| {
						info!("size of zip file {:?} doesn't fit in usize: {x:?}", self.name());
						0
					});
				let mut data = Vec::with_capacity(capacity);
				self.read_to_end(&mut data)?;
				data
			};

			if self.name().ends_with(".class") {
				JarEntryEnum::Class(VecClass(data))
			} else {
				JarEntryEnum::Other(data)
			}
		})
	}
}

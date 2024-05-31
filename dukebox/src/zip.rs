use anyhow::Result;
use std::io::{Cursor, Read, Seek};
use zip::ZipArchive;
use duke::visitor::MultiClassVisitor;
use crate::{BasicFileAttributes, Jar, JarEntry};

pub mod mem;
pub mod file;
pub mod both;

pub(crate) trait JarFromReader {
	type Reader<'a>: Read + Seek + 'a where Self: 'a;

	fn open(&self) -> Result<Self::Reader<'_>>;

	fn for_each_class(&self, mut f: impl FnMut(Cursor<Vec<u8>>) -> Result<()>) -> Result<()> {
		let reader = self.open()?;
		let mut zip = ZipArchive::new(reader)?;

		for index in 0..zip.len() {
			let mut file = zip.by_index(index)?;
			if file.name().ends_with(".class") {

				let mut vec = Vec::new();
				file.read_to_end(&mut vec)?;
				let reader = Cursor::new(vec);

				f(reader)?;
			}
		}

		Ok(())
	}
}

impl<T: JarFromReader> Jar for T {
	type Entry<'a> = ZipFileEntry where Self: 'a;
	type Iter<'a> = std::vec::IntoIter<ZipFileEntry> where Self: 'a;

	fn entries<'a: 'b, 'b>(&'a self) -> Result<Self::Iter<'b>> {
		let reader = self.open()?;
		let mut zip = ZipArchive::new(reader)?;

		let mut out = Vec::with_capacity(zip.len());
		for index in 0..zip.len() {
			let mut file = zip.by_index(index)?;
			out.push(ZipFileEntry {
				is_dir: file.is_dir(),
				name: file.name().to_owned(),
				vec: { let mut vec = Vec::new(); file.read_to_end(&mut vec)?; vec },
				attrs: BasicFileAttributes { // TODO: implement reading the more exact file modification times from the extra data of the zip file
					mtime: file.last_modified(),
					atime: (),
					ctime: (),
				},
			});
		}

		Ok(out.into_iter())
	}
}

pub struct ZipFileEntry {
	is_dir: bool,
	name: String,
	vec: Vec<u8>,
	attrs: BasicFileAttributes,
}

impl JarEntry for ZipFileEntry {
	fn is_dir(&self) -> bool {
		self.is_dir
	}

	fn name(&self) -> &str {
		&self.name
	}

	fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V> {
		let mut reader = Cursor::new(&self.vec);

		duke::read_class_multi(&mut reader, visitor)
	}

	fn get_vec(&self) -> Vec<u8> {
		self.vec.clone()
	}

	fn attrs(&self) -> BasicFileAttributes {
		self.attrs.clone()
	}
}

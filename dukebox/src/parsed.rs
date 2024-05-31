use std::io::Cursor;
use anyhow::Result;
use indexmap::IndexMap;
use duke::tree::class::ClassFile;
use duke::visitor::MultiClassVisitor;
use crate::{BasicFileAttributes, Jar, JarEntry};

struct ParsedJar {
	entries: IndexMap<String, ParsedJarEntry>,
}

impl Jar for ParsedJar {
	type Entry<'a> = (&'a String, &'a ParsedJarEntry);
	type Iter<'a> = indexmap::map::Iter<'a, String, ParsedJarEntry> where Self: 'a;

	fn entries<'a: 'b, 'b>(&'a self) -> Result<Self::Iter<'b>> {
		Ok(self.entries.iter())
	}
}

enum ParsedJarEntry {
	Class {
		class: ClassFile,
	},
	ClassAsVec {
		data: Vec<u8>,
	},
	Other {
		data: Vec<u8>,
	},
	Dir {
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
				class.clone().accept(visitor)
			},
			ParsedJarEntry::ClassAsVec { data, .. } => {
				duke::read_class_multi(&mut Cursor::new(data), visitor)
			},
			_ => Ok(visitor),
		}
	}

	fn get_vec(&self) -> Vec<u8> {
		todo!()
	}

	fn attrs(&self) -> BasicFileAttributes {
		todo!()
	}
}

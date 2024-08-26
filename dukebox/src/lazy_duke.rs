use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use anyhow::Result;
use duke::tree::class::ClassFile;
use duke::visitor::MultiClassVisitor;

/// A lazily read [`ClassFile`].
#[derive(Clone)]
pub enum ClassRepr {
	Parsed {
		class: ClassFile,
	},
	Vec {
		data: Vec<u8>,
	},
}

impl Debug for ClassRepr {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			ClassRepr::Parsed { class } => f.debug_struct("Parsed").field("class", &class).finish(),
			ClassRepr::Vec { data } => f.debug_struct("Vec").field("size", &data.len()).finish_non_exhaustive(),
		}
	}
}

impl ClassRepr {
	pub(crate) fn visit_as_class<V: MultiClassVisitor>(self, visitor: V) -> Result<V> {
		match self {
			ClassRepr::Parsed { class } => {
				class.accept(visitor)
			},
			ClassRepr::Vec { data } => {
				duke::read_class_multi(&mut Cursor::new(data), visitor)
			},
		}
	}

	pub fn read(self) -> Result<ClassFile> {
		match self {
			ClassRepr::Parsed { class } => Ok(class),
			ClassRepr::Vec { data } => duke::read_class(&mut Cursor::new(data)),
		}
	}

	pub(crate) fn write_from_ref(&self) -> Result<Cow<[u8]>> {
		match self {
			ClassRepr::Parsed { class } => {
				let mut buf = Vec::new();
				duke::write_class(&mut buf, class)?;
				Ok(Cow::Owned(buf))
			},
			ClassRepr::Vec { data } => Ok(Cow::Borrowed(data)),
		}
	}

	pub(crate) fn write(&self) -> Result<Cow<'_, [u8]>> {
		match self {
			ClassRepr::Parsed { class } => {
				let mut buf = Vec::new();
				duke::write_class(&mut buf, class)?;
				Ok(Cow::Owned(buf))
			},
			ClassRepr::Vec { data } => Ok(Cow::Borrowed(data)),
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

impl From<Vec<u8>> for ClassRepr {
	fn from(data: Vec<u8>) -> Self {
		ClassRepr::Vec { data }
	}
}
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use anyhow::Result;
use duke::tree::class::ClassFile;
use duke::visitor::MultiClassVisitor;
use crate::IsClass;

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

impl IsClass for ClassRepr {
	fn read(self) -> Result<ClassFile> {
		match self {
			ClassRepr::Parsed { class } => Ok(class),
			ClassRepr::Vec { data } => duke::read_class(&mut Cursor::new(data)),
		}
	}

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M> {
		match self {
			ClassRepr::Parsed { class } => class.accept(visitor),
			ClassRepr::Vec { data } => duke::read_class_multi(&mut Cursor::new(data), visitor),
		}
	}

	type Written<'a> = Cow<'a, [u8]> where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>> {
		match self {
			ClassRepr::Parsed { class } => {
				let mut buf = Vec::new();
				duke::write_class(&mut buf, class)?;
				Ok(Cow::Owned(buf))
			},
			ClassRepr::Vec { data } => Ok(Cow::Borrowed(data)),
		}
	}

	fn into_class_repr(self) -> ClassRepr {
		self
	}
}

impl IsClass for &ClassRepr {
	fn read(self) -> Result<ClassFile> {
		match self {
			ClassRepr::Parsed { class } => Ok(class.clone()),
			ClassRepr::Vec { data } => duke::read_class(&mut Cursor::new(data)),
		}
	}

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M> {
		match self {
			ClassRepr::Parsed { class } => class.clone().accept(visitor),
			ClassRepr::Vec { data } => duke::read_class_multi(&mut Cursor::new(data), visitor),
		}
	}

	type Written<'a> = Cow<'a, [u8]> where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>> {
		match self {
			ClassRepr::Parsed { class } => {
				let mut buf = Vec::new();
				duke::write_class(&mut buf, class)?;
				Ok(Cow::Owned(buf))
			},
			ClassRepr::Vec { data } => Ok(Cow::Borrowed(data)),
		}
	}

	fn into_class_repr(self) -> ClassRepr {
		self.clone()
	}
}
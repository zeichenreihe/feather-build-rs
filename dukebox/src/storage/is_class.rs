use std::io::Cursor;
use anyhow::Result;
use duke::tree::class::ClassFile;
use duke::visitor::MultiClassVisitor;
use crate::storage::ClassRepr;

/// Represents a class entry.
///
/// A class entry can be [`read`][IsClass::read] into a [`ClassFile`].
/// It can also be accepted into a visitor with [`IsClass::visit`].
/// Lastly, it can be written into something that can be looked at as a `&[u8]`, with
/// the [IsClass::write] method.
///
/// `Vec<u8>` does not implement [`IsClass`], use [`VecClass`] instead.
pub trait IsClass {
	fn read(self) -> Result<ClassFile>;

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M>;

	type Written<'a>: AsRef<[u8]> where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>>;

	// TODO: remove?
	fn into_class_repr(self) -> ClassRepr;
}

/// A class stored in a `Vec<u8>`.
///
/// The new-type is here to prevent mis-use of other `Vec<u8>` as classes.
pub struct VecClass(pub Vec<u8>);

impl IsClass for VecClass {
	fn read(self) -> Result<ClassFile> {
		duke::read_class(&mut Cursor::new(self.0))
	}

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M> {
		duke::read_class_multi(&mut Cursor::new(self.0), visitor)
	}

	type Written<'a> = &'a [u8] where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>> {
		Ok(&self.0)
	}

	fn into_class_repr(self) -> ClassRepr {
		ClassRepr::Vec { data: self.0 }
	}
}

impl IsClass for ClassFile {
	fn read(self) -> Result<ClassFile> {
		Ok(self)
	}

	fn visit<M: MultiClassVisitor>(self, visitor: M) -> Result<M> {
		self.accept(visitor)
	}

	type Written<'a> = Vec<u8> where Self: 'a;
	fn write(&self) -> Result<Self::Written<'_>> {
		let mut buf = Vec::new();
		duke::write_class(&mut buf, self)?;
		Ok(buf)
	}

	fn into_class_repr(self) -> ClassRepr {
		ClassRepr::Parsed { class: self }
	}
}

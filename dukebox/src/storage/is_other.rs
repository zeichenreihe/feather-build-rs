use std::borrow::Cow;

/// Represents a normal file entry.
///
/// Note that this is not a class.
pub trait IsOther {
	fn get_data(&self) -> &[u8];
	fn get_data_owned(self) -> Vec<u8>;
	fn as_cow(&self) -> Cow<[u8]> {
		self.get_data().into()
	}
}

impl IsOther for Vec<u8> {
	fn get_data(&self) -> &[u8] {
		self
	}
	fn get_data_owned(self) -> Vec<u8> {
		self
	}
}

impl<T: AsRef<[u8]>> IsOther for &'_ T {
	fn get_data(&self) -> &[u8] {
		self.as_ref()
	}
	fn get_data_owned(self) -> Vec<u8> {
		self.as_ref().to_owned()
	}
}

impl IsOther for Cow<'_, [u8]> {
	fn get_data(&self) -> &[u8] {
		self
	}
	fn get_data_owned(self) -> Vec<u8> {
		self.into_owned()
	}
}

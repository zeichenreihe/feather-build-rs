
/// Assuming a `struct Foo(Cow<'static, str>);`, creates implementations for
/// - `From<String> for Foo`, `From<&str> for Foo`, and
/// - `From<Foo> for String`, `From<&'a Foo> for &'a str`, and
/// - `.as_mut_string(&mut self) -> &mut String`, `.as_str(&self) -> &str` and
/// - `AsRef<str> for Foo`.
macro_rules! from_impl_for_string_and_str {
	($name:ident) => {
		impl From<String> for $name {
			fn from(value: String) -> Self {
				$name(value.into())
			}
		}

		impl From<&'static str> for $name {
			fn from(value: &'static str) -> Self {
				$name(value.into())
			}
		}

		impl From<$name> for String {
			fn from(value: $name) -> Self {
				value.0.into_owned()
			}
		}

		impl<'a> From<&'a $name> for &'a str {
			fn from(value: &'a $name) -> Self {
				&value.0
			}
		}

		impl $name {
			pub fn as_mut_string(&mut self) -> &mut String {
				self.0.to_mut()
			}

			pub fn as_str(&self) -> &str {
				&self.0
			}
		}

		impl AsRef<str> for $name {
			fn as_ref(&self) -> &str {
				self.as_str()
			}
		}
	}
}

/// Assuming a `struct Foo(Cow<'static, str>);`, creates implementations for
/// - `PartialEq<&str> for Foo`, `PartialEq<str> for Foo`, and
/// - `PartialEq<Foo> for &str`, `PartialEq<Foo> for str`.
macro_rules! partial_eq_impl_for_str {
	($name:ident) => {
		impl PartialEq<&str> for $name {
			fn eq(&self, other: &&str) -> bool {
				self.0 == *other
			}
		}

		impl PartialEq<str> for $name {
			fn eq(&self, other: &str) -> bool {
				self.0 == other
			}
		}

		impl PartialEq<$name> for &str {
			fn eq(&self, other: &$name) -> bool {
				*self == other.0
			}
		}

		impl PartialEq<$name> for str {
			fn eq(&self, other: &$name) -> bool {
				self == other.0
			}
		}
	}
}

pub(crate) use from_impl_for_string_and_str;
pub(crate) use partial_eq_impl_for_str;

macro_rules! make_string_str_like {
	(
		$( #[$owned_doc:meta] )*
		$owned:ident ,
		$( #[$borrowed_doc:meta] )*
		$borrowed:ident $(,)?
	) => {
		$( #[$owned_doc] )*
		#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
		pub struct $owned(String);

		$( #[$borrowed_doc] )*
		#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
		#[repr(transparent)]
		pub struct $borrowed(str);

		impl $owned {
			pub fn as_slice(&self) -> &$borrowed {
				$borrowed::from_str(&self.0)
			}
		}

		impl $borrowed {
			pub fn as_str(&self) -> &str {
				&self.0
			}

			pub const fn from_str<'a>(s: &'a str) -> &'a $borrowed {
				// SAFETY: &'a $borrowed and &'a str have the same layout.
				// TODO: give this to other people and ask if it's fine!
				let s: &'a $borrowed = unsafe { std::mem::transmute(s) };
				s
			}
		}

		impl AsRef<str> for $borrowed {
			fn as_ref(&self) -> &str {
				self.as_str()
			}
		}

		impl AsRef<str> for $owned {
			fn as_ref(&self) -> &str {
				self.as_str()
			}
		}

		impl std::borrow::Borrow<$borrowed> for $owned {
			fn borrow(&self) -> &$borrowed {
				self.as_slice()
			}
		}

		impl std::ops::Deref for $owned {
			type Target = $borrowed;

			fn deref(&self) -> &Self::Target {
				self.as_slice()
			}
		}

		impl<'a> From<&'a str> for &'a $borrowed {
			fn from(value: &'a str) -> Self {
				$borrowed::from_str(value)
			}
		}
		impl From<String> for $owned {
			fn from(value: String) -> Self {
				$owned(value)
			}
		}
		impl From<&str> for $owned {
			fn from(value: &str) -> Self {
				$owned(value.to_owned())
			}
		}

		impl std::hash::Hash for $owned where $borrowed: std::hash::Hash {
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				std::hash::Hash::hash(self.as_slice(), state)
			}
		}

		// PartialEq between $borrowed and $owned
		impl PartialEq<$borrowed> for $owned {
			fn eq(&self, other: &$borrowed) -> bool {
				self.as_str() == other.as_str()
			}
		}
		impl PartialEq<$owned> for $borrowed {
			fn eq(&self, other: &$owned) -> bool {
				self.as_str() == other.as_str()
			}
		}
		impl<'a> PartialEq<&'a $borrowed> for $owned {
			fn eq(&self, other: &&'a $borrowed) -> bool {
				self.as_str() == other.as_str()
			}
		}
		impl<'a> PartialEq<$owned> for &'a $borrowed {
			fn eq(&self, other: &$owned) -> bool {
				self.as_str() == other.as_str()
			}
		}

		impl std::borrow::ToOwned for $borrowed {
			type Owned = $owned;

			fn to_owned(&self) -> Self::Owned {
				$owned(self.0.to_owned())
			}
		}
	}
}

pub(crate) use make_string_str_like;
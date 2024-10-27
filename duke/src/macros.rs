macro_rules! make_display {
	($owned:ident, $borrowed:ident) => {
		impl Display for $owned {
			fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
				Display::fmt(self.as_slice(), f)
			}
		}
		impl Display for $borrowed {
			fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
				let inner: &java_string::JavaStr = self.as_inner();
				inner.as_str()
					.map_err(|_| std::fmt::Error)
					.and_then(|s| write!(f, "{}", s))
			}
		}
	}
}

/// Creates implementations for [String]/[str] like types.
///
/// You need to have a function
/// ```no_run
/// # struct Owned;
/// # struct BorrowedInner;
/// # struct SomeErrorType;
/// impl Owned {
///     fn check_valid(inner: &BorrowedInner) -> Result<(), SomeErrorType> {
///         // ...
/// # Ok(())
///     }
/// }
/// ```
/// that checks if the contents are valid.
macro_rules! make_string_str_like {
	(
		$( #[$owned_doc:meta] )*
		$owned_vis:vis $owned:ident ( $owned_inner:ty ) ;
		$( #[$borrowed_doc:meta] )*
		$borrowed_vis:vis $borrowed:ident ( $borrowed_inner:ty );
	) => {
		$( #[$owned_doc] )*
		#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
		$owned_vis struct $owned($owned_inner);

		$( #[$borrowed_doc] )*
		#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
		#[repr(transparent)]
		$borrowed_vis struct $borrowed($borrowed_inner);

		impl $owned {
			pub fn as_slice(&self) -> &$borrowed {
				self
			}

			pub fn into_inner(self) -> $owned_inner {
				self.0
			}
            #[doc = concat!("Constructs [`", stringify!($owned), "`] from [`",
				stringify!($owned_inner), "`] without checking any content.")]
			///
			/// # Safety
			#[doc = concat!("`s` must only contain valid contents for [`", stringify!($owned), "`]. See [`",
				stringify!($owned), "::check_valid`] for the concrete values that are allowed.")]
			pub const unsafe fn from_inner_unchecked(s: $owned_inner) -> $owned {
				$owned(s)
			}

			/// Checks if a given value is valid for being represented by this type.
			///
			#[doc = concat!("This also applies to the slice type, [`", stringify!($borrowed), "`].")]
			///
			/// See [`Self::check_valid`] for the specification on what's valid.
			pub fn is_valid(inner: &$borrowed_inner) -> bool {
				let result: Result<(), _> = Self::check_valid(inner);
				result.is_ok()
			}
		}

		impl $borrowed {
			pub fn as_inner(&self) -> &$borrowed_inner {
				&self.0
			}

            #[doc = concat!("Constructs [`&", stringify!($borrowed), "`][", stringify!($borrowed),
				"] from [`&", stringify!($borrowed_inner), "`][", stringify!($borrowed_inner),
				"] without checking any content.")]
			///
			/// # Safety
            #[doc = concat!("`s` must only contain valid contents for [`", stringify!($borrowed), "`].")]
			/// See [`Self::check_valid`] for the concrete values that are allowed.
			#[allow(clippy::needless_lifetimes)] // TODO: we're more explicit about the lifetime, switch to expect
			pub const unsafe fn from_inner_unchecked<'a>(s: &'a $borrowed_inner) -> &'a $borrowed {
				// SAFETY: &'a $borrowed and &'a $borrowed_inner have the same layout.
				unsafe { std::mem::transmute::<&'a $borrowed_inner, &'a $borrowed>(s) }
			}
		}

		impl AsRef<$borrowed_inner> for $borrowed {
			fn as_ref(&self) -> &$borrowed_inner {
				&self.0
			}
		}

		impl AsRef<$borrowed_inner> for $owned {
			fn as_ref(&self) -> &$borrowed_inner {
				&self.0
			}
		}

		impl std::borrow::Borrow<$borrowed> for $owned
			where $owned_inner: std::borrow::Borrow<$borrowed_inner>
		{
			fn borrow(&self) -> &$borrowed {
				// SAFETY: $owned always contains valid content for $borrowed.
				unsafe { $borrowed::from_inner_unchecked(&self.0) }
			}
		}

		impl std::ops::Deref for $owned
			where $owned_inner: std::ops::Deref<Target=$borrowed_inner>
		{
			type Target = $borrowed;

			// deref may be inserted by the compiler at any time
			// therefore the call path must not use deref itself...
			fn deref(&self) -> &Self::Target {
				// SAFETY: $owned always contains valid content for $borrowed.
				unsafe { $borrowed::from_inner_unchecked(&self.0) }
			}
		}

		impl<'a> TryFrom<&'a $borrowed_inner> for &'a $borrowed {
			type Error = anyhow::Error;

			fn try_from(value: &'a $borrowed_inner) -> anyhow::Result<&'a $borrowed> {
				match $owned::check_valid(value) {
					// SAFETY: We just checked that `value` is valid for $owned.
					Ok(()) => Ok(unsafe { $borrowed::from_inner_unchecked(value) }),
					Err(e) => {
						use anyhow::Context;
						Err(e).with_context(|| anyhow::anyhow!("on value {value:?}"))
					},
				}
			}
		}
		impl<'a> TryFrom<$owned_inner> for $owned {
			type Error = anyhow::Error;

			fn try_from(value: $owned_inner) -> anyhow::Result<$owned> {
				match $owned::check_valid(&value) {
					// SAFETY: We just checked that `value` is valid for $owned.
					Ok(()) => Ok(unsafe { $owned::from_inner_unchecked(value) }),
					Err(e) => {
						use anyhow::Context;
						Err(e).with_context(|| anyhow::anyhow!("on value {value:?}"))
					},
				}
			}
		}
		impl<'a> TryFrom<&'a $borrowed_inner> for $owned {
			type Error = anyhow::Error;

			fn try_from(value: &'a $borrowed_inner) -> anyhow::Result<$owned> {
				<&$borrowed>::try_from(value).map(ToOwned::to_owned)
			}
		}

		impl<'a> From<&'a $borrowed> for &'a $borrowed_inner {
			fn from(value: &'a $borrowed) -> Self {
				&value.0
			}
		}
		impl From<$owned> for $owned_inner {
			fn from(value: $owned) -> Self {
				value.0
			}
		}

		impl std::hash::Hash for $owned
			where $borrowed: std::hash::Hash
		{
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				std::hash::Hash::hash(self.as_slice(), state)
			}
		}

		// PartialEq between $borrowed and $owned
		impl PartialEq<$borrowed> for $owned
			where $borrowed_inner: PartialEq
		{
			fn eq(&self, other: &$borrowed) -> bool {
				self.0 == other.0
			}
		}
		impl PartialEq<$owned> for $borrowed
			where $borrowed_inner: PartialEq
		{
			fn eq(&self, other: &$owned) -> bool {
				self.0 == other.0
			}
		}
		impl<'a> PartialEq<&'a $borrowed> for $owned
			where $borrowed_inner: PartialEq
		{
			fn eq(&self, other: &&'a $borrowed) -> bool {
				self.0 == other.0
			}
		}
		impl<'a> PartialEq<$owned> for &'a $borrowed
			where $borrowed_inner: PartialEq
		{
			fn eq(&self, other: &$owned) -> bool {
				self.0 == other.0
			}
		}

		impl std::borrow::ToOwned for $borrowed
			where $borrowed_inner: std::borrow::ToOwned<Owned=$owned_inner>
		{
			type Owned = $owned;

			fn to_owned(&self) -> Self::Owned {
				$owned(self.0.to_owned())
			}
		}
	}
}

pub(crate) use {make_display, make_string_str_like};
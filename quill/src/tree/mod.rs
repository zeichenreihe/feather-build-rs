use anyhow::Result;
use crate::tree::names::Names;

pub mod mappings;
pub mod mappings_diff;

pub trait NodeInfo<I> {
	fn get_node_info(&self) -> &I;
	fn get_node_info_mut(&mut self) -> &mut I;
	fn new(info: I) -> Self;
}

pub trait NodeJavadocInfo<I> {
	fn get_node_javadoc_info(&self) -> &I;
	fn get_node_javadoc_info_mut(&mut self) -> &mut I;
}

pub trait ToKey<K> {
	fn get_key(&self) -> Result<K>;
}

pub trait FromKey<K> {
	fn from_key(key: K) -> Self;
}

pub trait GetNames<const N: usize, T> {
	fn get_names(&self) -> &Names<N, T>;
	fn get_names_mut(&mut self) -> &mut Names<N, T>;
}

pub mod names {
	use std::fmt::{Debug, Formatter};
	use std::ops::{Index, IndexMut};
	use anyhow::{anyhow, bail, Context, Error, Result};
	use java_string::JavaStr;

	/// Describes a given namespace of a mapping tree.
	///
	/// This object exists to remove out of bounds checks. If this object exists from a given mapping (obtained via
	/// [`Namespaces::get_namespace`]), no range checking is necessary.
	#[derive(Debug, Copy, Clone, PartialEq)]
	pub struct Namespace<const N: usize>(pub(super) usize);

	impl<const N: usize> Namespace<N> {
		pub fn new(id: usize) -> Result<Namespace<N>> { // TODO: was pub(crate), not sure if this should be public api...
			if id >= N {
				bail!("cannot create namespace with id larger or equal to number of namespaces: {id} >= {N}");
			}
			Ok(Namespace(id))
		}
	}

	/// A struct storing the names of the namespaces.
	///
	/// Implements the [Index] and [IndexMut] traits for [Namespace].
	#[derive(Clone, PartialEq)]
	pub struct Namespaces<const N: usize> {
		names: [String; N]
	}

	impl<const N: usize> Index<Namespace<N>> for Namespaces<N> {
		type Output = String;

		fn index(&self, index: Namespace<N>) -> &Self::Output {
			&self.names[index.0]
		}
	}

	impl<const N: usize> IndexMut<Namespace<N>> for Namespaces<N> {
		fn index_mut(&mut self, index: Namespace<N>) -> &mut Self::Output {
			&mut self.names[index.0]
		}
	}

	impl<const N: usize> Namespaces<N> {
		pub(crate) fn names(&self) -> &[String; N] {
			&self.names
		}

		pub(crate) fn get_namespace(&self, name: &str) -> Result<Namespace<N>> {
			for (id, namespace) in self.names.iter().enumerate() {
				if namespace == name {
					return Ok(Namespace(id))
				}
			}
			bail!("cannot find namespace with name {name:?}, only got {self:?}");
		}

		/// Returns an error if the names of `self` aren't the names given in the argument.
		/// This can be used to check that after reading mappings, you have the correct namespaces in them.
		pub fn check_that(&self, names: [&str; N]) -> Result<()> {
			if self.names != names {
				bail!("expected namespaces {names:?}, got {self:?}");
			}
			Ok(())
		}

		pub(crate) fn reorder(&self, table: [Namespace<N>; N]) -> Namespaces<N> {
			Namespaces {
				names: table.map(|namespace| self[namespace].clone()),
			}
		}

		pub(crate) fn change_name(&mut self, namespace: Namespace<N>, from: &str, to: &str) -> Result<String> {
			if self[namespace] != from {
				bail!("can't change name of namespace {namespace:?} from {from:?} to {to:?}: old name doesn't match: {self:?}");
			}
			let old = std::mem::replace(&mut self[namespace], to.to_owned());

			Ok(old)
		}

		pub(crate) fn change_names(&mut self, from: [&str; N], to: [&str; N]) -> Result<()> {
			if self.names.as_slice() != from {
				bail!("cannot rename namespaces {self:?} to {to:?}: expected {from:?}");
			}

			self.names = to.map(String::from);
			Ok(())
		}
	}

	impl<const N: usize> Debug for Namespaces<N> {
		fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
			f.debug_list()
				.entries(&self.names)
				.finish()
		}
	}

	impl<const N: usize> TryFrom<[String; N]> for Namespaces<N> {
		type Error = Error;

		fn try_from(value: [String; N]) -> Result<Self> {
			if value.iter().any(|i| i.is_empty()) {
				bail!("found empty namespace name in {value:?}, every namespace name must be non-empty");
			}

			Ok(Namespaces { names: value })
		}
	}

	impl<const N: usize> From<Namespaces<N>> for [String; N] {
		fn from(value: Namespaces<N>) -> Self {
			value.names
		}
	}
	impl<'a, const N: usize> From<&'a Namespaces<N>> for &'a [String; N] {
		fn from(value: &'a Namespaces<N>) -> Self {
			&value.names
		}
	}

	/// A struct storing names for namespaces.
	///
	/// Invariants:
	/// the length `N` is at least `2`
	///
	/// Implements the [Index] and [IndexMut] traits for [Namespace].
	#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
	pub struct Names<const N: usize, T> {
		names: [Option<T>; N],
	}

	impl<const N: usize, T> Index<Namespace<N>> for Names<N, T> {
		type Output = Option<T>;

		fn index(&self, index: Namespace<N>) -> &Self::Output {
			&self.names[index.0]
		}
	}

	impl<const N: usize, T> IndexMut<Namespace<N>> for Names<N, T> {
		fn index_mut(&mut self, index: Namespace<N>) -> &mut Self::Output {
			&mut self.names[index.0]
		}
	}

	impl<const N: usize, T> Names<N, T> {
		pub(crate) fn none() -> Names<N, T> {
			let names = std::array::from_fn(|_| None);
			Names { names }
		}

		pub(crate) fn from_first_name(src: T) -> Names<N, T> {
			let mut names = std::array::from_fn(|_| None);
			if let Some(zero) = names.first_mut() {
				*zero = Some(src);
			}
			Names { names }
		}

		pub(crate) fn first_name(&self) -> Result<&T> where T: Debug {
			self.names.first().context("N = 0 is too small for having a name in first namespace")?
				.as_ref().with_context(|| anyhow!("no name for the first namespace: {self:?}"))
		}

		pub(crate) fn names(&self) -> &[Option<T>; N] {
			&self.names
		}

		pub(crate) fn get_mut_with_src(&mut self, namespace: Namespace<N>) -> Result<(Option<&T>, Option<&mut T>)> {
			if namespace.0 == 0 {
				bail!("can't make a mutable and immutable reference to the same member of the array at once");
			}

			match &mut self.names[..] {
				[] | [_] => bail!("not enough length: N = {N}"),
				[head, tail @ ..] => {
					Ok((head.as_ref(), tail[namespace.0 - 1].as_mut()))
				},
			}
		}

		pub(crate) fn reorder(&self, table: [Namespace<N>; N]) -> Result<Names<N, T>>
		where
			T: Clone + Debug,
		{
			let names = table.map(|namespace| self[namespace].clone());

			Ok(Names { names })
		}

		// TODO: doc
		/// Returns the old name. The return value is equivalent to `from.cloned()`.
		pub fn change_name(&mut self, namespace: Namespace<N>, from: Option<&T>, to: Option<&T>) -> Result<Option<T>>
			where
				T: Debug + Clone + PartialEq,
		{
			if namespace.0 == 0 {
				bail!("cannot edit the first namespace, as it needs to be kept in sync with the keys")
			}
			if self[namespace].as_ref() != from {
				bail!("can't change name in namespace {namespace:?} from {from:?} to {to:?}: old name doesn't match: {self:?}");
			}
			let old = std::mem::replace(&mut self[namespace], to.cloned());

			Ok(old)
		}
	}

	impl<const N: usize, T: Debug> Debug for Names<N, T> {
		fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
			f.debug_list()
				.entries(&self.names)
				.finish()
		}
	}

	/// Note that empty inputs are converted into `None`.
	///
	/// Emptiness is determined by the `AsRef<str>` implementation, and then `.is_empty()`.
	impl<const N: usize, T> From<[T; N]> for Names<N, T> where T: AsRef<JavaStr> {
		fn from(value: [T; N]) -> Self {
			let names = value.map(|x| if x.as_ref().is_empty() { None } else { Some(x) });

			Names { names }
		}
	}

	impl<const N: usize, T> TryFrom<[Option<T>; N]> for Names<N, T> where T: AsRef<JavaStr> + Debug {
		type Error = Error;

		fn try_from(value: [Option<T>; N]) -> Result<Self> {
			if value.iter().any(|i| i.as_ref().is_some_and(|i| i.as_ref().is_empty())) {
				bail!("cannot create names where an existing name is an empty string: {value:?}");
			}

			Ok(Names { names: value })
		}
	}

	impl<const N: usize, T> From<Names<N, T>> for [Option<T>; N] {
		fn from(value: Names<N, T>) -> Self {
			value.names
		}
	}

	impl<'a, const N: usize, T> From<&'a Names<N, T>> for &'a [Option<T>; N] {
		fn from(value: &'a Names<N, T>) -> Self {
			&value.names
		}
	}
}

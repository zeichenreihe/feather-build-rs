
pub mod mappings;
pub mod mappings_diff;

pub trait NodeInfo<I> {
	fn get_node_info(&self) -> &I;
	fn get_node_info_mut(&mut self) -> &mut I;
	fn new(info: I) -> Self;
}

pub trait ToKey<K> {
	fn get_key(&self) -> K;
}

pub(crate) trait FromKey<K> {
	fn from_key(key: K) -> Self;
}

pub mod names {
	use std::fmt::{Debug, Formatter};
	use std::ops::{Index, IndexMut};
	use anyhow::{bail, Result};

	/// Describes a given namespace of a mapping tree.
	///
	/// This object exists to remove out of bounds checks. If this object exists from a given mapping (obtained via
	/// [`Namespaces::get_namespace`]), no range checking is necessary.
	#[derive(Debug, Copy, Clone, PartialEq)]
	pub struct Namespace<const N: usize>(usize);

	impl<const N: usize> Namespace<N> {
		pub(crate) fn new(id: usize) -> Result<Namespace<N>> {
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
		pub(crate) fn names(&self) -> impl Iterator<Item=&String> {
			self.names.iter()
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

	impl<const N: usize> From<[String; N]> for Namespaces<N> {
		fn from(names: [String; N]) -> Self {
			Namespaces { names }
		}
	}

	impl<const N: usize> From<Namespaces<N>> for [String; N] {
		fn from(value: Namespaces<N>) -> Self {
			value.names
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
		/// Invariants:
		/// the first item, is always [Some]
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
		pub(crate) fn from_first_name(src: T) -> Names<N, T> {
			// TODO: check invariants
			let mut names = std::array::from_fn(|_| None);
			names[0] = Some(src);
			Names { names }
		}

		pub(crate) fn first_name(&self) -> &T {
			self.names[0].as_ref().unwrap()
		}

		pub(crate) fn names(&self) -> impl Iterator<Item=Option<&T>> {
			self.names.iter().map(|x| x.as_ref())
		}

		pub(crate) fn get_mut_with_src(&mut self, namespace: Namespace<N>) -> Result<(&T, Option<&mut T>)> {
			if namespace.0 == 0 {
				bail!("can't make a mutable and immutable reference to the same member of the array at once");
			}

			match &mut self.names[..] {
				[] | [_] => bail!("not enough length"),
				[head, tail @ ..] => {
					Ok((head.as_ref().unwrap(), tail[namespace.0 - 1].as_mut()))
				},
			}
		}

		pub(crate) fn reorder(&self, table: [Namespace<N>; N]) -> Result<Names<N, T>>
		where
			T: Clone + Debug,
		{
			let names = table.map(|namespace| self[namespace].clone());

			if names[0].is_none() {
				bail!("can't reorder names: {self:?} with {table:?} gives {names:?}: we don't get a name for the first namespace");
			}

			Ok(Names { names })
		}

		pub(crate) fn change_name(&mut self, namespace: Namespace<N>, from: Option<&T>, to: Option<&T>) -> Result<Option<T>>
			where
				T: Debug + Clone + PartialEq,
		{
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

	impl<const N: usize, T> From<[T; N]> for Names<N, T> {
		fn from(value: [T; N]) -> Self {
			Names { names: value.map(Some) }
		}
	}

	impl<const N: usize, T> From<[Option<T>; N]> for Names<N, T> {
		fn from(value: [Option<T>; N]) -> Self {
			// TODO: some size checks
			Names { names: value }
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

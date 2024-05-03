use std::fmt::Debug;
use anyhow::Result;
use std::hash::Hash;
use anyhow::bail;
use indexmap::IndexMap;
use indexmap::map::Entry;

pub(crate) mod mappings;
pub(crate) mod mappings_diff;
mod action;
pub(crate) mod descriptor;
pub(crate) mod access_flags;

/// Describes a given namespace of a mapping tree.
/// When this exists, the namespace it's from has the namespace stored in `.0`, and reading will not panic.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct Namespace<const N: usize>(usize);

mod names {
	use std::fmt::{Debug, Formatter};
	use anyhow::{Context, Result};
	use crate::tree::Namespace;

	/// A struct storing names for namespaces.
	#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
	pub(crate) struct Names<const N: usize> {
		names: [Option<String>; N],
	}

	impl<const N: usize> Names<N> {
		pub(crate) fn src(&self) -> Result<&String> {
			self.names[0].as_ref().context("No name for namespace zero")
		}

		pub(crate) fn get(&self, namespace: Namespace<N>) -> Option<&String> {
			self.names[namespace.0].as_ref()
		}

		pub(crate) fn names(&self) -> impl Iterator<Item=Option<&String>> {
			self.names.iter().map(|x| x.as_ref())
		}
	}

	impl<const N: usize> Debug for Names<N> {
		fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
			f.debug_list()
				.entry(&self.names)
				.finish()
		}
	}

	impl<const N: usize> From<[String; N]> for Names<N> {
		fn from(value: [String; N]) -> Self {
			Names { names: value.map(|x| Some(x)) }
		}
	}

	impl<const N: usize> From<[Option<String>; N]> for Names<N> {
		fn from(value: [Option<String>; N]) -> Self {
			Names { names: value }
		}
	}

	impl<const N: usize> From<Names<N>> for [Option<String>; N] {
		fn from(value: Names<N>) -> Self {
			value.names
		}
	}

	impl<'a, const N: usize> From<&'a Names<N>> for &'a [Option<String>; N] {
		fn from(value: &'a Names<N>) -> Self {
			&value.names
		}
	}
}
pub(crate) use names::Names;

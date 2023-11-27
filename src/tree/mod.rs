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

pub(crate) trait NodeData<T> {
	fn node_data(&self) -> &T;
}
pub(crate) trait NodeDataMut<T>: NodeData<T> {
	fn node_data_mut(&mut self) -> &mut T;
}

macro_rules! impl_node_data {
	(<$($bounds:ident$(,)?)*>, $ty:ty, $t:ty) => {
		impl<$($bounds,)*> NodeData<$t> for $ty {
			fn node_data(&self) -> &$t {
				&self.info
			}
		}
		impl<$($bounds,)*> NodeDataMut<$t> for $ty {
			fn node_data_mut(&mut self) -> &mut $t {
				&mut self.info
			}
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct MappingNowode<I, Ck, C, Fk, F, Mk, M, Pk, P, J> {
	pub(crate) info: I,
	pub(crate) javadoc: Option<J>,
	pub(crate) classes: IndexMap<Ck, ClassNowode<C, Fk, F, Mk, M, Pk, P, J>>
}

impl<I, Ck, C, Fk, F, Mk, M, Pk, P, J> MappingNowode<I, Ck, C, Fk, F, Mk, M, Pk, P, J>
where
	Ck: Eq + Hash + Debug,
	C: Debug,
	Fk: Eq + Hash + Debug,
	F: Debug,
	Mk: Eq + Hash + Debug,
	M: Debug,
	Pk: Eq + Hash + Debug,
	P: Debug,
	J: Debug,
{
	pub(crate) fn new(info: I) -> MappingNowode<I, Ck, C, Fk, F, Mk, M, Pk, P, J> {
		MappingNowode {
			info,
			javadoc: None,
			classes: IndexMap::new(),
		}
	}

	pub(crate) fn add_class(&mut self, key: Ck, child: ClassNowode<C, Fk, F, Mk, M, Pk, P, J>) -> Result<()> {
		match self.classes.entry(key) {
			Entry::Occupied(e) => {
				bail!("Cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
			},
			Entry::Vacant(e) => {
				e.insert(child);
			},
		}

		Ok(())
	}
}
impl_node_data!(<I, Ck, C, Fk, F, Mk, M, Pk, P, J>, MappingNowode<I, Ck, C, Fk, F, Mk, M, Pk, P, J>, I);

#[derive(Debug, Clone)]
pub(crate) struct ClassNowode<C, Fk, F, Mk, M, Pk, P, J> {
	pub(crate) info: C,
	pub(crate) javadoc: Option<J>,
	pub(crate) fields: IndexMap<Fk, FieldNowode<F, J>>,
	pub(crate) methods: IndexMap<Mk, MethodNowode<M, Pk, P, J>>,
}

impl<C, Fk, F, Mk, M, Pk, P, J> ClassNowode<C, Fk, F, Mk, M, Pk, P, J>
where
	Fk: Eq + Hash + Debug,
	F: Debug,
	Mk: Eq + Hash + Debug,
	M: Debug,
	Pk: Eq + Hash + Debug,
	P: Debug,
	J: Debug,
{
	pub(crate) fn new(info: C) -> ClassNowode<C, Fk, F, Mk, M, Pk, P, J> {
		ClassNowode {
			info,
			javadoc: None,
			fields: IndexMap::new(),
			methods: IndexMap::new(),
		}
	}

	pub(crate) fn add_field(&mut self, key: Fk, child: FieldNowode<F, J>) -> Result<()> {
		match self.fields.entry(key) {
			Entry::Occupied(e) => {
				bail!("Cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
			},
			Entry::Vacant(e) => {
				e.insert(child);
			},
		}

		Ok(())
	}

	pub(crate) fn add_method(&mut self, key: Mk, child: MethodNowode<M, Pk, P, J>) -> Result<()> {
		match self.methods.entry(key) {
			Entry::Occupied(e) => {
				bail!("Cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
			},
			Entry::Vacant(e) => {
				e.insert(child);
			},
		}

		Ok(())
	}
}
impl_node_data!(<C, Fk, F, Mk, M, Pk, P, J>, ClassNowode<C, Fk, F, Mk, M, Pk, P, J>, C);

#[derive(Debug, Clone)]
pub(crate) struct FieldNowode<F, J> {
	pub(crate) info: F,
	pub(crate) javadoc: Option<J>,
}

impl<F, J> FieldNowode<F, J> {
	pub(crate) fn new(info: F) -> FieldNowode<F, J> {
		FieldNowode {
			javadoc: None,
			info,
		}
	}
}
impl_node_data!(<F, J>, FieldNowode<F, J>, F);

#[derive(Debug, Clone)]
pub(crate) struct MethodNowode<M, Pk, P, J> {
	pub(crate) info: M,
	pub(crate) javadoc: Option<J>,
	pub(crate) parameters: IndexMap<Pk, ParameterNowode<P, J>>
}

impl<M, Pk, P, J> MethodNowode<M, Pk, P, J>
where
	Pk: Eq + Hash + Debug,
	P: Debug,
	J: Debug,
{
	pub(crate) fn new(info: M) -> MethodNowode<M, Pk, P, J> {
		MethodNowode {
			info,
			javadoc: None,
			parameters: IndexMap::new(),
		}
	}

	pub(crate) fn add_parameter(&mut self, key: Pk, child: ParameterNowode<P, J>) -> Result<()> {
		match self.parameters.entry(key) {
			Entry::Occupied(e) => {
				bail!("Cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
			},
			Entry::Vacant(e) => {
				e.insert(child);
			},
		}

		Ok(())
	}
}
impl_node_data!(<M, Pk, P, J>, MethodNowode<M, Pk, P, J>, M);

#[derive(Debug, Clone)]
pub(crate) struct ParameterNowode<P, J> {
	pub(crate) info: P,
	pub(crate) javadoc: Option<J>,
}

impl<P, J> ParameterNowode<P, J> {
	pub(crate) fn new(info: P) -> ParameterNowode<P, J> {
		ParameterNowode {
			info,
			javadoc: None,
		}
	}
}
impl_node_data!(<P, J>, ParameterNowode<P, J>, P);

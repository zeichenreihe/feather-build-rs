use anyhow::Result;
use std::fmt::Debug;
use std::hash::Hash;
use anyhow::bail;
use indexmap::IndexMap;
use indexmap::map::Entry;

pub(crate) mod mappings;
pub(crate) mod mappings_diff;

pub(crate) trait NodeData<T> {
	fn node_data(&self) -> &T;
}
pub(crate) trait NodeDataMut<T> {
	fn node_data_mut(&mut self) -> &mut T;
}
pub(crate) trait NodeJavadoc<J> {
	fn node_javadoc(&self) -> &Option<J>;
}
pub(crate) trait NodeJavadocMut<J> {
	fn node_javadoc_mut(&mut self) -> &mut Option<J>;
}

macro_rules! impl_as_inner_and_javadoc {
	(<$($bounds:ident$(,)?)*>, $ty:ty, $t:ty, $j:ty) => {
		impl<$($bounds,)*> NodeData<$t> for $ty {
			fn node_data(&self) -> &$t {
				&self.inner
			}
		}
		impl<$($bounds,)*> NodeDataMut<$t> for $ty {
			fn node_data_mut(&mut self) -> &mut $t {
				&mut self.inner
			}
		}
		impl<$($bounds,)*> NodeJavadoc<$j> for $ty {
			fn node_javadoc(&self) -> &Option<$j> {
				&self.javadoc
			}
		}
		impl<$($bounds,)*> NodeJavadocMut<$j> for $ty {
			fn node_javadoc_mut(&mut self) -> &mut Option<$j> {
				&mut self.javadoc
			}
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct MappingNowode<I, Ck, C, Fk, F, Mk, M, Pk, P, J> {
	inner: I,
	javadoc: Option<J>,
	classes: IndexMap<Ck, ClassNowode<C, Fk, F, Mk, M, Pk, P, J>>
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
	pub(crate) fn new(inner: I) -> MappingNowode<I, Ck, C, Fk, F, Mk, M, Pk, P, J> {
		MappingNowode {
			inner,
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

	pub(crate) fn classes(&self) -> impl Iterator<Item=&ClassNowode<C, Fk, F, Mk, M, Pk, P, J>> {
		self.classes.values()
	}
}
impl_as_inner_and_javadoc!(<I, Ck, C, Fk, F, Mk, M, Pk, P, J>, MappingNowode<I, Ck, C, Fk, F, Mk, M, Pk, P, J>, I, J);

#[derive(Debug, Clone)]
pub(crate) struct ClassNowode<C, Fk, F, Mk, M, Pk, P, J> {
	inner: C,
	javadoc: Option<J>,
	fields: IndexMap<Fk, FieldNowode<F, J>>,
	methods: IndexMap<Mk, MethodNowode<M, Pk, P, J>>,
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
	pub(crate) fn new(inner: C) -> ClassNowode<C, Fk, F, Mk, M, Pk, P, J> {
		ClassNowode {
			inner,
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

	pub(crate) fn fields(&self) -> impl Iterator<Item=&FieldNowode<F, J>> {
		self.fields.values()
	}

	pub(crate) fn methods(&self) -> impl Iterator<Item=&MethodNowode<M, Pk, P, J>> {
		self.methods.values()
	}
}
impl_as_inner_and_javadoc!(<C, Fk, F, Mk, M, Pk, P, J>, ClassNowode<C, Fk, F, Mk, M, Pk, P, J>, C, J);

#[derive(Debug, Clone)]
pub(crate) struct FieldNowode<F, J> {
	inner: F,
	javadoc: Option<J>,
}

impl<F, J> FieldNowode<F, J> {
	pub(crate) fn new(inner: F) -> FieldNowode<F, J> {
		FieldNowode {
			javadoc: None,
			inner,
		}
	}
}
impl_as_inner_and_javadoc!(<F, J>, FieldNowode<F, J>, F, J);

#[derive(Debug, Clone)]
pub(crate) struct MethodNowode<M, Pk, P, J> {
	inner: M,
	javadoc: Option<J>,
	parameters: IndexMap<Pk, ParameterNowode<P, J>>
}

impl<M, Pk, P, J> MethodNowode<M, Pk, P, J>
where
	Pk: Eq + Hash + Debug,
	P: Debug,
	J: Debug,
{
	pub(crate) fn new(inner: M) -> MethodNowode<M, Pk, P, J> {
		MethodNowode {
			inner,
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

	pub(crate) fn parameters(&self) -> impl Iterator<Item=&ParameterNowode<P, J>> {
		self.parameters.values()
	}
}
impl_as_inner_and_javadoc!(<M, Pk, P, J>, MethodNowode<M, Pk, P, J>, M, J);

#[derive(Debug, Clone)]
pub(crate) struct ParameterNowode<P, J> {
	inner: P,
	javadoc: Option<J>,
}

impl<P, J> ParameterNowode<P, J> {
	pub(crate) fn new(inner: P) -> ParameterNowode<P, J> {
		ParameterNowode {
			inner,
			javadoc: None,
		}
	}
}
impl_as_inner_and_javadoc!(<P, J>, ParameterNowode<P, J>, P, J);

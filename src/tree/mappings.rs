use std::fmt::Debug;
use crate::tree::{ClassNowode, FieldNowode, MappingNowode, MethodNowode, Names, ParameterNowode};

pub(crate) type Mappings<const N: usize> = MappingNowode<
	MappingInfo<N>,
	ClassKey, ClassMapping<N>,
	FieldKey, FieldMapping<N>,
	MethodKey, MethodMapping<N>,
	ParameterKey, ParameterMapping<N>,
	JavadocMapping
>;
pub(crate) type ClassNowodeMapping<const N: usize> = ClassNowode<
	ClassMapping<N>,
	FieldKey, FieldMapping<N>,
	MethodKey, MethodMapping<N>,
	ParameterKey, ParameterMapping<N>,
	JavadocMapping
>;
pub(crate) type FieldNowodeMapping<const N: usize> = FieldNowode<
	FieldMapping<N>,
	JavadocMapping
>;
pub(crate) type MethodNowodeMapping<const N: usize> = MethodNowode<
	MethodMapping<N>,
	ParameterKey, ParameterMapping<N>,
	JavadocMapping,
>;
pub(crate) type ParameterNowodeMapping<const N: usize> = ParameterNowode<
	ParameterMapping<N>,
	JavadocMapping
>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MappingInfo<const N: usize> {
	pub(crate) namespaces: [String; N],
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClassKey {
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct ClassMapping<const N: usize> {
	pub(crate) names: Names<N>,
}

impl<const N: usize> ClassMapping<N> {
	pub(crate) fn get_key(&self) -> ClassKey {
		ClassKey {
			src: self.names.src().clone(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldKey {
	pub(crate) desc: String,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct FieldMapping<const N: usize> {
	pub(crate) desc: String,
	pub(crate) names: Names<N>,
}

impl<const N: usize> FieldMapping<N> {
	pub(crate) fn get_key(&self) -> FieldKey {
		FieldKey {
			desc: self.desc.clone(),
			src: self.names.src().clone(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodKey {
	pub(crate) desc: String,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct MethodMapping<const N: usize> {
	pub(crate) desc: String,
	pub(crate) names: Names<N>,
}

impl<const N: usize> MethodMapping<N> {
	pub(crate) fn get_key(&self) -> MethodKey {
		MethodKey {
			desc: self.desc.clone(),
			src: self.names.src().clone(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ParameterKey {
	pub(crate) index: usize,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct ParameterMapping<const N: usize> {
	pub(crate) index: usize,
	pub(crate) names: Names<N>,
}

impl<const N: usize> ParameterMapping<N> {
	pub(crate) fn get_key(&self) -> ParameterKey {
		ParameterKey {
			index: self.index,
			src: self.names.src().clone(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct JavadocMapping(pub(crate) String);

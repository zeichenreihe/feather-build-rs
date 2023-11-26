use anyhow::{anyhow, Context, Result};
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
	src: String,
}

impl ClassKey {
	pub(crate) fn new(src: String) -> ClassKey {
		ClassKey { src }
	}
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct ClassMapping<const N: usize> {
	pub(crate) names: Names<N>,
}

impl<const N: usize> ClassMapping<N> {
	pub(crate) fn get_key(&self) -> Result<ClassKey> {
		Ok(ClassKey {
			src: self.names.src()
				.with_context(|| anyhow!("Cannot create key of class {self:?}: no name for first namespace"))?
				.clone(),
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldKey {
	desc: String,
	src: String,
}

impl FieldKey {
	pub(crate) fn new(desc: String, src: String) -> FieldKey {
		FieldKey { desc, src }
	}
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct FieldMapping<const N: usize> {
	pub(crate) desc: String,
	pub(crate) names: Names<N>,
}

impl<const N: usize> FieldMapping<N> {
	pub(crate) fn get_key(&self) -> Result<FieldKey> {
		Ok(FieldKey {
			desc: self.desc.clone(),
			src: self.names.src()
				.with_context(|| anyhow!("Cannot create key of field {self:?}: no name for first namespace"))?
				.clone(),
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodKey {
	desc: String,
	src: String,
}

impl MethodKey {
	pub(crate) fn new(desc: String, src: String) -> MethodKey {
		MethodKey { desc, src }
	}
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct MethodMapping<const N: usize> {
	pub(crate) desc: String,
	pub(crate) names: Names<N>,
}

impl<const N: usize> MethodMapping<N> {
	pub(crate) fn get_key(&self) -> Result<MethodKey> {
		Ok(MethodKey {
			desc: self.desc.clone(),
			src: self.names.src()
				.with_context(|| anyhow!("Cannot create key of method {self:?}: no name for first namespace"))?
				.clone(),
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ParameterKey {
	index: usize,
}

impl ParameterKey {
	pub(crate) fn new(index: usize) -> ParameterKey {
		ParameterKey { index }
	}
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct ParameterMapping<const N: usize> {
	pub(crate) index: usize,
	pub(crate) names: Names<N>,
}

impl<const N: usize> ParameterMapping<N> {
	pub(crate) fn get_key(&self) -> Result<ParameterKey> {
		Ok(ParameterKey {
			index: self.index,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct JavadocMapping(pub(crate) String);

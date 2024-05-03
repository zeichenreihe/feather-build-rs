use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use crate::tree::Names;
#[derive(Debug, Clone)]
pub(crate) struct Mappings<const N: usize> {
	pub(crate) info: MappingInfo<N>,
	pub(crate) classes: IndexMap<ClassKey, ClassNowodeMapping<N>>,
	pub(crate) javadoc: Option<JavadocMapping>,
}

impl<const N: usize> Mappings<N> {
	pub(crate) fn new(info: MappingInfo<N>) -> Mappings<N> {
		Mappings {
			info,
			classes: IndexMap::new(),
			javadoc: None,
		}
	}

	pub(crate) fn add_class(&mut self, key: ClassKey, child: ClassNowodeMapping<N>) -> Result<()> {
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

#[derive(Debug, Clone)]
pub(crate) struct ClassNowodeMapping<const N: usize> {
	pub(crate) info: ClassMapping<N>,
	pub(crate) fields: IndexMap<FieldKey, FieldNowodeMapping<N>>,
	pub(crate) methods: IndexMap<MethodKey, MethodNowodeMapping<N>>,
	pub(crate) javadoc: Option<JavadocMapping>,
}

impl<const N: usize> ClassNowodeMapping<N> {
	pub(crate) fn new(info: ClassMapping<N>) -> ClassNowodeMapping<N> {
		ClassNowodeMapping {
			info,
			fields: IndexMap::new(),
			methods: IndexMap::new(),
			javadoc: None,
		}
	}

	pub(crate) fn add_field(&mut self, key: FieldKey, child: FieldNowodeMapping<N>) -> Result<()> {
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

	pub(crate) fn add_method(&mut self, key: MethodKey, child: MethodNowodeMapping<N>) -> Result<()> {
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

#[derive(Debug, Clone)]
pub(crate) struct FieldNowodeMapping<const N: usize> {
	pub(crate) info: FieldMapping<N>,
	pub(crate) javadoc: Option<JavadocMapping>,
}

impl<const N: usize> FieldNowodeMapping<N> {
	pub(crate) fn new(info: FieldMapping<N>) -> FieldNowodeMapping<N> {
		FieldNowodeMapping {
			info,
			javadoc: None,
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct MethodNowodeMapping<const N: usize> {
	pub(crate) info: MethodMapping<N>,
	pub(crate) parameters: IndexMap<ParameterKey, ParameterNowodeMapping<N>>,
	pub(crate) javadoc: Option<JavadocMapping>,
}

impl<const N: usize> MethodNowodeMapping<N> {
	pub(crate) fn new(info: MethodMapping<N>) -> MethodNowodeMapping<N> {
		MethodNowodeMapping {
			info,
			parameters: IndexMap::new(),
			javadoc: None,
		}
	}

	pub(crate) fn add_parameter(&mut self, key: ParameterKey, child: ParameterNowodeMapping<N>) -> Result<()> {
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

#[derive(Debug, Clone)]
pub(crate) struct ParameterNowodeMapping<const N: usize> {
	pub(crate) info: ParameterMapping<N>,
	pub(crate) javadoc: Option<JavadocMapping>,
}

impl<const N: usize> ParameterNowodeMapping<N> {
	pub(crate) fn new(info: ParameterMapping<N>) -> ParameterNowodeMapping<N> {
		ParameterNowodeMapping {
			info,
			javadoc: None,
		}
	}
}

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

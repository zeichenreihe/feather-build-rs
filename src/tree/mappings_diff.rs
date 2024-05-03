use anyhow::{bail, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use crate::tree::mappings::{ClassKey, ClassMapping, FieldKey, FieldMapping, JavadocMapping, MappingInfo, MethodKey, MethodMapping, ParameterKey, ParameterMapping};

#[derive(Debug, Clone, Default)]
pub(crate) enum Action<T> {
	Add(T),
	Remove(T),
	Edit(T, T),
	#[default]
	None,
}

#[derive(Debug, Clone)]
pub(crate) struct MappingsDiff {
	pub(crate) info: Action<MappingInfo<2>>,
	pub(crate) classes: IndexMap<ClassKey, ClassNowodeDiff>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl MappingsDiff {
	pub(crate) fn new(info: Action<MappingInfo<2>>) -> MappingsDiff {
		MappingsDiff {
			info,
			javadoc: None,
			classes: IndexMap::new(),
		}
	}

	pub(crate) fn add_class(&mut self, key: ClassKey, child: ClassNowodeDiff) -> Result<()> {
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
pub(crate) struct ClassNowodeDiff {
	pub(crate) info: Action<ClassMapping<2>>,
	pub(crate) fields: IndexMap<FieldKey, FieldNowodeDiff>,
	pub(crate) methods: IndexMap<MethodKey, MethodNowodeDiff>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl ClassNowodeDiff {
	pub(crate) fn new(info: Action<ClassMapping<2>>) -> ClassNowodeDiff {
		ClassNowodeDiff {
			info,
			fields: IndexMap::new(),
			methods: IndexMap::new(),
			javadoc: None,
		}
	}

	pub(crate) fn add_field(&mut self, key: FieldKey, child: FieldNowodeDiff) -> Result<()> {
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

	pub(crate) fn add_method(&mut self, key: MethodKey, child: MethodNowodeDiff) -> Result<()> {
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
pub(crate) struct FieldNowodeDiff {
	pub(crate) info: Action<FieldMapping<2>>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl FieldNowodeDiff {
	pub(crate) fn new(info: Action<FieldMapping<2>>) -> FieldNowodeDiff {
		FieldNowodeDiff {
			info,
			javadoc: None,
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct MethodNowodeDiff {
	pub(crate) info: Action<MethodMapping<2>>,
	pub(crate) parameters: IndexMap<ParameterKey, ParameterNowodeDiff>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl MethodNowodeDiff {
	pub(crate) fn new(info: Action<MethodMapping<2>>) -> MethodNowodeDiff {
		MethodNowodeDiff {
			info,
			parameters: IndexMap::new(),
			javadoc: None,
		}
	}

	pub(crate) fn add_parameter(&mut self, key: ParameterKey, child: ParameterNowodeDiff) -> Result<()> {
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
pub(crate) struct ParameterNowodeDiff {
	pub(crate) info: Action<ParameterMapping<2>>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl ParameterNowodeDiff {
	pub(crate) fn new(info: Action<ParameterMapping<2>>) -> ParameterNowodeDiff {
		ParameterNowodeDiff {
			info,
			javadoc: None,
		}
	}
}

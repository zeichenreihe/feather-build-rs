use anyhow::{bail, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use duke::tree::class::ClassName;
use duke::tree::field::{FieldName, FieldNameAndDesc};
use duke::tree::method::{MethodName, MethodNameAndDesc, ParameterName};
use crate::tree::mappings::{JavadocMapping, ParameterKey};
use crate::tree::NodeInfo;

#[derive(Debug, Clone, Default)]
pub(crate) enum Action<T> {
	Add(T),
	Remove(T),
	Edit(T, T),
	#[default]
	None,
}

impl<T: PartialEq> Action<T> {
	pub(crate) fn is_diff(&self) -> bool {
		match self {
			Action::Add(_) => true,
			Action::Remove(_) => true,
			Action::Edit(a, b) => a != b,
			Action::None => false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct MappingsDiff {
	pub(crate) info: Action<String>,
	pub(crate) classes: IndexMap<ClassName, ClassNowodeDiff>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl NodeInfo<Action<String>> for MappingsDiff {
	fn get_node_info(&self) -> &Action<String> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut Action<String> {
		&mut self.info
	}

	fn new(info: Action<String>) -> Self {
		MappingsDiff {
			info,
			javadoc: None,
			classes: IndexMap::new(),
		}
	}
}

impl MappingsDiff {
	pub(crate) fn add_class(&mut self, key: ClassName, child: ClassNowodeDiff) -> Result<()> {
		match self.classes.entry(key) {
			Entry::Occupied(e) => {
				bail!("cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
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
	pub(crate) info: Action<ClassName>,
	pub(crate) fields: IndexMap<FieldNameAndDesc, FieldNowodeDiff>,
	pub(crate) methods: IndexMap<MethodNameAndDesc, MethodNowodeDiff>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl NodeInfo<Action<ClassName>> for ClassNowodeDiff {
	fn get_node_info(&self) -> &Action<ClassName> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut Action<ClassName> {
		&mut self.info
	}

	fn new(info: Action<ClassName>) -> Self {
		ClassNowodeDiff {
			info,
			fields: IndexMap::new(),
			methods: IndexMap::new(),
			javadoc: None,
		}
	}
}

impl ClassNowodeDiff {
	pub(crate) fn add_field(&mut self, key: FieldNameAndDesc, child: FieldNowodeDiff) -> Result<()> {
		match self.fields.entry(key) {
			Entry::Occupied(e) => {
				bail!("cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
			},
			Entry::Vacant(e) => {
				e.insert(child);
			},
		}

		Ok(())
	}

	pub(crate) fn add_method(&mut self, key: MethodNameAndDesc, child: MethodNowodeDiff) -> Result<()> {
		match self.methods.entry(key) {
			Entry::Occupied(e) => {
				bail!("cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
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
	pub(crate) info: Action<FieldName>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl NodeInfo<Action<FieldName>> for FieldNowodeDiff {
	fn get_node_info(&self) -> &Action<FieldName> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut Action<FieldName> {
		&mut self.info
	}

	fn new(info: Action<FieldName>) -> FieldNowodeDiff {
		FieldNowodeDiff {
			info,
			javadoc: None,
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct MethodNowodeDiff {
	pub(crate) info: Action<MethodName>,
	pub(crate) parameters: IndexMap<ParameterKey, ParameterNowodeDiff>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl NodeInfo<Action<MethodName>> for MethodNowodeDiff {
	fn get_node_info(&self) -> &Action<MethodName> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut Action<MethodName> {
		&mut self.info
	}

	fn new(info: Action<MethodName>) -> Self {
		MethodNowodeDiff {
			info,
			parameters: IndexMap::new(),
			javadoc: None,
		}
	}
}

impl MethodNowodeDiff {
	pub(crate) fn add_parameter(&mut self, key: ParameterKey, child: ParameterNowodeDiff) -> Result<()> {
		match self.parameters.entry(key) {
			Entry::Occupied(e) => {
				bail!("cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
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
	pub(crate) info: Action<ParameterName>,
	pub(crate) javadoc: Option<Action<JavadocMapping>>,
}

impl NodeInfo<Action<ParameterName>> for ParameterNowodeDiff {
	fn get_node_info(&self) -> &Action<ParameterName> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut Action<ParameterName> {
		&mut self.info
	}

	fn new(info: Action<ParameterName>) -> ParameterNowodeDiff {
		ParameterNowodeDiff {
			info,
			javadoc: None,
		}
	}
}

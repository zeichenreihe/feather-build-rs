use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
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

fn add_child<Key, Node, Info>(map: &mut IndexMap<Key, Node>, key: Key, child: Node) -> Result<&mut Node>
	where
		Node: NodeInfo<Info>,
		Key: Debug + PartialEq + Eq + Hash,
{
	match map.entry(key) {
		Entry::Occupied(e) => bail!("cannot add child: key {:?} already exists", e.key()),
		Entry::Vacant(e) => Ok(e.insert(child)),
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
	pub(crate) fn add_class(&mut self, key: ClassName, child: ClassNowodeDiff) -> Result<&mut ClassNowodeDiff> {
		add_child(&mut self.classes, key, child)
			.with_context(|| anyhow!("failed to add class diff to mappings diff {:?}", self.info))
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
	pub(crate) fn add_field(&mut self, key: FieldNameAndDesc, child: FieldNowodeDiff) -> Result<&mut FieldNowodeDiff> {
		add_child(&mut self.fields, key, child)
			.with_context(|| anyhow!("failed to add field diff to class diff {:?}", self.info))
	}

	pub(crate) fn add_method(&mut self, key: MethodNameAndDesc, child: MethodNowodeDiff) -> Result<&mut MethodNowodeDiff> {
		add_child(&mut self.methods, key, child)
			.with_context(|| anyhow!("failed to add method diff to class diff {:?}", self.info))
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
	pub(crate) fn add_parameter(&mut self, key: ParameterKey, child: ParameterNowodeDiff) -> Result<&mut ParameterNowodeDiff> {
		add_child(&mut self.parameters, key, child)
			.with_context(|| anyhow!("failed to add parameter diff to method diff {:?}", self.info))
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

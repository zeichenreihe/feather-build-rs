use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use duke::tree::class::ClassName;
use duke::tree::field::{FieldName, FieldNameAndDesc};
use duke::tree::method::{MethodName, MethodNameAndDesc, ParameterName};
use crate::tree::mappings::{JavadocMapping, ParameterKey};
use crate::tree::{NodeInfo, NodeJavadocInfo};

mod action;
pub use action::*;

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

/// A diff on a whole mappings tree.
///
/// Implements [`Default`] with [`Action::None`].
#[derive(Clone, Debug, Default)]
pub struct MappingsDiff {
	pub info: Action<String>,
	pub classes: IndexMap<ClassName, ClassNowodeDiff>,
	pub javadoc: Option<Action<JavadocMapping>>,
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

impl NodeJavadocInfo<Action<JavadocMapping>> for MappingsDiff {
	fn get_node_javadoc_info(&self) -> &Option<Action<JavadocMapping>> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<Action<JavadocMapping>> {
		&mut self.javadoc
	}
}

impl MappingsDiff {
	pub(crate) fn add_class(&mut self, key: ClassName, child: ClassNowodeDiff) -> Result<&mut ClassNowodeDiff> {
		add_child(&mut self.classes, key, child)
			.with_context(|| anyhow!("failed to add class diff to mappings diff {:?}", self.info))
	}
}

/// A diff on a class node.
///
/// Implements [`Default`] with [`Action::None`].
#[derive(Clone, Debug, Default)]
pub struct ClassNowodeDiff {
	pub info: Action<ClassName>,
	pub fields: IndexMap<FieldNameAndDesc, FieldNowodeDiff>,
	pub methods: IndexMap<MethodNameAndDesc, MethodNowodeDiff>,
	pub javadoc: Option<Action<JavadocMapping>>,
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

impl NodeJavadocInfo<Action<JavadocMapping>> for ClassNowodeDiff {
	fn get_node_javadoc_info(&self) -> &Option<Action<JavadocMapping>> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<Action<JavadocMapping>> {
		&mut self.javadoc
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

/// A diff on a field node.
///
/// Implements [`Default`] with [`Action::None`].
#[derive(Clone, Debug, Default)]
pub struct FieldNowodeDiff {
	pub info: Action<FieldName>,
	pub javadoc: Option<Action<JavadocMapping>>,
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

impl NodeJavadocInfo<Action<JavadocMapping>> for FieldNowodeDiff {
	fn get_node_javadoc_info(&self) -> &Option<Action<JavadocMapping>> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<Action<JavadocMapping>> {
		&mut self.javadoc
	}
}

/// A diff on a method node.
///
/// Implements [`Default`] with [`Action::None`].
#[derive(Clone, Debug, Default)]
pub struct MethodNowodeDiff {
	pub info: Action<MethodName>,
	pub parameters: IndexMap<ParameterKey, ParameterNowodeDiff>,
	pub javadoc: Option<Action<JavadocMapping>>,
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

impl NodeJavadocInfo<Action<JavadocMapping>> for MethodNowodeDiff {
	fn get_node_javadoc_info(&self) -> &Option<Action<JavadocMapping>> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<Action<JavadocMapping>> {
		&mut self.javadoc
	}
}

impl MethodNowodeDiff {
	pub(crate) fn add_parameter(&mut self, key: ParameterKey, child: ParameterNowodeDiff) -> Result<&mut ParameterNowodeDiff> {
		add_child(&mut self.parameters, key, child)
			.with_context(|| anyhow!("failed to add parameter diff to method diff {:?}", self.info))
	}
}

/// A diff on a parameter node.
///
/// Implements [`Default`] with [`Action::None`].
#[derive(Clone, Debug, Default)]
pub struct ParameterNowodeDiff {
	pub info: Action<ParameterName>,
	pub javadoc: Option<Action<JavadocMapping>>,
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

impl NodeJavadocInfo<Action<JavadocMapping>> for ParameterNowodeDiff {
	fn get_node_javadoc_info(&self) -> &Option<Action<JavadocMapping>> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<Action<JavadocMapping>> {
		&mut self.javadoc
	}
}

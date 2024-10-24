use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use duke::tree::class::{ObjClassName, ObjClassNameSlice};
use duke::tree::field::{FieldDescriptor, FieldName, FieldNameAndDesc};
use duke::tree::method::{MethodDescriptor, MethodName, MethodNameAndDesc, ParameterName};
use crate::tree::names::{Names, Namespace, Namespaces};
use crate::tree::{FromKey, GetNames, NodeInfo, NodeJavadocInfo, ToKey};

fn add_child<Key, Node, Info>(map: &mut IndexMap<Key, Node>, child: Node) -> Result<&mut Node>
where
	Node: NodeInfo<Info>,
	Info: ToKey<Key>,
	Key: Debug + Eq + Hash,
{
	let key = child.get_node_info().get_key().context("cannot add child: failed to get its key")?;

	match map.entry(key) {
		Entry::Occupied(e) => bail!("cannot add child: key {:?} already exists", e.key()),
		Entry::Vacant(e) => Ok(e.insert(child)),
	}
}

#[derive(Debug, Clone)]
pub struct Mappings<const N: usize> {
	pub info: MappingInfo<N>,
	pub classes: IndexMap<ObjClassName, ClassNowodeMapping<N>>,
	pub javadoc: Option<JavadocMapping>,
}

impl<const N: usize> NodeInfo<MappingInfo<N>> for Mappings<N> {
	fn get_node_info(&self) -> &MappingInfo<N> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut MappingInfo<N> {
		&mut self.info
	}

	fn new(info: MappingInfo<N>) -> Self {
		Mappings {
			info,
			classes: IndexMap::new(),
			javadoc: None,
		}
	}
}

impl<const N: usize> NodeJavadocInfo<Option<JavadocMapping>> for Mappings<N> {
	fn get_node_javadoc_info(&self) -> &Option<JavadocMapping> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<JavadocMapping> {
		&mut self.javadoc
	}
}

impl<const N: usize> Mappings<N> {
	pub fn from_namespaces(namespaces: [&str; N]) -> Result<Mappings<N>> {
		Namespaces::try_from(namespaces.map(|x| x.to_owned()))
			.map(|namespaces| Mappings::new(MappingInfo { namespaces }))
	}

	pub(crate) fn add_class(&mut self, child: ClassNowodeMapping<N>) -> Result<&mut ClassNowodeMapping<N>> {
		add_child(&mut self.classes, child)
			.with_context(|| anyhow!("failed to add class to mappings {:?}", self.info))
	}

	pub(crate) fn get_class_name(&self, class: &ObjClassNameSlice, namespace: Namespace<N>) -> Result<&ObjClassNameSlice> {
		self.classes.get(class)
			.with_context(|| anyhow!("no entry for class {class:?}"))?
			.info
			.names[namespace]
			.as_deref()
			.with_context(|| anyhow!("no name for namespace {namespace:?} for class {class:?}"))
	}
}

#[derive(Debug, Clone)]
pub struct ClassNowodeMapping<const N: usize> {
	pub info: ClassMapping<N>,
	pub fields: IndexMap<FieldNameAndDesc, FieldNowodeMapping<N>>,
	pub methods: IndexMap<MethodNameAndDesc, MethodNowodeMapping<N>>,
	pub javadoc: Option<JavadocMapping>,
}

impl<const N: usize> NodeInfo<ClassMapping<N>> for ClassNowodeMapping<N> {
	fn get_node_info(&self) -> &ClassMapping<N> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut ClassMapping<N> {
		&mut self.info
	}

	fn new(info: ClassMapping<N>) -> Self {
		ClassNowodeMapping {
			info,
			fields: IndexMap::new(),
			methods: IndexMap::new(),
			javadoc: None,
		}
	}
}

impl<const N: usize> NodeJavadocInfo<Option<JavadocMapping>> for ClassNowodeMapping<N> {
	fn get_node_javadoc_info(&self) -> &Option<JavadocMapping> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<JavadocMapping> {
		&mut self.javadoc
	}
}

impl<const N: usize> ClassNowodeMapping<N> {
	pub(crate) fn add_field(&mut self, child: FieldNowodeMapping<N>) -> Result<&mut FieldNowodeMapping<N>> {
		add_child(&mut self.fields, child)
			.with_context(|| anyhow!("failed to add field to class {:?}", self.info))
	}

	pub(crate) fn add_method(&mut self, child: MethodNowodeMapping<N>) -> Result<&mut MethodNowodeMapping<N>> {
		add_child(&mut self.methods, child)
			.with_context(|| anyhow!("failed to add method to class {:?}", self.info))
	}
}

#[derive(Debug, Clone)]
pub struct FieldNowodeMapping<const N: usize> {
	pub info: FieldMapping<N>,
	pub javadoc: Option<JavadocMapping>,
}

impl<const N: usize> NodeInfo<FieldMapping<N>> for FieldNowodeMapping<N> {
	fn get_node_info(&self) -> &FieldMapping<N> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut FieldMapping<N> {
		&mut self.info
	}

	fn new(info: FieldMapping<N>) -> FieldNowodeMapping<N> {
		FieldNowodeMapping {
			info,
			javadoc: None,
		}
	}
}

impl<const N: usize> NodeJavadocInfo<Option<JavadocMapping>> for FieldNowodeMapping<N> {
	fn get_node_javadoc_info(&self) -> &Option<JavadocMapping> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<JavadocMapping> {
		&mut self.javadoc
	}
}

#[derive(Debug, Clone)]
pub struct MethodNowodeMapping<const N: usize> {
	pub info: MethodMapping<N>,
	pub parameters: IndexMap<ParameterKey, ParameterNowodeMapping<N>>,
	pub javadoc: Option<JavadocMapping>,
}

impl<const N: usize> NodeInfo<MethodMapping<N>> for MethodNowodeMapping<N> {
	fn get_node_info(&self) -> &MethodMapping<N> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut MethodMapping<N> {
		&mut self.info
	}

	fn new(info: MethodMapping<N>) -> Self {
		MethodNowodeMapping {
			info,
			parameters: IndexMap::new(),
			javadoc: None,
		}
	}
}

impl<const N: usize> NodeJavadocInfo<Option<JavadocMapping>> for MethodNowodeMapping<N> {
	fn get_node_javadoc_info(&self) -> &Option<JavadocMapping> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<JavadocMapping> {
		&mut self.javadoc
	}
}

impl<const N: usize> MethodNowodeMapping<N> {
	pub(crate) fn add_parameter(&mut self, child: ParameterNowodeMapping<N>) -> Result<&mut ParameterNowodeMapping<N>> {
		add_child(&mut self.parameters, child)
			.with_context(|| anyhow!("failed to add parameter to method {:?}", self.info))
	}
}

#[derive(Debug, Clone)]
pub struct ParameterNowodeMapping<const N: usize> {
	pub info: ParameterMapping<N>,
	pub javadoc: Option<JavadocMapping>,
}

impl<const N: usize> NodeInfo<ParameterMapping<N>> for ParameterNowodeMapping<N> {
	fn get_node_info(&self) -> &ParameterMapping<N> {
		&self.info
	}

	fn get_node_info_mut(&mut self) -> &mut ParameterMapping<N> {
		&mut self.info
	}

	fn new(info: ParameterMapping<N>) -> ParameterNowodeMapping<N> {
		ParameterNowodeMapping {
			info,
			javadoc: None,
		}
	}
}

impl<const N: usize> NodeJavadocInfo<Option<JavadocMapping>> for ParameterNowodeMapping<N> {
	fn get_node_javadoc_info(&self) -> &Option<JavadocMapping> {
		&self.javadoc
	}

	fn get_node_javadoc_info_mut(&mut self) -> &mut Option<JavadocMapping> {
		&mut self.javadoc
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct MappingInfo<const N: usize> {
	pub namespaces: Namespaces<N>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClassKey {
	pub src: ObjClassName,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ClassMapping<const N: usize> {
	pub names: Names<N, ObjClassName>,
}

impl<const N: usize> ToKey<ObjClassName> for ClassMapping<N> {
	fn get_key(&self) -> Result<ObjClassName> {
		self.names.first_name().cloned()
	}
}

impl<const N: usize> FromKey<ObjClassName> for ClassMapping<N> {
	fn from_key(key: ObjClassName) -> ClassMapping<N> {
		ClassMapping {
			names: Names::from_first_name(key),
		}
	}
}

impl<const N: usize> GetNames<N, ObjClassName> for ClassMapping<N> {
	fn get_names(&self) -> &Names<N, ObjClassName> {
		&self.names
	}

	fn get_names_mut(&mut self) -> &mut Names<N, ObjClassName> {
		&mut self.names
	}
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct FieldMapping<const N: usize> {
	pub desc: FieldDescriptor,
	pub names: Names<N, FieldName>,
}

impl<const N: usize> ToKey<FieldNameAndDesc> for FieldMapping<N> {
	fn get_key(&self) -> Result<FieldNameAndDesc> {
		Ok(FieldNameAndDesc {
			desc: self.desc.clone(),
			name: self.names.first_name()?.clone(),
		})
	}
}

impl<const N: usize> FromKey<FieldNameAndDesc> for FieldMapping<N> {
	fn from_key(key: FieldNameAndDesc) -> FieldMapping<N> {
		FieldMapping {
			desc: key.desc,
			names: Names::from_first_name(key.name),
		}
	}
}

impl<const N: usize> GetNames<N, FieldName> for FieldMapping<N> {
	fn get_names(&self) -> &Names<N, FieldName> {
		&self.names
	}

	fn get_names_mut(&mut self) -> &mut Names<N, FieldName> {
		&mut self.names
	}
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct MethodMapping<const N: usize> {
	pub desc: MethodDescriptor,
	pub names: Names<N, MethodName>,
}

impl<const N: usize> ToKey<MethodNameAndDesc> for MethodMapping<N> {
	fn get_key(&self) -> Result<MethodNameAndDesc> {
		Ok(MethodNameAndDesc {
			desc: self.desc.clone(),
			name: self.names.first_name()?.clone(),
		})
	}
}

impl<const N: usize> FromKey<MethodNameAndDesc> for MethodMapping<N> {
	fn from_key(key: MethodNameAndDesc) -> MethodMapping<N> {
		MethodMapping {
			desc: key.desc,
			names: Names::from_first_name(key.name),
		}
	}
}

impl<const N: usize> GetNames<N, MethodName> for MethodMapping<N> {
	fn get_names(&self) -> &Names<N, MethodName> {
		&self.names
	}

	fn get_names_mut(&mut self) -> &mut Names<N, MethodName> {
		&mut self.names
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParameterKey {
	pub index: usize,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ParameterMapping<const N: usize> {
	pub index: usize,
	pub names: Names<N, ParameterName>,
}

impl<const N: usize> ToKey<ParameterKey> for ParameterMapping<N> {
	fn get_key(&self) -> Result<ParameterKey> {
		Ok(ParameterKey {
			index: self.index,
		})
	}
}

impl<const N: usize> FromKey<ParameterKey> for ParameterMapping<N> {
	fn from_key(key: ParameterKey) -> ParameterMapping<N> {
		ParameterMapping {
			index: key.index,
			names: Names::none(),
		}
	}
}

impl<const N: usize> GetNames<N, ParameterName> for ParameterMapping<N> {
	fn get_names(&self) -> &Names<N, ParameterName> {
		&self.names
	}

	fn get_names_mut(&mut self) -> &mut Names<N, ParameterName> {
		&mut self.names
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JavadocMapping(pub String);

impl From<String> for JavadocMapping {
	fn from(value: String) -> Self {
		JavadocMapping(value)
	}
}

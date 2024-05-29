use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use duke::tree::class::ClassName;
use duke::tree::field::{FieldDescriptor, FieldName};
use duke::tree::method::{MethodDescriptor, MethodName, ParameterName};
use crate::tree::names::{Names, Namespace, Namespaces};
use crate::tree::{FromKey, NodeInfo, ToKey};

#[derive(Debug, Clone)]
pub struct Mappings<const N: usize> {
	pub info: MappingInfo<N>,
	pub classes: IndexMap<ClassName, ClassNowodeMapping<N>>,
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

impl<const N: usize> Mappings<N> {
	pub(crate) fn add_class(&mut self, child: ClassNowodeMapping<N>) -> Result<()> {
		match self.classes.entry(child.info.get_key()) {
			Entry::Occupied(e) => {
				bail!("cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
			},
			Entry::Vacant(e) => {
				e.insert(child);
			},
		}

		Ok(())
	}

	pub(crate) fn get_class_name(&self, class: &ClassName, namespace: Namespace<N>) -> Result<&ClassName> {
		self.classes.get(class)
			.with_context(|| anyhow!("no entry for class {class:?}"))?
			.info
			.names[namespace]
			.as_ref()
			.with_context(|| anyhow!("no name for namespace {namespace:?} for class {class:?}"))
	}
}

#[derive(Debug, Clone)]
pub struct ClassNowodeMapping<const N: usize> {
	pub info: ClassMapping<N>,
	pub fields: IndexMap<FieldKey, FieldNowodeMapping<N>>,
	pub methods: IndexMap<MethodKey, MethodNowodeMapping<N>>,
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

impl<const N: usize> ClassNowodeMapping<N> {
	pub(crate) fn add_field(&mut self, child: FieldNowodeMapping<N>) -> Result<()> {
		match self.fields.entry(child.info.get_key()) {
			Entry::Occupied(e) => {
				bail!("cannot add child {child:?} for key {:?}, as there's already one: {:?}", e.key(), e.get());
			},
			Entry::Vacant(e) => {
				e.insert(child);
			},
		}

		Ok(())
	}

	pub(crate) fn add_method(&mut self, child: MethodNowodeMapping<N>) -> Result<()> {
		match self.methods.entry(child.info.get_key()) {
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

#[derive(Debug, Clone)]
pub struct MethodNowodeMapping<const N: usize> {
	pub info: MethodMapping<N>,
	pub parameters: IndexMap<ParameterKey, ParameterNowodeMapping<N>>,
	pub(crate) javadoc: Option<JavadocMapping>,
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

impl<const N: usize> MethodNowodeMapping<N> {
	pub(crate) fn add_parameter(&mut self, child: ParameterNowodeMapping<N>) -> Result<()> {
		match self.parameters.entry(child.info.get_key()) {
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
pub struct ParameterNowodeMapping<const N: usize> {
	pub info: ParameterMapping<N>,
	pub(crate) javadoc: Option<JavadocMapping>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct MappingInfo<const N: usize> {
	pub namespaces: Namespaces<N>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClassKey {
	pub(crate) src: ClassName,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ClassMapping<const N: usize> {
	pub names: Names<N, ClassName>,
}

impl<const N: usize> ToKey<ClassName> for ClassMapping<N> {
	fn get_key(&self) -> ClassName {
		self.names.first_name().clone()
	}
}

impl<const N: usize> FromKey<ClassName> for ClassMapping<N> {
	fn from_key(key: ClassName) -> ClassMapping<N> {
		ClassMapping {
			names: Names::from_first_name(key),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldKey {
	pub desc: FieldDescriptor,
	pub name: FieldName,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct FieldMapping<const N: usize> {
	pub(crate) desc: FieldDescriptor,
	pub names: Names<N, FieldName>,
}

impl<const N: usize> ToKey<FieldKey> for FieldMapping<N> {
	fn get_key(&self) -> FieldKey {
		FieldKey {
			desc: self.desc.clone(),
			name: self.names.first_name().clone(),
		}
	}
}

impl<const N: usize> FromKey<FieldKey> for FieldMapping<N> {
	fn from_key(key: FieldKey) -> FieldMapping<N> {
		FieldMapping {
			desc: key.desc,
			names: Names::from_first_name(key.name),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodKey {
	pub desc: MethodDescriptor,
	pub name: MethodName,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct MethodMapping<const N: usize> {
	pub desc: MethodDescriptor,
	pub names: Names<N, MethodName>,
}

impl<const N: usize> ToKey<MethodKey> for MethodMapping<N> {
	fn get_key(&self) -> MethodKey {
		MethodKey {
			desc: self.desc.clone(),
			name: self.names.first_name().clone(),
		}
	}
}

impl<const N: usize> FromKey<MethodKey> for MethodMapping<N> {
	fn from_key(key: MethodKey) -> MethodMapping<N> {
		MethodMapping {
			desc: key.desc,
			names: Names::from_first_name(key.name),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParameterKey {
	pub(crate) index: usize,
	pub(crate) name: ParameterName,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ParameterMapping<const N: usize> {
	pub(crate) index: usize,
	pub names: Names<N, ParameterName>,
}

impl<const N: usize> ToKey<ParameterKey> for ParameterMapping<N> {
	fn get_key(&self) -> ParameterKey {
		ParameterKey {
			index: self.index,
			name: self.names.first_name().clone(),
		}
	}
}

impl<const N: usize> FromKey<ParameterKey> for ParameterMapping<N> {
	fn from_key(key: ParameterKey) -> ParameterMapping<N> {
		ParameterMapping {
			index: key.index,
			names: Names::from_first_name(key.name),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JavadocMapping(pub(crate) String);

impl From<String> for JavadocMapping {
	fn from(value: String) -> Self {
		JavadocMapping(value)
	}
}

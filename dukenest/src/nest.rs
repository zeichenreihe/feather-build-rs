//! Module containing the data types that are used to store nesting information.
use indexmap::IndexMap;
use java_string::JavaString;
use duke::tree::class::{InnerClassFlags, ObjClassName};
use duke::tree::method::MethodNameAndDesc;

#[derive(Clone, Debug)]
pub enum NestType {
	Anonymous,
	Inner,
	Local,
}

#[derive(Clone, Debug)]
pub struct Nest {
	pub nest_type: NestType,

	pub class_name: ObjClassName,
	pub encl_class_name: ObjClassName,
	pub encl_method: Option<MethodNameAndDesc>,

	pub inner_name: JavaString,
	pub inner_access: InnerClassFlags,
}

/// Represents nests.
///
/// The default is an empty map.
#[derive(Clone, Debug, Default)]
pub struct Nests {
	pub all: IndexMap<ObjClassName, Nest>,
}

impl Nests {
	pub(crate) fn add(&mut self, nest: Nest) {
		self.all.insert(nest.class_name.clone(), nest);
	}
}

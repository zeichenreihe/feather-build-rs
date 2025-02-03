//! Module containing the data types that are used to store nesting information.
use std::marker::PhantomData;
use indexmap::IndexMap;
use java_string::JavaString;
use duke::tree::class::{InnerClassFlags, ObjClassName};
use duke::tree::method::MethodNameAndDesc;

#[derive(Clone, Copy, Debug)]
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

	pub inner_name: ObjClassName,
	pub inner_access: InnerClassFlags,
}

/// Represents nests.
///
/// The default is an empty map.
#[derive(Clone, Debug)]
pub struct Nests<Namespace> {
	pub phantom: PhantomData<Namespace>,
	pub all: IndexMap<ObjClassName, Nest>,
}

impl<Namespace> Default for Nests<Namespace> {
	fn default() -> Self {
		Nests {
			phantom: PhantomData,
			all: IndexMap::default(),
		}
	}
}

impl<Namespace> Nests<Namespace> {
	pub(crate) fn add(&mut self, nest: Nest) {
		self.all.insert(nest.class_name.clone(), nest);
	}
}

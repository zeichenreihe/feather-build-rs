use std::ops::ControlFlow;
use anyhow::Result;
use crate::tree::class::{ClassAccess, ObjClassName};
use crate::tree::version::Version;
use crate::visitor::class::ClassVisitor;

mod implementations;

pub mod simple;

pub mod class;
pub(crate) mod attribute;
pub(crate) mod field;
pub mod method;
pub mod annotation;
pub(crate) mod record;

pub trait MultiClassVisitor
where
	Self: Sized,
	Self::ClassVisitor: ClassVisitor,
{
	type ClassVisitor;
	type ClassResidual;

	fn visit_class(self, version: Version, access: ClassAccess, name: ObjClassName, super_class: Option<ObjClassName>, interfaces: Vec<ObjClassName>)
		-> Result<ControlFlow<Self, (Self::ClassResidual, Self::ClassVisitor)>>;
	fn finish_class(this: Self::ClassResidual, class_visitor: Self::ClassVisitor) -> Result<Self>;
}
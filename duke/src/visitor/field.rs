use anyhow::Result;
use crate::tree::field::{ConstantValue, FieldSignature};
use crate::tree::type_annotation::TargetInfoField;
use crate::visitor::annotation::{AnnotationsVisitor, TypeAnnotationsVisitor};
use crate::visitor::attribute::UnknownAttributeVisitor;

pub trait FieldVisitor
where
	Self: Sized,
	Self::AnnotationsVisitor: AnnotationsVisitor,
	Self::TypeAnnotationsVisitor: TypeAnnotationsVisitor<TargetInfoField>,
	Self::UnknownAttribute: UnknownAttributeVisitor,
{
	type AnnotationsVisitor;
	type AnnotationsResidual;
	type TypeAnnotationsVisitor;
	type TypeAnnotationsResidual;
	type UnknownAttribute;

	fn interests(&self) -> FieldInterests;

	fn visit_deprecated_and_synthetic_attribute(&mut self, deprecated: bool, synthetic: bool) -> Result<()>;

	fn visit_constant_value(&mut self, constant_value: ConstantValue) -> Result<()>;
	fn visit_signature(&mut self, signature: FieldSignature) -> Result<()>;

	fn visit_annotations(self, visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)>;
	fn finish_annotations(this: Self::AnnotationsResidual, annotations_visitor: Self::AnnotationsVisitor) -> Result<Self>;
	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)>;
	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self>;

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()>;
}


#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct FieldInterests {
	pub constant_value: bool,
	pub signature: bool,

	pub runtime_visible_annotations: bool,
	pub runtime_invisible_annotations: bool,
	pub runtime_visible_type_annotations: bool,
	pub runtime_invisible_type_annotations: bool,

	pub unknown_attributes: bool,
}

impl FieldInterests {
	pub fn none() -> FieldInterests {
		Self::default()
	}
	pub fn all() -> FieldInterests {
		FieldInterests {
			constant_value: true,
			signature: true,

			runtime_visible_annotations: true,
			runtime_invisible_annotations: true,
			runtime_visible_type_annotations: true,
			runtime_invisible_type_annotations: true,

			unknown_attributes: true,
		}
	}
}
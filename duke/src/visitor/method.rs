pub mod code;

use anyhow::Result;
use crate::tree::class::ClassName;
use crate::tree::method::{MethodParameter, MethodSignature};
use crate::tree::type_annotation::TargetInfoMethod;
use crate::visitor::annotation::{AnnotationsVisitor, TypeAnnotationsVisitor, UnnamedElementValueVisitor};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::method::code::CodeVisitor;

pub trait MethodVisitor
where
	Self: Sized,
	Self::AnnotationsVisitor: AnnotationsVisitor,
	Self::TypeAnnotationsVisitor: TypeAnnotationsVisitor<TargetInfoMethod>,
	Self::AnnotationDefaultVisitor: UnnamedElementValueVisitor,
	Self::CodeVisitor: CodeVisitor,
	Self::UnknownAttribute: UnknownAttributeVisitor,
{
	type AnnotationsVisitor;
	type AnnotationsResidual;
	type TypeAnnotationsVisitor;
	type TypeAnnotationsResidual;
	type AnnotationDefaultVisitor;
	type AnnotationDefaultResidual;
	type CodeVisitor;
	type UnknownAttribute;

	fn interests(&self) -> MethodInterests;

	fn visit_deprecated_and_synthetic_attribute(&mut self, deprecated: bool, synthetic: bool) -> Result<()>;

	fn visit_exceptions(&mut self, exceptions: Vec<ClassName>) -> Result<()>;
	fn visit_signature(&mut self, signature: MethodSignature) -> Result<()>;

	fn visit_annotations(self, visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)>;
	fn finish_annotations(this: Self::AnnotationsResidual, annotations_visitor: Self::AnnotationsVisitor) -> Result<Self>;
	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)>;
	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self>;

	fn visit_annotation_default(self) -> Result<(Self::AnnotationDefaultResidual, Self::AnnotationDefaultVisitor)>;
	fn finish_annotation_default(this: Self::AnnotationDefaultResidual, element_value_visitor: Self::AnnotationDefaultVisitor) -> Result<Self>;

	fn visit_parameters(&mut self, method_parameters: Vec<MethodParameter>) -> Result<()>;
	// TODO: parameter stuff; at least one of these two will vanish, the other one needs to get arguments
	fn visit_annotable_parameter_count(&mut self);
	fn visit_parameter_annotation(&mut self);

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()>;

	fn visit_code(&mut self) -> Result<Option<Self::CodeVisitor>>;
	fn finish_code(&mut self, code_visitor: Self::CodeVisitor) -> Result<()>;
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct MethodInterests {
	pub code: bool,
	pub exceptions: bool,
	pub signature: bool,

	pub runtime_visible_annotations: bool,
	pub runtime_invisible_annotations: bool,
	pub runtime_visible_type_annotations: bool,
	pub runtime_invisible_type_annotations: bool,
	pub runtime_visible_parameter_annotations: bool,
	pub runtime_invisible_parameter_annotations: bool,

	pub annotation_default: bool,
	pub method_parameters: bool,

	pub unknown_attributes: bool,
}

impl MethodInterests {
	pub fn none() -> MethodInterests {
		Self::default()
	}
	pub fn all() -> MethodInterests {
		MethodInterests {
			code: true,
			exceptions: true,
			signature: true,

			runtime_visible_annotations: true,
			runtime_invisible_annotations: true,
			runtime_visible_type_annotations: true,
			runtime_invisible_type_annotations: true,
			runtime_visible_parameter_annotations: true,
			runtime_invisible_parameter_annotations: true,

			annotation_default: true,
			method_parameters: true,

			unknown_attributes: true,
		}
	}
}

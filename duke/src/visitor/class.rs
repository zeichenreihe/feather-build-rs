use std::ops::ControlFlow;
use anyhow::Result;
use crate::tree::class::{ClassName, ClassSignature, EnclosingMethod, InnerClass};
use crate::tree::field::{FieldAccess, FieldDescriptor, FieldName};
use crate::tree::method::{MethodAccess, MethodDescriptor, MethodName};
use crate::tree::module::{Module, PackageName};
use crate::tree::record::RecordName;
use crate::tree::type_annotation::TargetInfoClass;
use crate::visitor::annotation::{AnnotationsVisitor, TypeAnnotationsVisitor};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::field::FieldVisitor;
use crate::visitor::method::MethodVisitor;
use crate::visitor::record::RecordComponentVisitor;

// TODO: write doc comments
// TODO: include what may be called multiple times, what not; also write what should give errors,
//  the impls are responsible for erroring in case of multiple visits; this also applies for the other visitor traits

pub trait ClassVisitor
where
	Self: Sized,
	Self::AnnotationsVisitor: AnnotationsVisitor,
	Self::TypeAnnotationsVisitor: TypeAnnotationsVisitor<TargetInfoClass>,
	Self::UnknownAttribute: UnknownAttributeVisitor,
	Self::FieldVisitor: FieldVisitor,
	Self::MethodVisitor: MethodVisitor,
	Self::RecordComponentVisitor: RecordComponentVisitor,
{
	type AnnotationsVisitor;
	type AnnotationsResidual;
	type TypeAnnotationsVisitor;
	type TypeAnnotationsResidual;
	type RecordComponentVisitor;
	type RecordComponentResidual;
	type FieldVisitor;
	type FieldResidual;
	type MethodVisitor;
	type MethodResidual;
	type UnknownAttribute;

	fn interests(&self) -> ClassInterests;

	fn visit_deprecated_and_synthetic_attribute(&mut self, deprecated: bool, synthetic: bool) -> Result<()>;

	fn visit_inner_classes(&mut self, inner_classes: Vec<InnerClass>) -> Result<()>;
	fn visit_enclosing_method(&mut self, enclosing_method: EnclosingMethod) -> Result<()>;
	fn visit_signature(&mut self, signature: ClassSignature) -> Result<()>;

	fn visit_source_file(&mut self, source_file: String) -> Result<()>;
	fn visit_source_debug_extension(&mut self, source_debug_extension: String) -> Result<()>;

	// TODO: check all ControlFlow usages, only use it where the ::interests isn't enough
	fn visit_annotations(self, visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)>;
	fn finish_annotations(this: Self::AnnotationsResidual, annotations_visitor: Self::AnnotationsVisitor) -> Result<Self>;
	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)>;
	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self>;

	fn visit_module(&mut self, module: Module) -> Result<()>;
	fn visit_module_packages(&mut self, module_packages: Vec<PackageName>) -> Result<()>;
	fn visit_module_main_class(&mut self, module_main_class: ClassName) -> Result<()>;

	// TODO: rename this to visit_nest_host (and arg as well), since attr is named "NestHost"
	fn visit_nest_host_class(&mut self, nest_host_class: ClassName) -> Result<()>;
	fn visit_nest_members(&mut self, nest_members: Vec<ClassName>) -> Result<()>;
	fn visit_permitted_subclasses(&mut self, permitted_subclasses: Vec<ClassName>) -> Result<()>;

	fn visit_record_component(self, name: RecordName, descriptor: FieldDescriptor)
		-> Result<ControlFlow<Self, (Self::RecordComponentResidual, Self::RecordComponentVisitor)>>;
	fn finish_record_component(this: Self::RecordComponentResidual, record_component_visitor: Self::RecordComponentVisitor) -> Result<Self>;

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()>;

	fn visit_field(self, access: FieldAccess, name: FieldName, descriptor: FieldDescriptor)
		-> Result<ControlFlow<Self, (Self::FieldResidual, Self::FieldVisitor)>>;
	fn finish_field(this: Self::FieldResidual, field_visitor: Self::FieldVisitor) -> Result<Self>;

	fn visit_method(self, access: MethodAccess, name: MethodName, descriptor: MethodDescriptor)
		-> Result<ControlFlow<Self, (Self::MethodResidual, Self::MethodVisitor)>>;
	fn finish_method(this: Self::MethodResidual, method_visitor: Self::MethodVisitor) -> Result<Self>;
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct ClassInterests {
	pub inner_classes: bool,
	pub enclosing_method: bool,
	pub signature: bool,

	pub source_file: bool,
	pub source_debug_extension: bool,

	pub runtime_visible_annotations: bool,
	pub runtime_invisible_annotations: bool,
	pub runtime_visible_type_annotations: bool,
	pub runtime_invisible_type_annotations: bool,

	pub module: bool,
	pub module_packages: bool,
	pub module_main_class: bool,

	pub nest_host: bool,
	pub nest_members: bool,

	pub permitted_subclasses: bool,
	pub record: bool,

	pub unknown_attributes: bool,

	pub fields: bool,
	pub methods: bool,
}

impl ClassInterests {
	pub fn none() -> ClassInterests {
		Self::default()
	}
	pub fn all() -> ClassInterests {
		ClassInterests {
			inner_classes: true,
			enclosing_method: true,
			signature: true,
			
			source_file: true,
			source_debug_extension: true,
			
			runtime_visible_annotations: true,
			runtime_invisible_annotations: true,
			runtime_visible_type_annotations: true,
			runtime_invisible_type_annotations: true,
			
			module: true,
			module_packages: true,
			module_main_class: true,
			
			nest_host: true,
			nest_members: true,
			
			permitted_subclasses: true,
			record: true,
			
			unknown_attributes: true,
			
			fields: true,
			methods: true,
		}
	}
}
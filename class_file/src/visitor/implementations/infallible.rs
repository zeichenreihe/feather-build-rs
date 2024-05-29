// TODO: also impl this for the never type !, once that's stable
use std::convert::Infallible;
use std::ops::ControlFlow;
use anyhow::Result;
use crate::tree::annotation::Object;
use crate::tree::class::{ClassAccess, ClassName, ClassSignature, EnclosingMethod, InnerClass};
use crate::tree::field::{ConstantValue, FieldAccess, FieldDescriptor, FieldName, FieldSignature};
use crate::tree::method::{MethodAccess, MethodDescriptor, MethodName, MethodParameter, MethodSignature};
use crate::tree::method::code::{Exception, Label, Lv};
use crate::tree::module::{Module, PackageName};
use crate::tree::record::RecordName;
use crate::tree::type_annotation::TypePath;
use crate::tree::version::Version;
use crate::visitor::annotation::{AnnotationsVisitor, NamedElementValuesVisitor, NamedElementValueVisitor, TypeAnnotationsVisitor, UnnamedElementValuesVisitor, UnnamedElementValueVisitor};
use crate::visitor::class::{ClassInterests, ClassVisitor};
use crate::visitor::field::{FieldInterests, FieldVisitor};
use crate::visitor::method::code::{CodeInterests, CodeVisitor};
use crate::visitor::method::{MethodInterests, MethodVisitor};
use crate::visitor::MultiClassVisitor;
use crate::visitor::record::{RecordComponentInterests, RecordComponentVisitor};

impl MultiClassVisitor for Infallible {
	type ClassVisitor = Infallible;
	type ClassResidual = Self;

	fn visit_class(self, _version: Version, _access: ClassAccess, _name: ClassName, _super_class: Option<ClassName>, _interfaces: Vec<ClassName>)
			-> Result<ControlFlow<Self, (Self::ClassResidual, Self::ClassVisitor)>> {
		unreachable!()
	}

	fn finish_class(_this: Self::ClassResidual, _class_visitor: Self::ClassVisitor) -> Result<Self> {
		unreachable!()
	}
}

impl ClassVisitor for Infallible {
	type AnnotationsVisitor = Infallible;
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = Infallible;
	type TypeAnnotationsResidual = Self;
	type RecordComponentVisitor = Infallible;
	type RecordComponentResidual = Self;
	type FieldVisitor = Infallible;
	type FieldResidual = Self;
	type MethodVisitor = Infallible;
	type MethodResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> ClassInterests {
		unreachable!()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, _deprecated: bool, _synthetic: bool) -> Result<()> {
		unreachable!()
	}

	fn visit_inner_classes(&mut self, _inner_classes: Vec<InnerClass>) -> Result<()> {
		unreachable!()
	}

	fn visit_enclosing_method(&mut self, _enclosing_method: EnclosingMethod) -> Result<()> {
		unreachable!()
	}

	fn visit_signature(&mut self, _signature: ClassSignature) -> Result<()> {
		unreachable!()
	}

	fn visit_source_file(&mut self, _source_file: String) -> Result<()> {
		unreachable!()
	}

	fn visit_source_debug_extension(&mut self, _source_debug_extension: String) -> Result<()> {
		unreachable!()
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_annotations(_this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_type_annotations(_this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_module(&mut self, _module: Module) -> Result<()> {
		unreachable!()
	}

	fn visit_module_packages(&mut self, _module_packages: Vec<PackageName>) -> Result<()> {
		unreachable!()
	}

	fn visit_module_main_class(&mut self, _module_main_class: ClassName) -> Result<()> {
		unreachable!()
	}

	fn visit_nest_host_class(&mut self, _nest_host_class: ClassName) -> Result<()> {
		unreachable!()
	}

	fn visit_nest_members(&mut self, _nest_members: Vec<ClassName>) -> Result<()> {
		unreachable!()
	}

	fn visit_permitted_subclasses(&mut self, _permitted_subclasses: Vec<ClassName>) -> Result<()> {
		unreachable!()
	}

	fn visit_record_component(self, _name: RecordName, _descriptor: FieldDescriptor)
			-> Result<ControlFlow<Self, (Self::RecordComponentResidual, Self::RecordComponentVisitor)>> {
		unreachable!()
	}

	fn finish_record_component(_this: Self::RecordComponentResidual, _record_component_visitor: Self::RecordComponentVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		unreachable!()
	}

	fn visit_field(self, _access: FieldAccess, _name: FieldName, _descriptor: FieldDescriptor)
			-> Result<ControlFlow<Self, (Self::FieldResidual, Self::FieldVisitor)>> {
		unreachable!()
	}

	fn finish_field(_this: Self::FieldResidual, _field_visitor: Self::FieldVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_method(self, _access: MethodAccess, _name: MethodName, _descriptor: MethodDescriptor)
			-> Result<ControlFlow<Self, (Self::MethodResidual, Self::MethodVisitor)>> {
		unreachable!()
	}

	fn finish_method(_this: Self::MethodResidual, _method_visitor: Self::MethodVisitor) -> Result<Self> {
		unreachable!()
	}
}

impl AnnotationsVisitor for Infallible {
	type NamedElementValuesVisitor = Infallible;
	type NamedElementValuesResidual = Self;

	fn visit_annotation(self, _annotation_descriptor: FieldDescriptor) -> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)> {
		unreachable!()
	}

	fn finish_annotation(_this: Self::NamedElementValuesResidual, _named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self> {
		unreachable!()
	}
}

impl<T> TypeAnnotationsVisitor<T> for Infallible {
	type NamedElementValuesVisitor = Infallible;
	type NamedElementValuesResidual = Infallible;

	fn visit_type_annotation(self, _type_reference: T, _type_path: TypePath, _annotation_descriptor: FieldDescriptor)
			-> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)> {
		unreachable!()
	}

	fn finish_type_annotation(_this: Self::NamedElementValuesResidual, _named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self> {
		unreachable!()
	}
}

impl NamedElementValuesVisitor for Infallible {}

impl NamedElementValueVisitor for Infallible {
	type AnnotationVisitor = Infallible;
	type AnnotationResidual = Self;
	type AnnotationArrayVisitor = Infallible;
	type AnnotationArrayResidual = Self;

	fn visit(&mut self, _name: String, _value: Object) -> Result<()> {
		unreachable!()
	}

	fn visit_enum(&mut self, _name: String, _type_name: FieldDescriptor, _const_name: String) -> Result<()> {
		unreachable!()
	}

	fn visit_class(&mut self, _name: String, _class: String) -> Result<()> {
		unreachable!()
	}

	fn visit_annotation(self, _name: String, _annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)> {
		unreachable!()
	}

	fn finish_annotation(_this: Self::AnnotationResidual, _annotation_visitor: Self::AnnotationVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_array(self, _name: String) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)> {
		unreachable!()
	}

	fn finish_array(_this: Self::AnnotationArrayResidual, _annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self> {
		unreachable!()
	}
}

impl UnnamedElementValuesVisitor for Infallible {}

impl UnnamedElementValueVisitor for Infallible {
	type AnnotationVisitor = Infallible;
	type AnnotationResidual = Self;
	type AnnotationArrayVisitor = Infallible;
	type AnnotationArrayResidual = Self;

	fn visit(&mut self, _value: Object) -> Result<()> {
		unreachable!()
	}

	fn visit_enum(&mut self, _type_name: FieldDescriptor, _const_name: String) -> Result<()> {
		unreachable!()
	}

	fn visit_class(&mut self, _class: String) -> Result<()> {
		unreachable!()
	}

	fn visit_annotation(self, _annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)> {
		unreachable!()
	}

	fn finish_annotation(_this: Self::AnnotationResidual, _annotation_visitor: Self::AnnotationVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_array(self) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)> {
		unreachable!()
	}

	fn finish_array(_this: Self::AnnotationArrayResidual, _annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self> {
		unreachable!()
	}
}

impl FieldVisitor for Infallible {
	type AnnotationsVisitor = Infallible;
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = Infallible;
	type TypeAnnotationsResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> FieldInterests {
		unreachable!()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, _deprecated: bool, _synthetic: bool) -> Result<()> {
		unreachable!()
	}

	fn visit_constant_value(&mut self, _constant_value: ConstantValue) -> Result<()> {
		unreachable!()
	}

	fn visit_signature(&mut self, _signature: FieldSignature) -> Result<()> {
		unreachable!()
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_annotations(_this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_type_annotations(_this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		unreachable!()
	}
}

impl MethodVisitor for Infallible {
	type AnnotationsVisitor = Infallible;
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = Infallible;
	type TypeAnnotationsResidual = Self;
	type AnnotationDefaultVisitor = Infallible;
	type AnnotationDefaultResidual = Self;
	type CodeVisitor = Infallible;
	type UnknownAttribute = ();

	fn interests(&self) -> MethodInterests {
		unreachable!()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, _deprecated: bool, _synthetic: bool) -> Result<()> {
		unreachable!()
	}

	fn visit_exceptions(&mut self, _exceptions: Vec<ClassName>) -> Result<()> {
		unreachable!()
	}

	fn visit_signature(&mut self, _signature: MethodSignature) -> Result<()> {
		unreachable!()
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_annotations(_this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_type_annotations(_this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_annotation_default(self) -> Result<(Self::AnnotationDefaultResidual, Self::AnnotationDefaultVisitor)> {
		unreachable!()
	}

	fn finish_annotation_default(_this: Self::AnnotationDefaultResidual, _element_value_visitor: Self::AnnotationDefaultVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_parameters(&mut self, _method_parameters: Vec<MethodParameter>) -> Result<()> {
		unreachable!()
	}

	fn visit_annotable_parameter_count(&mut self) {
		unreachable!()
	}

	fn visit_parameter_annotation(&mut self) {
		unreachable!()
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		unreachable!()
	}

	fn visit_code(&mut self) -> Result<Option<Self::CodeVisitor>> {
		unreachable!()
	}

	fn finish_code(&mut self, _code_visitor: Self::CodeVisitor) -> Result<()> {
		unreachable!()
	}
}

impl CodeVisitor for Infallible {
	type TypeAnnotationsVisitor = Infallible;
	type TypeAnnotationsResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> CodeInterests {
		unreachable!()
	}

	fn visit_max_stack_and_max_locals(&mut self, _max_stack: u16, _max_locals: u16) -> Result<()> {
		unreachable!()
	}

	fn visit_exception_table(&mut self, _exception_table: Vec<Exception>) -> Result<()> {
		unreachable!()
	}

	fn visit_last_label(&mut self, _last_label: Label) -> Result<()> {
		unreachable!()
	}

	fn visit_line_numbers(&mut self, _line_number_table: Vec<(Label, u16)>) -> Result<()> {
		unreachable!()
	}

	fn visit_local_variables(&mut self, _local_variables: Vec<Lv>) -> Result<()> {
		unreachable!()
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_type_annotations(_this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		unreachable!()
	}
}

impl RecordComponentVisitor for Infallible {
	type AnnotationsVisitor = Infallible;
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = Infallible;
	type TypeAnnotationsResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> RecordComponentInterests {
		unreachable!()
	}

	fn visit_signature(&mut self, _signature: FieldSignature) -> Result<()> {
		unreachable!()
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_annotations(_this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		unreachable!()
	}

	fn finish_type_annotations(_this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		unreachable!()
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		unreachable!()
	}
}
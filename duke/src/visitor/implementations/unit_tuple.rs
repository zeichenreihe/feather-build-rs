use std::ops::ControlFlow;
use anyhow::Result;
use java_string::JavaString;
use crate::class_reader::pool::PoolRead;
use crate::tree::annotation::Object;
use crate::tree::attribute::Attribute;
use crate::tree::class::{ClassAccess, ClassName, ClassSignature, EnclosingMethod, InnerClass};
use crate::tree::descriptor::ReturnDescriptor;
use crate::tree::field::{ConstantValue, FieldAccess, FieldDescriptor, FieldName, FieldSignature};
use crate::tree::method::{MethodAccess, MethodDescriptor, MethodName, MethodParameter, MethodSignature};
use crate::tree::method::code::{Exception, Label, Lv};
use crate::tree::module::{Module, PackageName};
use crate::tree::record::RecordName;
use crate::tree::type_annotation::TypePath;
use crate::tree::version::Version;
use crate::visitor::annotation::{UnnamedElementValueVisitor, NamedElementValueVisitor, NamedElementValuesVisitor, UnnamedElementValuesVisitor, AnnotationsVisitor, TypeAnnotationsVisitor};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::class::{ClassInterests, ClassVisitor};
use crate::visitor::field::{FieldInterests, FieldVisitor};
use crate::visitor::method::code::{CodeInterests, CodeVisitor};
use crate::visitor::method::{MethodInterests, MethodVisitor};
use crate::visitor::MultiClassVisitor;
use crate::visitor::record::{RecordComponentInterests, RecordComponentVisitor};

// TODO: ensure this stuff here returns Interests::all() always, and ControlFlow::Continue(...) or Some(()) instead of ControlFlow::Break(...) or None!
//  bc we want the impl for () to try all code paths, while we have std::convert::Infallible (or ! in the future)
//  for the other case, where we don't want to visit any members any deeper than needed

impl MultiClassVisitor for () {
	type ClassVisitor = ();
	type ClassResidual = Self;

	fn visit_class(self, _version: Version, _access: ClassAccess, _name: ClassName, _super_class: Option<ClassName>, _interfaces: Vec<ClassName>)
			-> Result<ControlFlow<Self, (Self::ClassResidual, Self::ClassVisitor)>> {
		Ok(ControlFlow::Continue((self, ())))
	}

	fn finish_class(this: Self::ClassResidual, _class_visitor: Self::ClassVisitor) -> Result<Self> {
		Ok(this)
	}
}

impl ClassVisitor for () {
	type AnnotationsVisitor = ();
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = ();
	type TypeAnnotationsResidual = Self;
	type RecordComponentVisitor = ();
	type RecordComponentResidual = Self;
	type FieldVisitor = ();
	type FieldResidual = Self;
	type MethodVisitor = ();
	type MethodResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> ClassInterests {
		ClassInterests::all()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, _deprecated: bool, _synthetic: bool) -> Result<()> {
		Ok(())
	}

	fn visit_inner_classes(&mut self, _inner_classes: Vec<InnerClass>) -> Result<()> {
		Ok(())
	}

	fn visit_enclosing_method(&mut self, _enclosing_method: EnclosingMethod) -> Result<()> {
		Ok(())
	}

	fn visit_signature(&mut self, _signature: ClassSignature) -> Result<()> {
		Ok(())
	}

	fn visit_source_file(&mut self, _source_file: JavaString) -> Result<()> {
		Ok(())
	}

	fn visit_source_debug_extension(&mut self, _source_debug_extension: JavaString) -> Result<()> {
		Ok(())
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self, Self::AnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotations(this: Self, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_module(&mut self, _module: Module) -> Result<()> {
		Ok(())
	}

	fn visit_module_packages(&mut self, _module_packages: Vec<PackageName>) -> Result<()> {
		Ok(())
	}

	fn visit_module_main_class(&mut self, _module_main_class: ClassName) -> Result<()> {
		Ok(())
	}

	fn visit_nest_host_class(&mut self, _nest_host_class: ClassName) -> Result<()> {
		Ok(())
	}

	fn visit_nest_members(&mut self, _nest_members: Vec<ClassName>) -> Result<()> {
		Ok(())
	}

	fn visit_permitted_subclasses(&mut self, _permitted_subclasses: Vec<ClassName>) -> Result<()> {
		Ok(())
	}

	fn visit_record_component(self, _name: RecordName, _descriptor: FieldDescriptor) -> Result<ControlFlow<Self, (Self, Self::RecordComponentVisitor)>> {
		Ok(ControlFlow::Continue((self, ())))
	}

	fn finish_record_component(this: Self, _record_component_visitor: Self::RecordComponentVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		Ok(())
	}

	fn visit_field(self, _access: FieldAccess, _name: FieldName, _descriptor: FieldDescriptor) -> Result<ControlFlow<Self, (Self, Self::FieldVisitor)>> {
		Ok(ControlFlow::Continue((self, ())))
	}

	fn finish_field(this: Self, _field_visitor: Self::FieldVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_method(self, _access: MethodAccess, _name: MethodName, _descriptor: MethodDescriptor) -> Result<ControlFlow<Self, (Self, Self::MethodVisitor)>> {
		Ok(ControlFlow::Continue((self, ())))
	}

	fn finish_method(this: Self, _method_visitor: Self::MethodVisitor) -> Result<Self> {
		Ok(this)
	}
}

impl AnnotationsVisitor for () {
	type NamedElementValuesVisitor = ();
	type NamedElementValuesResidual = Self;

	fn visit_annotation(self, _annotation_descriptor: FieldDescriptor) -> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotation(this: Self::NamedElementValuesResidual, _named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self> {
		Ok(this)
	}
}

impl<T> TypeAnnotationsVisitor<T> for () {
	type NamedElementValuesVisitor = ();
	type NamedElementValuesResidual = Self;

	fn visit_type_annotation(self, _type_reference: T, _type_path: TypePath, _annotation_descriptor: FieldDescriptor)
			-> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)> {
		Ok((self, ()))
	}

	fn finish_type_annotation(this: Self::NamedElementValuesResidual, _named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self> {
		Ok(this)
	}
}

impl NamedElementValuesVisitor for () {}

impl NamedElementValueVisitor for () {
	type AnnotationVisitor = ();
	type AnnotationResidual = Self;
	type AnnotationArrayVisitor = ();
	type AnnotationArrayResidual = Self;

	fn visit(&mut self, _name: JavaString, _value: Object) -> Result<()> {
		Ok(())
	}

	fn visit_enum(&mut self, _name: JavaString, _type_name: FieldDescriptor, _const_name: JavaString) -> Result<()> {
		Ok(())
	}

	fn visit_class(&mut self, _name: JavaString, _class: ReturnDescriptor) -> Result<()> {
		Ok(())
	}

	fn visit_annotation(self, _name: JavaString, _annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotation(this: Self::AnnotationResidual, _annotation_visitor: Self::AnnotationVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_array(self, _name: JavaString) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)> {
		Ok((self, ()))
	}

	fn finish_array(this: Self::AnnotationArrayResidual, _annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self> {
		Ok(this)
	}
}

impl UnnamedElementValuesVisitor for () {}

impl UnnamedElementValueVisitor for () {
	type AnnotationVisitor = ();
	type AnnotationResidual = Self;
	type AnnotationArrayVisitor = ();
	type AnnotationArrayResidual = Self;

	fn visit(&mut self, _value: Object) -> Result<()> {
		Ok(())
	}

	fn visit_enum(&mut self, _type_name: FieldDescriptor, _const_name: JavaString) -> Result<()> {
		Ok(())
	}

	fn visit_class(&mut self, _class: ReturnDescriptor) -> Result<()> {
		Ok(())
	}

	fn visit_annotation(self, _annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotation(this: Self::AnnotationResidual, _annotation_visitor: Self::AnnotationVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_array(self) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)> {
		Ok((self, ()))
	}

	fn finish_array(this: Self::AnnotationArrayResidual, _annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self> {
		Ok(this)
	}
}

impl UnknownAttributeVisitor for () {
	fn read(_name: JavaString, _bytes: Vec<u8>, _pool: &PoolRead) -> Result<Self> {
		Ok(())
	}

	fn from_attribute(_attribute: Attribute) -> Result<Option<Self>> {
		Ok(None)
	}
}

impl FieldVisitor for () {
	type AnnotationsVisitor = ();
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = ();
	type TypeAnnotationsResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> FieldInterests {
		FieldInterests::all()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, _deprecated: bool, _synthetic: bool) -> Result<()> {
		Ok(())
	}

	fn visit_constant_value(&mut self, _constant_value: ConstantValue) -> Result<()> {
		Ok(())
	}

	fn visit_signature(&mut self, _signature: FieldSignature) -> Result<()> {
		Ok(())
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotations(this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		Ok(())
	}
}

impl MethodVisitor for () {
	type AnnotationsVisitor = ();
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = ();
	type TypeAnnotationsResidual = Self;
	type AnnotationDefaultVisitor = ();
	type AnnotationDefaultResidual = Self;
	type CodeVisitor = ();
	type UnknownAttribute = ();

	fn interests(&self) -> MethodInterests {
		MethodInterests::all()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, _deprecated: bool, _synthetic: bool) -> Result<()> {
		Ok(())
	}

	fn visit_exceptions(&mut self, _exceptions: Vec<ClassName>) -> Result<()> {
		Ok(())
	}

	fn visit_signature(&mut self, _signature: MethodSignature) -> Result<()> {
		Ok(())
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotations(this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_annotation_default(self) -> Result<(Self::AnnotationDefaultResidual, Self::AnnotationDefaultVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotation_default(this: Self::AnnotationDefaultResidual, _element_value_visitor: Self::AnnotationDefaultVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_parameters(&mut self, _method_parameters: Vec<MethodParameter>) -> Result<()> {
		Ok(())
	}

	fn visit_annotable_parameter_count(&mut self) {
		todo!()
	}

	fn visit_parameter_annotation(&mut self) {
		todo!()
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		Ok(())
	}

	fn visit_code(&mut self) -> Result<Option<Self::CodeVisitor>> {
		Ok(Some(()))
	}

	fn finish_code(&mut self, _code_visitor: Self::CodeVisitor) -> Result<()> {
		Ok(())
	}
}

impl CodeVisitor for () {
	type TypeAnnotationsVisitor = ();
	type TypeAnnotationsResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> CodeInterests {
		CodeInterests::all()
	}

	fn visit_max_stack_and_max_locals(&mut self, _max_stack: u16, _max_locals: u16) -> Result<()> {
		Ok(())
	}

	fn visit_exception_table(&mut self, _exception_table: Vec<Exception>) -> Result<()> {
		Ok(())
	}

	fn visit_last_label(&mut self, _last_label: Label) -> Result<()> {
		Ok(())
	}

	fn visit_line_numbers(&mut self, _line_number_table: Vec<(Label, u16)>) -> Result<()> {
		Ok(())
	}

	fn visit_local_variables(&mut self, _local_variables: Vec<Lv>) -> Result<()> {
		Ok(())
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		Ok(())
	}
}

impl RecordComponentVisitor for () {
	type AnnotationsVisitor = ();
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = ();
	type TypeAnnotationsResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> RecordComponentInterests {
		RecordComponentInterests::all()
	}

	fn visit_signature(&mut self, _signature: FieldSignature) -> Result<()> {
		Ok(())
	}

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_annotations(this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok((self, ()))
	}

	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, _type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		Ok(())
	}
}
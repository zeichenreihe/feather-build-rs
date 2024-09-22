use std::convert::Infallible;
use std::ops::ControlFlow;
use anyhow::Result;
use java_string::JavaString;
use crate::tree::class::{ClassName, ClassSignature, EnclosingMethod, InnerClass};
use crate::tree::field::{FieldAccess, FieldDescriptor, FieldName};
use crate::tree::method::{MethodAccess, MethodDescriptor, MethodName};
use crate::tree::module::{Module, PackageName};
use crate::tree::record::RecordName;
use crate::visitor::class::{ClassInterests, ClassVisitor};
use crate::visitor::field::FieldVisitor;
use crate::visitor::method::MethodVisitor;

pub trait SimpleClassVisitor
where
	Self::FieldVisitor: FieldVisitor,
	Self::MethodVisitor: MethodVisitor,
{
	type FieldVisitor;
	type MethodVisitor;

	fn visit_field(
		&mut self,
		access: FieldAccess,
		name: FieldName,
		descriptor: FieldDescriptor,
	) -> Result<Option<Self::FieldVisitor>>;
	fn finish_field(
		&mut self,
		field_visitor: Self::FieldVisitor,
	) -> Result<()>;

	fn visit_method(
		&mut self,
		access: MethodAccess,
		name: MethodName,
		descriptor: MethodDescriptor,
	) -> Result<Option<Self::MethodVisitor>>;
	fn finish_method(
		&mut self,
		method_visitor: Self::MethodVisitor,
	) -> Result<()>;
}

impl<T> ClassVisitor for T where T: SimpleClassVisitor {
	type AnnotationsVisitor = Infallible;
	type AnnotationsResidual = Self;
	type TypeAnnotationsVisitor = Infallible;
	type TypeAnnotationsResidual = Self;
	type RecordComponentVisitor = Infallible;
	type RecordComponentResidual = Self;
	type FieldVisitor = T::FieldVisitor;
	type FieldResidual = Self;
	type MethodVisitor = T::MethodVisitor;
	type MethodResidual = Self;
	type UnknownAttribute = ();

	fn interests(&self) -> ClassInterests {
		ClassInterests {
			fields: true,
			methods: true,
			..ClassInterests::none()
		}
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

	fn visit_annotations(self, _visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		unreachable!()
	}
	fn finish_annotations(this: Self::AnnotationsResidual, _annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_type_annotations(self, _visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		unreachable!()
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

	fn visit_record_component(self, _name: RecordName, _descriptor: FieldDescriptor)
			-> Result<ControlFlow<Self, (Self::RecordComponentResidual, Self::RecordComponentVisitor)>> {
		Ok(ControlFlow::Break(self))
	}

	fn finish_record_component(this: Self::RecordComponentResidual, _record_component_visitor: Self::RecordComponentVisitor) -> Result<Self> {
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, _unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		Ok(())
	}

	fn visit_field(mut self, access: FieldAccess, name: FieldName, descriptor: FieldDescriptor)
			-> Result<ControlFlow<Self, (Self::FieldResidual, Self::FieldVisitor)>> {
		if let Some(fv) = SimpleClassVisitor::visit_field(&mut self, access, name, descriptor)? {
			Ok(ControlFlow::Continue((self, fv)))
		} else {
			Ok(ControlFlow::Break(self))
		}
	}

	fn finish_field(mut this: Self::FieldResidual, field_visitor: Self::FieldVisitor) -> Result<Self> {
		SimpleClassVisitor::finish_field(&mut this, field_visitor)?;
		Ok(this)
	}

	fn visit_method(mut self, access: MethodAccess, name: MethodName, descriptor: MethodDescriptor)
			-> Result<ControlFlow<Self, (Self::MethodResidual, Self::MethodVisitor)>> {
		if let Some(mv) = SimpleClassVisitor::visit_method(&mut self, access, name, descriptor)? {
			Ok(ControlFlow::Continue((self, mv)))
		} else {
			Ok(ControlFlow::Break(self))
		}
	}

	fn finish_method(mut this: Self::MethodResidual, method_visitor: Self::MethodVisitor) -> Result<Self> {
		SimpleClassVisitor::finish_method(&mut this, method_visitor)?;
		Ok(this)
	}
}
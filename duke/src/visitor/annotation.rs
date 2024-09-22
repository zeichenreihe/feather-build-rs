use anyhow::Result;
use java_string::JavaString;
use crate::tree::annotation::Object;
use crate::tree::descriptor::ReturnDescriptor;
use crate::tree::field::FieldDescriptor;
use crate::tree::type_annotation::TypePath;

pub trait AnnotationsVisitor
where
	Self: Sized,
	Self::NamedElementValuesVisitor: NamedElementValuesVisitor,
{
	type NamedElementValuesVisitor;
	type NamedElementValuesResidual;

	fn visit_annotation(self, annotation_descriptor: FieldDescriptor) -> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)>;
	fn finish_annotation(this: Self::NamedElementValuesResidual, named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self>;
}

pub trait TypeAnnotationsVisitor<T>
where
	Self: Sized,
	Self::NamedElementValuesVisitor: NamedElementValuesVisitor,
{
	type NamedElementValuesVisitor;
	type NamedElementValuesResidual;

	fn visit_type_annotation(self, type_reference: T, type_path: TypePath, annotation_descriptor: FieldDescriptor)
		-> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)>;

	fn finish_type_annotation(this: Self::NamedElementValuesResidual, named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self>;
}

/// Signals that the visitor can visit multiple elements.
pub trait NamedElementValuesVisitor: NamedElementValueVisitor + Sized {}

pub trait NamedElementValueVisitor
where
	Self: Sized,
	Self::AnnotationVisitor: NamedElementValuesVisitor,
	Self::AnnotationArrayVisitor: UnnamedElementValuesVisitor,
{
	type AnnotationVisitor;
	type AnnotationResidual;
	type AnnotationArrayVisitor;
	type AnnotationArrayResidual;

	fn visit(&mut self, name: JavaString, value: Object) -> Result<()>;

	fn visit_enum(
		&mut self,
		name: JavaString,
		type_name: FieldDescriptor,
		const_name: JavaString,
	) -> Result<()>;

	fn visit_class(&mut self, name: JavaString, class: ReturnDescriptor) -> Result<()>;

	fn visit_annotation(self, name: JavaString, annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)>;
	fn finish_annotation(this: Self::AnnotationResidual, annotation_visitor: Self::AnnotationVisitor) -> Result<Self>;

	fn visit_array(self, name: JavaString) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)>;
	fn finish_array(this: Self::AnnotationArrayResidual, annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self>;
}

/// Signals that the visitor can visit multiple elements.
pub trait UnnamedElementValuesVisitor: UnnamedElementValueVisitor {}

pub trait UnnamedElementValueVisitor
where
	Self: Sized,
	Self::AnnotationVisitor: NamedElementValuesVisitor,
	Self::AnnotationArrayVisitor: UnnamedElementValuesVisitor,
{
	type AnnotationVisitor;
	type AnnotationResidual;
	type AnnotationArrayVisitor;
	type AnnotationArrayResidual;

	fn visit(&mut self, value: Object) -> Result<()>;

	fn visit_enum(
		&mut self,
		type_name: FieldDescriptor,
		const_name: JavaString,
	) -> Result<()>;

	fn visit_class(&mut self, class: ReturnDescriptor) -> Result<()>;

	fn visit_annotation(self, annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)>;
	fn finish_annotation(this: Self::AnnotationResidual, annotation_visitor: Self::AnnotationVisitor) -> Result<Self>;

	fn visit_array(self) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)>;
	fn finish_array(this: Self::AnnotationArrayResidual, annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self>;
}
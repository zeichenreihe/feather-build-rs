use anyhow::Result;
use std::fmt::{Debug, Display, Formatter};
use std::ops::ControlFlow;
use crate::macros::make_string_str_like;
use crate::tree::annotation::Annotation;
use crate::tree::attribute::Attribute;
use crate::tree::field::{FieldDescriptor, FieldSignature};
use crate::tree::type_annotation::{TargetInfoField, TypeAnnotation};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::class::ClassVisitor;
use crate::visitor::record::RecordComponentVisitor;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordComponent {
	pub name: RecordName,
	pub descriptor: FieldDescriptor,

	pub(crate) signature: Option<FieldSignature>,

	pub(crate) runtime_visible_annotations: Vec<Annotation>,
	pub(crate) runtime_invisible_annotations: Vec<Annotation>,
	pub(crate) runtime_visible_type_annotations: Vec<TypeAnnotation<TargetInfoField>>,
	pub(crate) runtime_invisible_type_annotations: Vec<TypeAnnotation<TargetInfoField>>,

	pub(crate) attributes: Vec<Attribute>,
}

impl RecordComponent {
	pub fn new(name: RecordName, descriptor: FieldDescriptor) -> RecordComponent {
		RecordComponent {
			name,
			descriptor,

			signature: None,

			runtime_visible_annotations: Vec::new(),
			runtime_invisible_annotations: Vec::new(),
			runtime_visible_type_annotations: Vec::new(),
			runtime_invisible_type_annotations: Vec::new(),

			attributes: Vec::new(),
		}
	}

	pub fn accept<C: ClassVisitor>(self, visitor: C) -> Result<C> {
		match visitor.visit_record_component(self.name, self.descriptor)? {
			ControlFlow::Continue((visitor, mut record_component_visitor)) => {
				let interests = record_component_visitor.interests();

				if interests.signature {
					if let Some(signature) = self.signature {
						record_component_visitor.visit_signature(signature)?;
					}
				}

				if interests.runtime_visible_annotations && !self.runtime_visible_annotations.is_empty() {
					let (visitor, mut annotations_visitor) = record_component_visitor.visit_annotations(true)?;
					for annotation in self.runtime_visible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					record_component_visitor = RecordComponentVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if interests.runtime_invisible_annotations && !self.runtime_invisible_annotations.is_empty() {
					let (visitor, mut annotations_visitor) = record_component_visitor.visit_annotations(false)?;
					for annotation in self.runtime_invisible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					record_component_visitor = RecordComponentVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if interests.runtime_visible_type_annotations && !self.runtime_visible_type_annotations.is_empty() {
					let (visitor, mut type_annotations_visitor) = record_component_visitor.visit_type_annotations(true)?;
					for annotation in self.runtime_visible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					record_component_visitor = RecordComponentVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}
				if interests.runtime_invisible_type_annotations && !self.runtime_invisible_type_annotations.is_empty() {
					let (visitor, mut type_annotations_visitor) = record_component_visitor.visit_type_annotations(false)?;
					for annotation in self.runtime_invisible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					record_component_visitor = RecordComponentVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}

				if interests.unknown_attributes {
					for attribute in self.attributes {
						if let Some(attribute) = UnknownAttributeVisitor::from_attribute(attribute)? {
							record_component_visitor.visit_unknown_attribute(attribute)?;
						}
					}
				}

				ClassVisitor::finish_record_component(visitor, record_component_visitor)
			}
			ControlFlow::Break(visitor) => Ok(visitor)
		}
	}
}

make_string_str_like!(RecordName, RecordNameSlice);

impl Display for RecordName {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		Display::fmt(self.as_slice(), f)
	}
}
impl Display for RecordNameSlice {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}


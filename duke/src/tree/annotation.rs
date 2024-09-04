use anyhow::Result;
use std::fmt::{Debug, Formatter};
use crate::tree::descriptor::ReturnDescriptor;
use crate::tree::field::FieldDescriptor;
use crate::visitor::annotation::{AnnotationsVisitor, NamedElementValuesVisitor, UnnamedElementValuesVisitor, UnnamedElementValueVisitor};

#[derive(Clone, PartialEq)]
pub struct Annotation {
	pub annotation_type: FieldDescriptor,
	pub element_value_pairs: Vec<ElementValuePair>,
}

impl Annotation {
	pub fn new(annotation_type: FieldDescriptor) -> Annotation {
		Annotation {
			annotation_type,
			element_value_pairs: Vec::new(),
		}
	}

	pub fn accept<A: AnnotationsVisitor>(self, visitor: A) -> Result<A> {
		let (visitor, mut pairs_visitor) = visitor.visit_annotation(self.annotation_type)?;

		pairs_visitor = accept_element_values_named(pairs_visitor, self.element_value_pairs)?;

		AnnotationsVisitor::finish_annotation(visitor, pairs_visitor)
	}
}

pub(crate) fn accept_element_values_named<A: NamedElementValuesVisitor>(mut outer: A, pairs: Vec<ElementValuePair>) -> Result<A> {
	for pair in pairs {
		match pair.value {
			ElementValue::Object(object) => {
				outer.visit(pair.name, object)?;
			}
			ElementValue::Enum { type_name, const_name } => {
				outer.visit_enum(pair.name, type_name, const_name)?;
			}
			ElementValue::Class(class) => {
				outer.visit_class(pair.name, class)?;
			}
			ElementValue::AnnotationInterface(annotation) => {
				let (visitor, mut inner) = outer.visit_annotation(pair.name, annotation.annotation_type)?;
				inner = accept_element_values_named(inner, annotation.element_value_pairs)?;
				outer = A::finish_annotation(visitor, inner)?;
			}
			ElementValue::ArrayType(element_values) => {
				let (visitor, mut inner) = outer.visit_array(pair.name)?;
				inner = accept_element_values_unnamed(inner, element_values)?;
				outer = A::finish_array(visitor, inner)?;
			}
		}
	}

	Ok(outer)
}

fn accept_element_values_unnamed<A: UnnamedElementValuesVisitor>(mut outer: A, element_values: Vec<ElementValue>) -> Result<A> {
	for value in element_values {
		outer = value.accept(outer)?;
	}

	Ok(outer)
}

impl ElementValue {
	pub fn accept<A: UnnamedElementValueVisitor>(self, mut outer: A) -> Result<A> {
		match self {
			ElementValue::Object(object) => {
				outer.visit(object)?;
			}
			ElementValue::Enum { type_name, const_name } => {
				outer.visit_enum(type_name, const_name)?;
			}
			ElementValue::Class(class) => {
				outer.visit_class(class)?;
			}
			ElementValue::AnnotationInterface(annotation) => {
				let (visitor, mut inner) = outer.visit_annotation(annotation.annotation_type)?;
				inner = accept_element_values_named(inner, annotation.element_value_pairs)?;
				outer = A::finish_annotation(visitor, inner)?;
			}
			ElementValue::ArrayType(element_values) => {
				let (visitor, mut inner) = outer.visit_array()?;
				inner = accept_element_values_unnamed(inner, element_values)?;
				outer = A::finish_array(visitor, inner)?;
			}
		}

		Ok(outer)
	}
}



impl Debug for Annotation {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "@{:?}", self.annotation_type)?;
		f.debug_map()
			.entries(self.element_value_pairs.iter()
				.map(|pair| (&pair.name, &pair.value))
			)
			.finish()
	}
}

#[derive(Clone, PartialEq)]
pub struct ElementValuePair {
	pub name: String,
	pub value: ElementValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElementValue {
	Object(Object),
	Enum {
		type_name: FieldDescriptor,
		const_name: String /* TODO: name of the constant */,
	},
	Class(ReturnDescriptor),
	AnnotationInterface(Annotation),
	ArrayType(Vec<ElementValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
	Byte(i8),
	Char(u16),
	Double(f64),
	Float(f32),
	Integer(i32),
	Long(i64),
	Short(i16),
	Boolean(bool),
	String(String),
}
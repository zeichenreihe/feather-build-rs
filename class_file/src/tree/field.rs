use std::borrow::Cow;
use anyhow::Result;
use std::fmt::{Debug, Display, Formatter};
use std::ops::ControlFlow;
use crate::macros::{from_impl_for_string_and_str, partial_eq_impl_for_str};
use crate::tree::annotation::Annotation;
use crate::tree::attribute::Attribute;
use crate::tree::class::ClassName;
use crate::tree::type_annotation::{TargetInfoField, TypeAnnotation};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::class::ClassVisitor;
use crate::visitor::field::FieldVisitor;

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
	pub access: FieldAccess,
	pub name: FieldName,
	pub descriptor: FieldDescriptor,

	pub has_deprecated_attribute: bool,
	pub has_synthetic_attribute: bool,

	pub constant_value: Option<ConstantValue>,
	pub signature: Option<FieldSignature>,

	pub runtime_visible_annotations: Vec<Annotation>,
	pub runtime_invisible_annotations: Vec<Annotation>,
	pub runtime_visible_type_annotations: Vec<TypeAnnotation<TargetInfoField>>,
	pub runtime_invisible_type_annotations: Vec<TypeAnnotation<TargetInfoField>>,

	pub attributes: Vec<Attribute>,
}

impl Field {
	pub fn new(access: FieldAccess, name: FieldName, descriptor: FieldDescriptor) -> Field {
		Field {
			access,
			name,
			descriptor,

			has_deprecated_attribute: false,
			has_synthetic_attribute: false,

			constant_value: None,
			signature: None,

			runtime_visible_annotations: Vec::new(),
			runtime_invisible_annotations: Vec::new(),
			runtime_visible_type_annotations: Vec::new(),
			runtime_invisible_type_annotations: Vec::new(),

			attributes: Vec::new(),
		}
	}

	pub fn accept<C: ClassVisitor>(self, visitor: C) -> Result<C> {
		match visitor.visit_field(self.access, self.name, self.descriptor)? {
			ControlFlow::Continue((visitor, mut field_visitor)) => {
				let interests = field_visitor.interests(); // TODO: make even more use of them

				field_visitor.visit_deprecated_and_synthetic_attribute(self.has_deprecated_attribute, self.has_synthetic_attribute)?;

				if let Some(constant_value) = self.constant_value {
					field_visitor.visit_constant_value(constant_value)?;
				}
				if let Some(signature) = self.signature {
					field_visitor.visit_signature(signature)?;
				}

				if !self.runtime_visible_annotations.is_empty() && interests.runtime_visible_annotations {
					let (visitor, mut annotations_visitor) = field_visitor.visit_annotations(true)?;
					for annotation in self.runtime_visible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					field_visitor = FieldVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if !self.runtime_invisible_annotations.is_empty() && interests.runtime_invisible_annotations {
					let (visitor, mut annotations_visitor) = field_visitor.visit_annotations(false)?;
					for annotation in self.runtime_invisible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					field_visitor = FieldVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if !self.runtime_visible_type_annotations.is_empty() && interests.runtime_visible_type_annotations {
					let (visitor, mut type_annotations_visitor) = field_visitor.visit_type_annotations(true)?;
					for annotation in self.runtime_visible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					field_visitor = FieldVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}
				if !self.runtime_invisible_type_annotations.is_empty() && interests.runtime_invisible_type_annotations {
					let (visitor, mut type_annotations_visitor) = field_visitor.visit_type_annotations(false)?;
					for annotation in self.runtime_invisible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					field_visitor = FieldVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}

				for attribute in self.attributes {
					if let Some(attribute) = UnknownAttributeVisitor::from_attribute(attribute)? {
						field_visitor.visit_unknown_attribute(attribute)?;
					}
				}

				ClassVisitor::finish_field(visitor, field_visitor)
			}
			ControlFlow::Break(visitor) => Ok(visitor)
		}
	}
}

#[derive(Copy, Clone, PartialEq)]
pub struct FieldAccess {
	pub is_public: bool,
	pub is_private: bool,
	pub is_protected: bool,
	pub is_static: bool,
	pub is_final: bool,
	pub is_volatile: bool,
	pub is_transient: bool,
	pub is_synthetic: bool,
	pub is_enum: bool,
}

impl Debug for FieldAccess {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("FieldAccess { ")?;
		if self.is_public     { f.write_str("public ")?; }
		if self.is_private    { f.write_str("private ")?; }
		if self.is_protected  { f.write_str("protected ")?; }
		if self.is_static     { f.write_str("static ")?; }
		if self.is_final      { f.write_str("final ")?; }
		if self.is_volatile   { f.write_str("volatile ")?; }
		if self.is_transient  { f.write_str("transient ")?; }
		if self.is_synthetic  { f.write_str("synthetic ")?; }
		if self.is_enum       { f.write_str("enum ")?; }
		f.write_str("}")
	}
}

impl From<u16> for FieldAccess {
	fn from(value: u16) -> Self {
		FieldAccess {
			is_public:    value & 0x0001 != 0,
			is_private:   value & 0x0002 != 0,
			is_protected: value & 0x0004 != 0,
			is_static:    value & 0x0008 != 0,
			is_final:     value & 0x0010 != 0,
			is_volatile:  value & 0x0040 != 0,
			is_transient: value & 0x0080 != 0,
			is_synthetic: value & 0x1000 != 0,
			is_enum:      value & 0x4000 != 0,
		}
	}
}

impl From<FieldAccess> for u16 {
	fn from(value: FieldAccess) -> Self {
		(if value.is_public    { 0x0001 } else { 0 }) |
		(if value.is_private   { 0x0002 } else { 0 }) |
		(if value.is_protected { 0x0004 } else { 0 }) |
		(if value.is_static    { 0x0008 } else { 0 }) |
		(if value.is_final     { 0x0010 } else { 0 }) |
		(if value.is_volatile  { 0x0040 } else { 0 }) |
		(if value.is_transient { 0x0080 } else { 0 }) |
		(if value.is_synthetic { 0x1000 } else { 0 }) |
		(if value.is_enum      { 0x4000 } else { 0 })
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FieldRef {
	pub class: ClassName,
	pub name: FieldName,
	pub desc: FieldDescriptor,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FieldName(Cow<'static, str>);

impl Display for FieldName {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

from_impl_for_string_and_str!(FieldName);
partial_eq_impl_for_str!(FieldName);

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FieldDescriptor(Cow<'static, str>);

from_impl_for_string_and_str!(FieldDescriptor);
partial_eq_impl_for_str!(FieldDescriptor);

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FieldSignature(Cow<'static, str>);

from_impl_for_string_and_str!(FieldSignature);
partial_eq_impl_for_str!(FieldSignature);

#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
	/// Also represents the value for a field of type `byte`, `char`, `short`, `boolean`.
	Integer(i32),
	Float(f32),
	Long(i64),
	Double(f64),
	String(String),
}

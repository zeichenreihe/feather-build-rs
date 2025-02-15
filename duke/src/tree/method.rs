pub mod code;

use anyhow::{bail, Result};
use std::fmt::{Debug, Display, Formatter};
use std::ops::ControlFlow;
use java_string::{JavaStr, JavaString};
use crate::macros::{make_display, make_string_str_like};
use crate::tree::annotation::{Annotation, ElementValue};
use crate::tree::attribute::Attribute;
use crate::tree::class::{ClassName, ObjClassName};
use crate::tree::method::code::Code;
use crate::tree::type_annotation::{TargetInfoMethod, TypeAnnotation};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::class::ClassVisitor;
use crate::visitor::method::MethodVisitor;

#[derive(Debug, Clone, PartialEq)]
pub struct Method {
	pub access: MethodAccess,
	pub name: MethodName,
	pub descriptor: MethodDescriptor,

	pub has_deprecated_attribute: bool,
	pub has_synthetic_attribute: bool,

	pub code: Option<Code>,
	pub exceptions: Option<Vec<ClassName>>,
	pub signature: Option<MethodSignature>,

	pub runtime_visible_annotations: Vec<Annotation>,
	pub runtime_invisible_annotations: Vec<Annotation>,
	pub runtime_visible_type_annotations: Vec<TypeAnnotation<TargetInfoMethod>>,
	pub runtime_invisible_type_annotations: Vec<TypeAnnotation<TargetInfoMethod>>,
	// TODO: runtime_[in]visible_parameter_annotations

	pub annotation_default: Option<ElementValue>,
	pub method_parameters: Option<Vec<MethodParameter>>,

	pub attributes: Vec<Attribute>,
}

impl Method {
	pub fn new(access: MethodAccess, name: MethodName, descriptor: MethodDescriptor) -> Method {
		Method {
			access,
			name,
			descriptor,

			has_deprecated_attribute: false,
			has_synthetic_attribute: false,

			code: None,
			exceptions: None,
			signature: None,

			runtime_visible_annotations: Vec::new(),
			runtime_invisible_annotations: Vec::new(),
			runtime_visible_type_annotations: Vec::new(),
			runtime_invisible_type_annotations: Vec::new(),

			annotation_default: None,
			method_parameters: None,

			attributes: Vec::new(),
		}
	}

	pub fn accept<C: ClassVisitor>(self, visitor: C) -> Result<C> {
		match visitor.visit_method(self.access, self.name, self.descriptor)? {
			ControlFlow::Continue((visitor, mut method_visitor)) => {
				let interests = method_visitor.interests(); // TODO: make even more use of them

				method_visitor.visit_deprecated_and_synthetic_attribute(self.has_deprecated_attribute, self.has_synthetic_attribute)?;

				if interests.code {
					if let Some(code) = self.code {
						method_visitor = code.accept(method_visitor)?;
					}
				}
				if interests.exceptions {
					if let Some(exceptions) = self.exceptions {
						method_visitor.visit_exceptions(exceptions)?;
					}
				}
				if interests.signature {
					if let Some(signature) = self.signature {
						method_visitor.visit_signature(signature)?;
					}
				}

				if interests.runtime_visible_annotations && !self.runtime_visible_annotations.is_empty() {
					let (visitor, mut annotations_visitor) = method_visitor.visit_annotations(true)?;
					for annotation in self.runtime_visible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					method_visitor = MethodVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if interests.runtime_invisible_annotations && !self.runtime_invisible_annotations.is_empty() {
					let (visitor, mut annotations_visitor) = method_visitor.visit_annotations(false)?;
					for annotation in self.runtime_invisible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					method_visitor = MethodVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if interests.runtime_visible_type_annotations && !self.runtime_visible_type_annotations.is_empty() {
					let (visitor, mut type_annotations_visitor) = method_visitor.visit_type_annotations(true)?;
					for annotation in self.runtime_visible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					method_visitor = MethodVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}
				if interests.runtime_invisible_type_annotations && !self.runtime_invisible_type_annotations.is_empty() {
					let (visitor, mut type_annotations_visitor) = method_visitor.visit_type_annotations(false)?;
					for annotation in self.runtime_invisible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					method_visitor = MethodVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}
				// TODO: method parameter annotations

				if interests.annotation_default {
					if let Some(annotation_default) = self.annotation_default {
						let (visitor, x) = method_visitor.visit_annotation_default()?;
						let x = annotation_default.accept(x)?;
						method_visitor = MethodVisitor::finish_annotation_default(visitor, x)?;
					}
				}
				if interests.method_parameters {
					if let Some(method_parameters) = self.method_parameters {
						method_visitor.visit_parameters(method_parameters)?;
					}
				}

				if interests.unknown_attributes {
					for attribute in self.attributes {
						if let Some(attribute) = UnknownAttributeVisitor::from_attribute(attribute)? {
							method_visitor.visit_unknown_attribute(attribute)?;
						}
					}
				}

				ClassVisitor::finish_method(visitor, method_visitor)
			}
			ControlFlow::Break(visitor) => Ok(visitor)
		}
	}

	pub fn into_name_and_desc(self) -> MethodNameAndDesc {
		MethodNameAndDesc {
			name: self.name,
			desc: self.descriptor,
		}
	}

	/// Clones `self.name` and `self.descriptor` into a new [`MethodNameAndDesc`].
	pub fn as_name_and_desc(&self) -> MethodNameAndDesc {
		MethodNameAndDesc {
			name: self.name.clone(),
			desc: self.descriptor.clone(),
		}
	}
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct MethodAccess {
	pub is_public: bool,
	pub is_private: bool,
	pub is_protected: bool,
	pub is_static: bool,
	pub is_final: bool,
	pub is_synchronized: bool,
	pub is_bridge: bool,
	pub is_varargs: bool,
	pub is_native: bool,
	pub is_abstract: bool,
	pub is_strict: bool,
	pub is_synthetic: bool,
}

impl Debug for MethodAccess {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("MethodAccess { ")?;
		if self.is_public       { f.write_str("public ")?; }
		if self.is_private      { f.write_str("private ")?; }
		if self.is_protected    { f.write_str("protected ")?; }
		if self.is_static       { f.write_str("static ")?; }
		if self.is_final        { f.write_str("final ")?; }
		if self.is_synchronized { f.write_str("synchronized ")?; }
		if self.is_bridge       { f.write_str("bridge ")?; }
		if self.is_varargs      { f.write_str("varargs ")?; }
		if self.is_native       { f.write_str("native ")?; }
		if self.is_abstract     { f.write_str("abstract ")?; }
		if self.is_strict       { f.write_str("strict ")?; }
		if self.is_synthetic    { f.write_str("synthetic ")?; }
		f.write_str("}")
	}
}

impl From<u16> for MethodAccess {
	fn from(value: u16) -> Self {
		MethodAccess {
			is_public:       value & 0x0001 != 0,
			is_private:      value & 0x0002 != 0,
			is_protected:    value & 0x0004 != 0,
			is_static:       value & 0x0008 != 0,
			is_final:        value & 0x0010 != 0,
			is_synchronized: value & 0x0020 != 0,
			is_bridge:       value & 0x0040 != 0,
			is_varargs:      value & 0x0080 != 0,
			is_native:       value & 0x0100 != 0,
			is_abstract:     value & 0x0400 != 0,
			is_strict:       value & 0x0800 != 0,
			is_synthetic:    value & 0x1000 != 0,
		}
	}
}

impl From<MethodAccess> for u16 {
	fn from(value: MethodAccess) -> Self {
		(if value.is_public       { 0x0001 } else { 0 }) |
		(if value.is_private      { 0x0002 } else { 0 }) |
		(if value.is_protected    { 0x0004 } else { 0 }) |
		(if value.is_static       { 0x0008 } else { 0 }) |
		(if value.is_final        { 0x0010 } else { 0 }) |
		(if value.is_synchronized { 0x0020 } else { 0 }) |
		(if value.is_bridge       { 0x0040 } else { 0 }) |
		(if value.is_varargs      { 0x0080 } else { 0 }) |
		(if value.is_native       { 0x0100 } else { 0 }) |
		(if value.is_abstract     { 0x0400 } else { 0 }) |
		(if value.is_strict       { 0x0800 } else { 0 }) |
		(if value.is_synthetic    { 0x1000 } else { 0 })
	}
}

/// A method reference. Can also reference array classes.
///
/// See [MethodRefObj] if you only want non-array classes.
/// Note that as arrays have methods, such as `.clone()` that can be referenced.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MethodRef {
	/// Other than for fields, there can be references to methods on array classes.
	/// One example is the `.clone()` method provided by `Object` and implemented (as any array `implements Cloneable`)
	/// by any array.
	pub class: ClassName,
	pub name: MethodName,
	pub desc: MethodDescriptor,
}

/// A [MethodRef] but with the extra requirement that it can only reference object classes.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MethodRefObj {
	pub class: ObjClassName,
	pub name: MethodName,
	pub desc: MethodDescriptor,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MethodNameAndDesc {
	pub name: MethodName,
	pub desc: MethodDescriptor,
}

impl MethodNameAndDesc {
	/// Add a [`ClassName`] to this [`MethodNameAndDesc`] to make a [`MethodRef`].
	///
	/// This is a [`ClassName`], as there can be references to methods on array classes, such as `.clone()`.
	pub fn with_class(self, class: ClassName) -> MethodRef {
		MethodRef { class, name: self.name, desc: self.desc }
	}

	/// Add a [`ObjClassName`] to this [`MethodNameAndDesc`] to make a [`MethodRefObj`].
	pub fn with_class_obj(self, class: ObjClassName) -> MethodRefObj {
		MethodRefObj { class, name: self.name, desc: self.desc }
	}
}

make_string_str_like!(
	pub MethodName(JavaString);
	pub MethodNameSlice(JavaStr);
);
make_display!(MethodName, MethodNameSlice);

impl MethodName {
	fn check_valid(inner: &JavaStr) -> Result<()> {
		if crate::tree::names::is_valid_method_name(inner) {
			Ok(())
		} else {
			bail!("invalid method name: must be either `<init>`, `<clinit>`, or non-empty and not contain any of `.`, `;`, `[`, `/`, `<`, and `>`");
		}
	}

	pub const INIT: &'static MethodNameSlice = {
		// SAFETY: `<init>` is a valid method name.
		unsafe { MethodNameSlice::from_inner_unchecked(JavaStr::from_str("<init>")) }
	};
	pub const CLINIT: &'static MethodNameSlice = {
		// SAFETY: `<clinit>` is a valid method name.
		unsafe { MethodNameSlice::from_inner_unchecked(JavaStr::from_str("<clinit>")) }
	};
}

make_string_str_like!(
	pub MethodDescriptor(JavaString);
	pub MethodDescriptorSlice(JavaStr);
);
make_display!(MethodDescriptor, MethodDescriptorSlice);

impl MethodDescriptor {
	fn check_valid(inner: &JavaStr) -> Result<()> {
		Ok(()) // TODO: parse the desc and fail if invalid
	}
}

make_string_str_like!(
	pub MethodSignature(JavaString);
	pub MethodSignatureSlice(JavaStr);
);

impl MethodSignature {
	fn check_valid(inner: &JavaStr) -> Result<()> {
		Ok(()) // TODO: signature format is even more complicated
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodParameter {
	pub name: Option<ParameterName>,
	pub flags: ParameterFlags,
}

make_string_str_like!(
	pub ParameterName(JavaString);
	pub ParameterNameSlice(JavaStr);
);
make_display!(ParameterName, ParameterNameSlice);

impl ParameterName {
	fn check_valid(inner: &JavaStr) -> Result<()> {
		if crate::tree::names::is_valid_unqualified_name(inner) {
			Ok(())
		} else {
			bail!("invalid parameter name: must be non-empty and not contain any of `.`, `;`, `[` and `/`");
		}
	}
}

#[derive(Copy, Clone, PartialEq)]
pub struct ParameterFlags {
	pub is_final: bool,
	pub is_synthetic: bool,
	pub is_mandated: bool,
}

impl Debug for ParameterFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("ParameterFlags { ")?;
		if self.is_final     { f.write_str("final ")?; }
		if self.is_synthetic { f.write_str("synthetic ")?; }
		if self.is_mandated  { f.write_str("mandated ")?; }
		f.write_str("}")
	}
}

impl From<u16> for ParameterFlags {
	fn from(value: u16) -> Self {
		ParameterFlags {
			is_final:     value & 0x0010 != 0,
			is_synthetic: value & 0x1000 != 0,
			is_mandated:  value & 0x8000 != 0,
		}
	}
}

impl From<ParameterFlags> for u16 {
	fn from(value: ParameterFlags) -> Self {
		(if value.is_final     { 0x0010 } else { 0 }) |
		(if value.is_synthetic { 0x1000 } else { 0 }) |
		(if value.is_mandated  { 0x8000 } else { 0 })
	}
}
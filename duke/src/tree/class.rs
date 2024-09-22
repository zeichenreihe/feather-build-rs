use std::fmt::{Debug, Display, Formatter};
use anyhow::{bail, Result};
use std::ops::ControlFlow;
use java_string::{JavaStr, JavaString};
use crate::macros::make_string_str_like;
use crate::tree::annotation::Annotation;
use crate::tree::attribute::Attribute;
use crate::tree::field::Field;
use crate::tree::method::{Method, MethodNameAndDesc};
use crate::tree::module::{Module, PackageName};
use crate::tree::record::RecordComponent;
use crate::tree::type_annotation::{TargetInfoClass, TypeAnnotation};
use crate::tree::version::Version;
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::class::ClassVisitor;
use crate::visitor::MultiClassVisitor;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassFile {
	pub version: Version,
	pub access: ClassAccess,
	pub name: ClassName,
	pub super_class: Option<ClassName>,
	pub interfaces: Vec<ClassName>,

	pub fields: Vec<Field>,
	pub methods: Vec<Method>,

	pub has_deprecated_attribute: bool,
	pub has_synthetic_attribute: bool,

	pub inner_classes: Option<Vec<InnerClass>>,
	pub enclosing_method: Option<EnclosingMethod>,
	pub signature: Option<ClassSignature>,

	pub source_file: Option<JavaString>,
	pub source_debug_extension: Option<JavaString>,

	pub runtime_visible_annotations: Vec<Annotation>,
	pub runtime_invisible_annotations: Vec<Annotation>,
	pub runtime_visible_type_annotations: Vec<TypeAnnotation<TargetInfoClass>>,
	pub runtime_invisible_type_annotations: Vec<TypeAnnotation<TargetInfoClass>>,

	pub module: Option<Module>,
	pub module_packages: Option<Vec<PackageName>>,
	pub module_main_class: Option<ClassName>,

	pub nest_host_class: Option<ClassName>,
	pub nest_members: Option<Vec<ClassName>>,
	pub permitted_subclasses: Option<Vec<ClassName>>,

	pub record_components: Vec<RecordComponent>,

	pub attributes: Vec<Attribute>,
}

impl ClassFile {
	pub fn new(version: Version, access: ClassAccess, name: ClassName, super_class: Option<ClassName>, interfaces: Vec<ClassName>) -> ClassFile {
		ClassFile {
			version,
			access,
			name,
			super_class,
			interfaces,

			fields: Vec::new(),
			methods: Vec::new(),

			has_deprecated_attribute: false,
			has_synthetic_attribute: false,

			inner_classes: None,
			enclosing_method: None,
			signature: None,

			source_file: None,
			source_debug_extension: None,

			runtime_visible_annotations: Vec::new(),
			runtime_invisible_annotations: Vec::new(),
			runtime_visible_type_annotations: Vec::new(),
			runtime_invisible_type_annotations: Vec::new(),

			module: None,
			module_packages: None,
			module_main_class: None,

			nest_host_class: None,
			nest_members: None,
			permitted_subclasses: None,

			record_components: Vec::new(),

			attributes: Vec::new(),
		}
	}

	pub fn accept<V: MultiClassVisitor>(self, visitor: V) -> Result<V> {
		match visitor.visit_class(self.version, self.access, self.name, self.super_class, self.interfaces)? {
			ControlFlow::Continue((visitor, mut class_visitor)) => {
				let interests = class_visitor.interests();

				class_visitor.visit_deprecated_and_synthetic_attribute(self.has_deprecated_attribute, self.has_synthetic_attribute)?;

				if interests.inner_classes {
					if let Some(inner_classes) = self.inner_classes {
						class_visitor.visit_inner_classes(inner_classes)?;
					}
				}
				if interests.enclosing_method {
					if let Some(enclosing_method) = self.enclosing_method {
						class_visitor.visit_enclosing_method(enclosing_method)?;
					}
				}
				if interests.signature {
					if let Some(signature) = self.signature {
						class_visitor.visit_signature(signature)?;
					}
				}

				if interests.source_file {
					if let Some(source_file) = self.source_file {
						class_visitor.visit_source_file(source_file)?;
					}
				}
				if interests.source_debug_extension {
					if let Some(source_debug_extension) = self.source_debug_extension {
						class_visitor.visit_source_debug_extension(source_debug_extension)?;
					}
				}

				if interests.runtime_visible_annotations && !self.runtime_visible_annotations.is_empty() {
					let (visitor, mut annotations_visitor) = class_visitor.visit_annotations(true)?;
					for annotation in self.runtime_visible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					class_visitor = ClassVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if interests.runtime_invisible_annotations && !self.runtime_invisible_annotations.is_empty() {
					let (visitor, mut annotations_visitor) = class_visitor.visit_annotations(false)?;
					for annotation in self.runtime_invisible_annotations {
						annotations_visitor = annotation.accept(annotations_visitor)?;
					}
					class_visitor = ClassVisitor::finish_annotations(visitor, annotations_visitor)?;
				}
				if interests.runtime_visible_type_annotations && !self.runtime_visible_type_annotations.is_empty() {
					let (visitor, mut type_annotations_visitor) = class_visitor.visit_type_annotations(true)?;
					for annotation in self.runtime_visible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					class_visitor = ClassVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}
				if interests.runtime_invisible_type_annotations && !self.runtime_invisible_type_annotations.is_empty() {
					let (visitor, mut type_annotations_visitor) = class_visitor.visit_type_annotations(false)?;
					for annotation in self.runtime_invisible_type_annotations {
						type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
					}
					class_visitor = ClassVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
				}

				if interests.module {
					if let Some(module) = self.module {
						class_visitor.visit_module(module)?;
					}
				}
				if interests.module_packages {
					if let Some(module_packages) = self.module_packages {
						class_visitor.visit_module_packages(module_packages)?;
					}
				}
				if interests.module_main_class {
					if let Some(module_main_class) = self.module_main_class {
						class_visitor.visit_module_main_class(module_main_class)?;
					}
				}

				if interests.nest_host {
					if let Some(nest_host_class) = self.nest_host_class {
						class_visitor.visit_nest_host_class(nest_host_class)?;
					}
				}
				if interests.nest_members {
					if let Some(nest_members) = self.nest_members {
						class_visitor.visit_nest_members(nest_members)?;
					}
				}

				if interests.permitted_subclasses {
					if let Some(permitted_subclasses) = self.permitted_subclasses {
						class_visitor.visit_permitted_subclasses(permitted_subclasses)?;
					}
				}
				if interests.record {
					for record_component in self.record_components {
						class_visitor = record_component.accept(class_visitor)?;
					}
				}

				if interests.unknown_attributes {
					for attribute in self.attributes {
						if let Some(attribute) = UnknownAttributeVisitor::from_attribute(attribute)? {
							class_visitor.visit_unknown_attribute(attribute)?;
						}
					}
				}

				if interests.fields {
					for field in self.fields {
						class_visitor = field.accept(class_visitor)?;
					}
				}
				if interests.methods {
					for method in self.methods {
						class_visitor = method.accept(class_visitor)?;
					}
				}

				MultiClassVisitor::finish_class(visitor, class_visitor)
			}
			ControlFlow::Break(visitor) => Ok(visitor),
		}
	}
}

/// Represents the access flags a class can have.
///
/// Take a look at the [Java Virtual Machine Specification](https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html#jvms-4.1-200-E.1), for
/// the meanings of these fields, and what combinations are legal and which not.
// TODO: add Default as for all false for other *Access as well (+ document it)
#[derive(Copy, Clone, Default, PartialEq)]
pub struct ClassAccess {
	pub is_public: bool,
	pub is_final: bool,
	pub is_super: bool,
	pub is_interface: bool,
	pub is_abstract: bool,
	pub is_synthetic: bool,
	pub is_annotation: bool,
	pub is_enum: bool,
	pub is_module: bool,
}

impl Debug for ClassAccess {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("ClassAccess { ")?;
		if self.is_public     { f.write_str("public ")?; }
		if self.is_final      { f.write_str("final ")?; }
		if self.is_super      { f.write_str("super ")?; }
		if self.is_interface  { f.write_str("interface ")?; }
		if self.is_abstract   { f.write_str("abstract ")?; }
		if self.is_synthetic  { f.write_str("synthetic ")?; }
		if self.is_annotation { f.write_str("annotation ")?; }
		if self.is_enum       { f.write_str("enum ")?; }
		if self.is_module     { f.write_str("module ")?; }
		f.write_str("}")
	}
}

/// Interprets an `u16` as the `access_flags` item of the `ClassFile` structure of the Java Virtual Machine Specification.
impl From<u16> for ClassAccess {
	fn from(value: u16) -> Self {
		ClassAccess {
			is_public:     value & 0x0001 != 0,
			is_final:      value & 0x0010 != 0,
			is_super:      value & 0x0020 != 0,
			is_interface:  value & 0x0200 != 0,
			is_abstract:   value & 0x0400 != 0,
			is_synthetic:  value & 0x1000 != 0,
			is_annotation: value & 0x2000 != 0,
			is_enum:       value & 0x4000 != 0,
			is_module:     value & 0x8000 != 0,
		}
	}
}

/// Creates an `u16` according to the `access_flags` item of the `ClassFile` structure of the Java Virtual Machine Specification.
impl From<ClassAccess> for u16 {
	fn from(value: ClassAccess) -> Self {
		(if value.is_public     { 0x0001 } else { 0 }) |
		(if value.is_final      { 0x0010 } else { 0 }) |
		(if value.is_super      { 0x0020 } else { 0 }) |
		(if value.is_interface  { 0x0200 } else { 0 }) |
		(if value.is_abstract   { 0x0400 } else { 0 }) |
		(if value.is_synthetic  { 0x1000 } else { 0 }) |
		(if value.is_annotation { 0x2000 } else { 0 }) |
		(if value.is_enum       { 0x4000 } else { 0 }) |
		(if value.is_module     { 0x8000 } else { 0 })
	}
}

// TODO: make it an error to construct a class name containing "illegal" (see jvm spec) chars, like `;`
// TODO: same goes for the other Name/Desc types
//  (this means removal of the From impls and instead making TryFrom methods...)
make_string_str_like!(
	/// Represents a class name. The class name uses [internal binary names](https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html#jvms-4.2.1),
	/// i.e. with complete path written out and using slashes.
	///
	/// # Examples
	/// The java class `java.lang.Thread` would get:
	/// ```
	/// use duke::tree::class::ClassName;
	/// let java_lang_thread = ClassName::from("java/lang/Thread");
	/// ```
	/// Note that there's an associated constant holding the name of the `java.lang.Object` class:
	/// ```
	/// use duke::tree::class::ClassName;
	/// let java_lang_object = ClassName::JAVA_LANG_OBJECT.clone();
	/// assert_eq!(java_lang_object, ClassName::from("java/lang/Object"));
	/// ```
	pub ClassName(JavaString);
	/// A [`ClassName`] slice.
	pub ClassNameSlice(JavaStr);
	is_valid(s) = if crate::tree::names::is_valid_class_name(s) {
		Ok(())
	} else {
		bail!("invalid class name: must be either array field descriptor; or must consist out of `/` separated non-empty parts, and not contain any of `.`, `;`, `[`")
	};
);

impl ClassName {
	/// A constant holding the class name of `Object`.
	pub const JAVA_LANG_OBJECT: &'static ClassNameSlice = unsafe { ClassNameSlice::from_inner_unchecked(JavaStr::from_str("java/lang/Object")) };

	// TODO: doc, check with JVM spec
	pub fn get_simple_name(&self) -> &JavaStr {
		let s = self.as_inner();
		s.rsplit_once('/')
			.map_or(s, |(_, simple)| simple)
	}
}

impl Display for ClassName {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		Display::fmt(self.as_slice(), f)
	}
}
impl Display for ClassNameSlice {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_inner())
	}
}

make_string_str_like!(
	/// Represents a class signature, from a generic such as `Foo<T extends Bar>`.
	pub ClassSignature(JavaString);
	pub ClassSignatureSlice(JavaStr);
	is_valid(__) = Ok(()); // TODO: signature format is even more complicated
);

#[derive(Debug, Clone, PartialEq)]
pub struct InnerClass {
	pub inner_class: ClassName,
	pub outer_class: Option<ClassName>,
	pub inner_name: Option<JavaString>,
	pub flags: InnerClassFlags,
}

#[derive(Copy, Clone, PartialEq)]
pub struct InnerClassFlags {
	pub is_public: bool,
	pub is_private: bool,
	pub is_protected: bool,
	pub is_static: bool,
	pub is_final: bool,
	pub is_interface: bool,
	pub is_abstract: bool,
	pub is_synthetic: bool,
	pub is_annotation: bool,
	pub is_enum: bool,
}

impl Debug for InnerClassFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("InnerClassFlags { ")?;
		if self.is_public     { f.write_str("public ")?; }
		if self.is_private    { f.write_str("private ")?; }
		if self.is_protected  { f.write_str("protected ")?; }
		if self.is_static     { f.write_str("static ")?; }
		if self.is_final      { f.write_str("final ")?; }
		if self.is_interface  { f.write_str("interface ")?; }
		if self.is_abstract   { f.write_str("abstract ")?; }
		if self.is_synthetic  { f.write_str("synthetic ")?; }
		if self.is_annotation { f.write_str("annotation ")?; }
		if self.is_enum       { f.write_str("enum ")?; }
		f.write_str("}")
	}
}

impl From<u16> for InnerClassFlags {
	fn from(value: u16) -> Self {
		InnerClassFlags {
			is_public:     value & 0x0001 != 0,
			is_private:    value & 0x0002 != 0,
			is_protected:  value & 0x0004 != 0,
			is_static:     value & 0x0008 != 0,
			is_final:      value & 0x0010 != 0,
			is_interface:  value & 0x0200 != 0,
			is_abstract:   value & 0x0400 != 0,
			is_synthetic:  value & 0x1000 != 0,
			is_annotation: value & 0x2000 != 0,
			is_enum:       value & 0x4000 != 0,
		}
	}
}

impl From<InnerClassFlags> for u16 {
	fn from(value: InnerClassFlags) -> Self {
		(if value.is_public     { 0x0001 } else { 0 }) |
		(if value.is_private    { 0x0002 } else { 0 }) |
		(if value.is_protected  { 0x0004 } else { 0 }) |
		(if value.is_static     { 0x0008 } else { 0 }) |
		(if value.is_final      { 0x0010 } else { 0 }) |
		(if value.is_interface  { 0x0200 } else { 0 }) |
		(if value.is_abstract   { 0x0400 } else { 0 }) |
		(if value.is_synthetic  { 0x1000 } else { 0 }) |
		(if value.is_annotation { 0x2000 } else { 0 }) |
		(if value.is_enum       { 0x4000 } else { 0 })
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnclosingMethod {
	pub class: ClassName,
	pub method: Option<MethodNameAndDesc>,
}
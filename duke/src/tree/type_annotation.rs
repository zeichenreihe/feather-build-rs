use anyhow::Result;
use crate::tree::annotation::Annotation;
use crate::tree::method::code::{Label, LabelRange, LvIndex};
use crate::visitor::annotation::TypeAnnotationsVisitor;

// TODO: might also be a good idea to provide java code snippets...

/// States exactly on which type the annotation is.
///
/// For the class file structure.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TargetInfoClass {
	/// The annotation is on a type parameter of a generic class or generic interface.
	ClassTypeParameter {
		/// Specifies the index of the type parameter. `0` means the first type parameter.
		index: u8,
	},
	/// The annotation is on the superclass in an `extends` clause.
	Extends,
	/// The annotation is on an super interface, specified by either the `implements` clause (for
	/// classes) or the `extends` clause (for interfaces).
	Implements {
		/// Specifies the index into the list of the class' interfaces. `0` means first entry of that list.
		///
		/// An index of [`u16::MAX`] is not legal.
		index: u16
	},
	/// The annotation is on a bound of a type parameter of a generic class or generic interface.
	ClassTypeParameterBound {
		/// Specifies the index of the type parameter. `0` means the first type parameter.
		type_parameter_index: u8,
		/// Specifies the index of the bound. `0` means the first bound.
		bound_index: u8,
	},
}

/// States exactly on which type the annotation is.
///
/// For the field info structure.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TargetInfoField {
	/// The annotation is on the type of a field declaration or on the type of a record component declaration.
	Field,
}

/// States exactly on which type the annotation is.
///
/// For the method info structure.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TargetInfoMethod {
	/// The annotation is on a type parameter of a generic method or generic constructor.
	MethodTypeParameter {
	/// Specifies the index of the type parameter. `0` means the first type parameter.
	index: u8,
	},
	/// The annotation is on a bound of a type parameter of a generic method or generic constructor.
	MethodTypeParameterBound {
		/// Specifies the index of the type parameter. `0` means the first type parameter.
		type_parameter_index: u8,
		/// Specifies the index of the bound. `0` means the first bound.
		bound_index: u8,
	},
	/// The annotation is on the return type of a method or on the type of a newly constructed object.
	// TODO: is that "newly constructed" really just return type of ctor?
	Return,
	// TODO: proper doc here / figure out where it happens
	/// "receiver type of method or constructor" or "the receiver type of a method or constructor"
	Receiver,
	/// The annotation is on a type of a formal parameter declaration of a method, constructor or lambda expression.
	FormalParameter {
		/// Specifies the formal parameter index.
		/// Note that an index `i` may, but is not required to, correspond to the `i`th parameter descriptor in the method descriptor.
		index: u8,
	},
	/// The annotation is on the `throws` clause of a method or constructor.
	Throws {
		/// Specifies the index into the list of the methods exceptions. `0` means first entry of that list.
		index: u16,
	},
}

/// States exactly on which type the annotation is.
///
/// For inside the `Code` attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetInfoCode {
	/// Indicates that the annotation is on the type of a local variable declaration.
	///
	/// In each [`LabelRange`] the local variable can be found in the [`LvIndex`].
	///
	/// A table of these values is required, because a local variable may have different
	/// indices ([`LvIndex`]) over multiple ranges.
	LocalVariable {
		table: Vec<(LabelRange, LvIndex)>,
	},
	/// Indicates that the annotation is on the type of a resource variable declaration.
	///
	/// This is a variable declared as a resource inside a try-with-resources statement.
	///
	/// In each [`LabelRange`] the resource variable can be found in the [`LvIndex`].
	///
	/// A table of these values is required, because a resource variable may have different
	/// indices ([`LvIndex`]) over multiple ranges.
	ResourceVariable {
		table: Vec<(LabelRange, LvIndex)>,
	},
	/// Indicates that the annotation appears on the type of an `catch` statement.
	ExceptionParameter {
		// TODO: update docs to have examples and links to fields where the index goes into
		/// Specifies the index into the `exception_table` of the `Code` attribute. `0` means first entry of that list.
		index: u16,
	},
	/// Indicates that the annotation appears on the type of an `instanceof` expression.
	InstanceOf(Label),
	/// Indicates that the annotation appears on the type of a `new` expression.
	New(Label),
	/// Indicates that the annotation appears on the type before the `::` of a method reference expression.
	// TODO: figure out where exactly, most likely this is for a reference to a ctor? so ::new?
	ConstructorReference(Label),
	/// Indicates that the annotation appears on the type before the `::` of a method reference expression.
	// TODO: figure out where exactly, this is most likely for a reference to a normal method (so not <init>)
	MethodReference(Label),
	/// Indicates that the annotation appears on the `index`th type in a cast expression.
	///
	/// In **most** cases, there's only one type in the cast expression, so the `index` is `0`.
	/// There can be more than one type in a cast expression: if it casts to an "intersection type".
	// TODO: give good example
	Cast {
		label: Label,
		index: u8,
	},
	/// Indicates that the annotation is on an explicit type argument of a constructor call.
	///
	/// More specifically, it's on the `index`th explicit type argument.
	ConstructorInvocationTypeArgument {
		label: Label,
		index: u8,
	},
	/// Indicates that the annotation is on an explicit type argument of a method call.
	///
	/// More specifically, it's on the `index`th explicit type argument.
	MethodInvocationTypeArgument {
		label: Label,
		index: u8,
	},
	/// Indicates that the annotation is on an explicit type argument of a constructor reference.
	///
	/// More specifically, it's on the `index`th explicit type argument.
	ConstructorReferenceTypeArgument {
		label: Label,
		index: u8,
	},
	/// Indicates that the annotation is on an explicit type argument of a method reference.
	///
	/// More specifically, it's on the `index`th explicit type argument.
	MethodReferenceTypeArgument {
		label: Label,
		index: u8,
	},
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TypePathKind {
	ArrayDeeper,
	NestedDeeper,
	WildcardBound,
	TypeArgument {
		index: u8,
	}
}

/// Specifies exactly where in the type the annotation is.
#[derive(Debug, Clone, PartialEq)]
pub struct TypePath {
	pub(crate) path: Vec<TypePathKind>
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation<T> {
	pub type_reference: T,
	pub type_path: TypePath,
	pub annotation: Annotation,
}

impl<T> TypeAnnotation<T> {
	pub fn new(type_reference: T, type_path: TypePath, annotation: Annotation) -> TypeAnnotation<T> {
		TypeAnnotation { type_reference, type_path, annotation }
	}

	pub fn accept<A: TypeAnnotationsVisitor<T>>(self, visitor: A) -> Result<A> {
		let (visitor, mut pairs_visitor) = visitor.visit_type_annotation(self.type_reference, self.type_path, self.annotation.annotation_type)?;

		pairs_visitor = super::annotation::accept_element_values_named(pairs_visitor, self.annotation.element_value_pairs)?;

		TypeAnnotationsVisitor::finish_type_annotation(visitor, pairs_visitor)
	}
}
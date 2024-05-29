use std::ops::ControlFlow;
use anyhow::{bail, Context, Result};
use crate::OptionExpansion;
use crate::class_reader::pool::PoolRead;
use crate::tree::annotation::{Annotation, ElementValue, ElementValuePair, Object};
use crate::tree::attribute::Attribute;
use crate::tree::class::{ClassAccess, ClassFile, ClassName, ClassSignature, EnclosingMethod, InnerClass};
use crate::tree::field::{ConstantValue, Field, FieldAccess, FieldDescriptor, FieldName, FieldSignature};
use crate::tree::method::{Method, MethodAccess, MethodDescriptor, MethodName, MethodParameter, MethodSignature};
use crate::tree::method::code::{Code, Exception, Instruction, InstructionListEntry, Label, Lv};
use crate::tree::module::{Module, PackageName};
use crate::tree::record::{RecordComponent, RecordName};
use crate::tree::type_annotation::{TargetInfoClass, TargetInfoCode, TargetInfoField, TargetInfoMethod, TypeAnnotation, TypePath};
use crate::tree::version::Version;
use crate::visitor::annotation::{UnnamedElementValueVisitor, NamedElementValueVisitor, NamedElementValuesVisitor, UnnamedElementValuesVisitor, AnnotationsVisitor, TypeAnnotationsVisitor};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::class::{ClassInterests, ClassVisitor};
use crate::visitor::field::{FieldInterests, FieldVisitor};
use crate::visitor::method::code::{CodeInterests, CodeVisitor, StackMapData};
use crate::visitor::method::{MethodInterests, MethodVisitor};
use crate::visitor::MultiClassVisitor;
use crate::visitor::record::{RecordComponentInterests, RecordComponentVisitor};

impl MultiClassVisitor for Option<ClassFile> {
	type ClassVisitor = ClassFile;
	type ClassResidual = ();

	fn visit_class(self, version: Version, access: ClassAccess, name: ClassName, super_class: Option<ClassName>, interfaces: Vec<ClassName>)
			-> Result<ControlFlow<Self, (Self::ClassResidual, Self::ClassVisitor)>> {
		if let Some(old) = self {
			bail!("only one class visit allowed, but was called a second time: we had: {old:#?}, now got called with: {version:?} {access:?} {name:?} {super_class:?} {interfaces:?}")
		}
		Ok(ControlFlow::Continue(((), ClassFile::new(version, access, name, super_class, interfaces))))
	}

	fn finish_class(_this: Self::ClassResidual, class_visitor: Self::ClassVisitor) -> Result<Self> {
		Ok(Some(class_visitor))
	}
}

impl MultiClassVisitor for Vec<ClassFile> {
	type ClassVisitor = ClassFile;
	type ClassResidual = Self;

	fn visit_class(self, version: Version, access: ClassAccess, name: ClassName, super_class: Option<ClassName>, interfaces: Vec<ClassName>)
			-> Result<ControlFlow<Self, (Self::ClassResidual, Self::ClassVisitor)>> {
		Ok(ControlFlow::Continue((self, ClassFile::new(version, access, name, super_class, interfaces))))
	}

	fn finish_class(mut this: Self::ClassResidual, class_visitor: Self::ClassVisitor) -> Result<Self> {
		this.push(class_visitor);
		Ok(this)
	}
}

impl ClassVisitor for ClassFile {
	type AnnotationsVisitor = Vec<Annotation>;
	type AnnotationsResidual = (Self, bool);
	type TypeAnnotationsVisitor = Vec<TypeAnnotation<TargetInfoClass>>;
	type TypeAnnotationsResidual = (Self, bool);
	type RecordComponentVisitor = RecordComponent;
	type RecordComponentResidual = Self;
	type FieldVisitor = Field;
	type FieldResidual = Self;
	type MethodVisitor = Method;
	type MethodResidual = Self;
	type UnknownAttribute = Attribute;

	fn interests(&self) -> ClassInterests {
		ClassInterests::all()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, deprecated: bool, synthetic: bool) -> Result<()> {
		self.has_deprecated_attribute = deprecated;
		self.has_synthetic_attribute = synthetic;
		Ok(())
	}

	fn visit_inner_classes(&mut self, inner_classes: Vec<InnerClass>) -> Result<()> {
		//TODO: not entirely sure if there may only be one...
		// -> again the question if an Option<Vec<_>> is best, or if an Vec<_> is enough
		self.inner_classes.insert_if_empty(inner_classes).context("only one InnerClasses attribute is allowed")?;
		Ok(())
	}

	fn visit_enclosing_method(&mut self, enclosing_method: EnclosingMethod) -> Result<()> {
		self.enclosing_method.insert_if_empty(enclosing_method).context("only one EnclosingMethod attribute is allowed")
	}

	fn visit_signature(&mut self, signature: ClassSignature) -> Result<()> {
		self.signature.insert_if_empty(signature).context("only one Signature attribute is allowed")
	}

	fn visit_source_file(&mut self, source_file: String) -> Result<()> {
		self.source_file.insert_if_empty(source_file).context("only one SourceFile attribute is allowed")
	}

	fn visit_source_debug_extension(&mut self, source_debug_extension: String) -> Result<()> {
		self.source_debug_extension.insert_if_empty(source_debug_extension).context("only one SourceDebugExtension attribute is allowed")
	}

	fn visit_annotations(self, visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_annotations((mut this, visible): Self::AnnotationsResidual, annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleAnnotations attribute is allowed")
			this.runtime_visible_annotations.extend(annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleAnnotations attribute is allowed")
			this.runtime_invisible_annotations.extend(annotations_visitor);
		}
		Ok(this)
	}

	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_type_annotations((mut this, visible): Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleTypeAnnotations attribute is allowed")
			this.runtime_visible_type_annotations.extend(type_annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleTypeAnnotations attribute is allowed")
			this.runtime_invisible_type_annotations.extend(type_annotations_visitor);
		}
		Ok(this)
	}

	fn visit_module(&mut self, module: Module) -> Result<()> {
		self.module.insert_if_empty(module).context("only one Module attribute is allowed")
	}

	fn visit_module_packages(&mut self, module_packages: Vec<PackageName>) -> Result<()> {
		self.module_packages.insert_if_empty(module_packages).context("only one ModulePackages attribute is allowed")
	}

	fn visit_module_main_class(&mut self, module_main_class: ClassName) -> Result<()> {
		self.module_main_class.insert_if_empty(module_main_class).context("only one ModuleMainClass attribute is allowed")
	}

	fn visit_nest_host_class(&mut self, nest_host_class: ClassName) -> Result<()> {
		self.nest_host_class.insert_if_empty(nest_host_class).context("only one NestHost attribute is allowed")
	}

	fn visit_nest_members(&mut self, nest_members: Vec<ClassName>) -> Result<()> {
		self.nest_members.insert_if_empty(nest_members).context("only one NestMembers attribute is allowed")
	}

	fn visit_permitted_subclasses(&mut self, permitted_subclasses: Vec<ClassName>) -> Result<()> {
		self.permitted_subclasses.insert_if_empty(permitted_subclasses).context("only one PermittedSubclasses attribute is allowed")
	}

	fn visit_record_component(self, name: RecordName, descriptor: FieldDescriptor)
			-> Result<ControlFlow<Self, (Self::RecordComponentResidual, Self::RecordComponentVisitor)>> {
		Ok(ControlFlow::Continue((self, RecordComponent::new(name, descriptor))))
	}

	fn finish_record_component(mut this: Self::RecordComponentResidual, record_component_visitor: Self::RecordComponentVisitor) -> Result<Self> {
		this.record_components.push(record_component_visitor);
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		self.attributes.push(unknown_attribute);
		Ok(())
	}

	fn visit_field(self, access: FieldAccess, name: FieldName, descriptor: FieldDescriptor)
			-> Result<ControlFlow<Self, (Self::FieldResidual, Self::FieldVisitor)>> {
		Ok(ControlFlow::Continue((self, Field::new(access, name, descriptor))))
	}

	fn finish_field(mut this: Self::FieldResidual, field_visitor: Self::FieldVisitor) -> Result<Self> {
		this.fields.push(field_visitor);
		Ok(this)
	}

	fn visit_method(self, access: MethodAccess, name: MethodName, descriptor: MethodDescriptor)
			-> Result<ControlFlow<Self, (Self::MethodResidual, Self::MethodVisitor)>> {
		Ok(ControlFlow::Continue((self, Method::new(access, name, descriptor))))
	}

	fn finish_method(mut this: Self::MethodResidual, method_visitor: Self::MethodVisitor) -> Result<Self> {
		this.methods.push(method_visitor);
		Ok(this)
	}
}

impl AnnotationsVisitor for Vec<Annotation> {
	type NamedElementValuesVisitor = Annotation;
	type NamedElementValuesResidual = Self;

	fn visit_annotation(self, annotation_descriptor: FieldDescriptor) -> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)> {
		Ok((self, Annotation::new(annotation_descriptor)))
	}

	fn finish_annotation(mut this: Self::NamedElementValuesResidual, named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self> {
		this.push(named_element_values_visitor);
		Ok(this)
	}
}

impl<T> TypeAnnotationsVisitor<T> for Vec<TypeAnnotation<T>> {
	type NamedElementValuesVisitor = Annotation;
	type NamedElementValuesResidual = (Self, T, TypePath);

	fn visit_type_annotation(self, type_reference: T, type_path: TypePath, annotation_descriptor: FieldDescriptor) -> Result<(Self::NamedElementValuesResidual, Self::NamedElementValuesVisitor)> {
		Ok(((self, type_reference, type_path), Annotation::new(annotation_descriptor)))
	}

	fn finish_type_annotation((mut this, type_reference, type_path): Self::NamedElementValuesResidual, named_element_values_visitor: Self::NamedElementValuesVisitor) -> Result<Self> {
		this.push(TypeAnnotation::new(type_reference, type_path, named_element_values_visitor));
		Ok(this)
	}
}

/*
TODO: (but not yet sure) We could change the [Un]NamedElementValue**s**Visitor (most of the time essentially a Vec<...>)
 to have a method like .visit() -> A (no Option<A> here!), and then A must impl the trait without the **s** and that trait maps A to B (each method)
 and then besides the .visit() method there's a .finish(B) -> () method that just stores it (optionally). This works bc essentially
 every **s** visitor currently also impls the no-**s** visitor, and that one currently behaves like a Vec<_> that just .push()es elements...
 */

impl NamedElementValuesVisitor for Annotation {}

impl NamedElementValueVisitor for Annotation {
	type AnnotationVisitor = Annotation;
	type AnnotationResidual = (Self, String);
	type AnnotationArrayVisitor = Vec<ElementValue>;
	type AnnotationArrayResidual = (Self, String);

	fn visit(&mut self, name: String, value: Object) -> Result<()> {
		self.element_value_pairs.push(ElementValuePair {
			name,
			value: ElementValue::Object(value),
		});
		Ok(())
	}

	fn visit_enum(&mut self, name: String, type_name: FieldDescriptor, const_name: String) -> Result<()> {
		self.element_value_pairs.push(ElementValuePair {
			name,
			value: ElementValue::Enum {
				type_name,
				const_name,
			}
		});
		Ok(())
	}

	fn visit_class(&mut self, name: String, class: String) -> Result<()> {
		self.element_value_pairs.push(ElementValuePair {
			name,
			value: ElementValue::Class(class),
		});
		Ok(())
	}

	fn visit_annotation(self, name: String, annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)> {
		Ok(((self, name), Annotation::new(annotation_type)))
	}

	fn finish_annotation((mut this, name): Self::AnnotationResidual, annotation_visitor: Self::AnnotationVisitor) -> Result<Self> {
		this.element_value_pairs.push(ElementValuePair {
			name,
			value: ElementValue::AnnotationInterface(annotation_visitor),
		});
		Ok(this)
	}

	fn visit_array(self, name: String) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)> {
		Ok(((self, name), Vec::new()))
	}

	fn finish_array((mut this, name): Self::AnnotationArrayResidual, annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self> {
		this.element_value_pairs.push(ElementValuePair {
			name,
			value: ElementValue::ArrayType(annotation_array_visitor),
		});
		Ok(this)
	}
}

impl UnnamedElementValuesVisitor for Vec<ElementValue> {}

impl UnnamedElementValueVisitor for Vec<ElementValue> {
	type AnnotationVisitor = Annotation;
	type AnnotationResidual = Self;
	type AnnotationArrayVisitor = Vec<ElementValue>;
	type AnnotationArrayResidual = Self;

	fn visit(&mut self, value: Object) -> Result<()> {
		self.push(ElementValue::Object(value));
		Ok(())
	}

	fn visit_enum(&mut self, type_name: FieldDescriptor, const_name: String) -> Result<()> {
		self.push(ElementValue::Enum {
			type_name,
			const_name,
		});
		Ok(())
	}

	fn visit_class(&mut self, class: String) -> Result<()> {
		self.push(ElementValue::Class(class));
		Ok(())
	}

	fn visit_annotation(self, annotation_type: FieldDescriptor) -> Result<(Self::AnnotationResidual, Self::AnnotationVisitor)> {
		Ok((self, Annotation::new(annotation_type)))
	}

	fn finish_annotation(mut this: Self::AnnotationResidual, annotation_visitor: Self::AnnotationVisitor) -> Result<Self> {
		this.push(ElementValue::AnnotationInterface(annotation_visitor));
		Ok(this)
	}

	fn visit_array(self) -> Result<(Self::AnnotationArrayResidual, Self::AnnotationArrayVisitor)> {
		Ok((self, Vec::new()))
	}

	fn finish_array(mut this: Self::AnnotationArrayResidual, annotation_array_visitor: Self::AnnotationArrayVisitor) -> Result<Self> {
		this.push(ElementValue::ArrayType(annotation_array_visitor));
		Ok(this)
	}
}

impl UnknownAttributeVisitor for Attribute {
	fn read(name: String, bytes: Vec<u8>, _pool: &PoolRead) -> Result<Self> {
		Ok(Attribute {
			name,
			bytes,
		})
	}

	fn from_attribute(attribute: Attribute) -> Result<Option<Self>> {
		Ok(Some(attribute))
	}
}

impl FieldVisitor for Field {
	type AnnotationsVisitor = Vec<Annotation>;
	type AnnotationsResidual = (Self, bool);
	type TypeAnnotationsVisitor = Vec<TypeAnnotation<TargetInfoField>>;
	type TypeAnnotationsResidual = (Self, bool);
	type UnknownAttribute = Attribute;

	fn interests(&self) -> FieldInterests {
		FieldInterests::all()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, deprecated: bool, synthetic: bool) -> Result<()> {
		self.has_deprecated_attribute = deprecated;
		self.has_synthetic_attribute = synthetic;
		Ok(())
	}

	fn visit_constant_value(&mut self, constant_value: ConstantValue) -> Result<()> {
		self.constant_value.insert_if_empty(constant_value).context("only one ConstantValue attribute is allowed")
	}

	fn visit_signature(&mut self, signature: FieldSignature) -> Result<()> {
		self.signature.insert_if_empty(signature).context("only one Signature attribute is allowed")
	}

	fn visit_annotations(self, visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_annotations((mut this, visible): Self::AnnotationsResidual, annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleAnnotations attribute is allowed")
			this.runtime_visible_annotations.extend(annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleAnnotations attribute is allowed")
			this.runtime_invisible_annotations.extend(annotations_visitor);
		}
		Ok(this)
	}

	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_type_annotations((mut this, visible): Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleTypeAnnotations attribute is allowed")
			this.runtime_visible_type_annotations.extend(type_annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleTypeAnnotations attribute is allowed")
			this.runtime_invisible_type_annotations.extend(type_annotations_visitor);
		}
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		self.attributes.push(unknown_attribute);
		Ok(())
	}
}

impl MethodVisitor for Method {
	type AnnotationsVisitor = Vec<Annotation>;
	type AnnotationsResidual = (Self, bool);
	type TypeAnnotationsVisitor = Vec<TypeAnnotation<TargetInfoMethod>>;
	type TypeAnnotationsResidual = (Self, bool);
	type AnnotationDefaultVisitor = Vec<ElementValue>;
	type AnnotationDefaultResidual = Self;
	type CodeVisitor = Code;
	type UnknownAttribute = Attribute;

	fn interests(&self) -> MethodInterests {
		MethodInterests::all()
	}

	fn visit_deprecated_and_synthetic_attribute(&mut self, deprecated: bool, synthetic: bool) -> Result<()> {
		self.has_deprecated_attribute = deprecated;
		self.has_synthetic_attribute = synthetic;
		Ok(())
	}

	fn visit_exceptions(&mut self, exceptions: Vec<ClassName>) -> Result<()> {
		self.exceptions.insert_if_empty(exceptions).context("only one Exceptions attribute is allowed")
	}

	fn visit_signature(&mut self, signature: MethodSignature) -> Result<()> {
		self.signature.insert_if_empty(signature).context("only one Signature attribute is allowed")
	}

	fn visit_annotations(self, visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_annotations((mut this, visible): Self::AnnotationsResidual, annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleAnnotations attribute is allowed")
			this.runtime_visible_annotations.extend(annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleAnnotations attribute is allowed")
			this.runtime_invisible_annotations.extend(annotations_visitor);
		}
		Ok(this)
	}

	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_type_annotations((mut this, visible): Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleTypeAnnotations attribute is allowed")
			this.runtime_visible_type_annotations.extend(type_annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleTypeAnnotations attribute is allowed")
			this.runtime_invisible_type_annotations.extend(type_annotations_visitor);
		}
		Ok(this)
	}

	fn visit_annotation_default(self) -> Result<(Self::AnnotationDefaultResidual, Self::AnnotationDefaultVisitor)> {
		Ok((self, Vec::new()))
	}

	fn finish_annotation_default(mut this: Self::AnnotationDefaultResidual, mut element_value_visitor: Self::AnnotationDefaultVisitor) -> Result<Self> {
		if element_value_visitor.len() != 1 {
			bail!("didn't get proper number of `element_value`s: expected exactly one: {element_value_visitor:?}");
		}
		let element_value = element_value_visitor.pop().unwrap();

		this.annotation_default = Some(element_value);

		Ok(this)
	}

	fn visit_parameters(&mut self, method_parameters: Vec<MethodParameter>) -> Result<()> {
		self.method_parameters.insert_if_empty(method_parameters).context("only one MethodParameters attribute is allowed")
	}

	fn visit_annotable_parameter_count(&mut self) {
		todo!()
	}

	fn visit_parameter_annotation(&mut self) {
		todo!()
	}

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		self.attributes.push(unknown_attribute);
		Ok(())
	}

	fn visit_code(&mut self) -> Result<Option<Self::CodeVisitor>> {
		Ok(Some(Code::default()))
	}

	fn finish_code(&mut self, code_visitor: Self::CodeVisitor) -> Result<()> {
		self.code.insert_if_empty(code_visitor).context("only one Code attribute is allowed")
	}
}

impl CodeVisitor for Code {
	type TypeAnnotationsVisitor = Vec<TypeAnnotation<TargetInfoCode>>;
	type TypeAnnotationsResidual = (Self, bool);
	type UnknownAttribute = Attribute;

	fn interests(&self) -> CodeInterests {
		CodeInterests::all()
	}

	fn visit_max_stack_and_max_locals(&mut self, max_stack: u16, max_locals: u16) -> Result<()> {
		self.max_stack = Some(max_stack);
		self.max_locals = Some(max_locals);
		Ok(())
	}

	fn visit_exception_table(&mut self, exception_table: Vec<Exception>) -> Result<()> {
		self.exception_table = exception_table;
		Ok(())
	}

	fn visit_instruction(&mut self, label: Option<Label>, frame: Option<StackMapData>, instruction: Instruction) -> Result<()> {
		self.instructions.push(InstructionListEntry {
			label,
			frame,
			instruction,
		});
		Ok(())
	}
	fn visit_last_label(&mut self, last_label: Label) -> Result<()> {
		self.last_label.insert_if_empty(last_label).context("you may only visit the last label once")
	}

	fn visit_line_numbers(&mut self, line_number_table: Vec<(Label, u16)>) -> Result<()> {
		self.line_numbers.insert_if_empty(line_number_table).context("you may only visit the line number table once")
	}

	fn visit_local_variables(&mut self, local_variables: Vec<Lv>) -> Result<()> {
		self.local_variables.insert_if_empty(local_variables).context("you may only visit the local variables once")
	}

	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_type_annotations((mut this, visible): Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		if visible {
			this.runtime_visible_type_annotations.extend(type_annotations_visitor);
		} else {
			this.runtime_invisible_type_annotations.extend(type_annotations_visitor);
		}
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		self.attributes.push(unknown_attribute);
		Ok(())
	}
}

impl RecordComponentVisitor for RecordComponent {
	type AnnotationsVisitor = Vec<Annotation>;
	type AnnotationsResidual = (Self, bool);
	type TypeAnnotationsVisitor = Vec<TypeAnnotation<TargetInfoField>>;
	type TypeAnnotationsResidual = (Self, bool);
	type UnknownAttribute = Attribute;

	fn interests(&self) -> RecordComponentInterests {
		RecordComponentInterests::all()
	}

	fn visit_signature(&mut self, signature: FieldSignature) -> Result<()> {
		self.signature.insert_if_empty(signature).context("only one Signature attribute is allowed")
	}

	fn visit_annotations(self, visible: bool) -> Result<(Self::AnnotationsResidual, Self::AnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_annotations((mut this, visible): Self::AnnotationsResidual, annotations_visitor: Self::AnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleAnnotations attribute is allowed")
			this.runtime_visible_annotations.extend(annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleAnnotations attribute is allowed")
			this.runtime_invisible_annotations.extend(annotations_visitor);
		}
		Ok(this)
	}


	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)> {
		Ok(((self, visible), Vec::new()))
	}

	fn finish_type_annotations((mut this, visible): Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self> {
		if visible {
			// TODO: .context("only one RuntimeVisibleTypeAnnotations attribute is allowed")
			this.runtime_visible_type_annotations.extend(type_annotations_visitor);
		} else {
			// TODO: .context("only one RuntimeInvisibleTypeAnnotations attribute is allowed")
			this.runtime_invisible_type_annotations.extend(type_annotations_visitor);
		}
		Ok(this)
	}

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()> {
		self.attributes.push(unknown_attribute);
		Ok(())
	}
}
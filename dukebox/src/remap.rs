use anyhow::Result;
use indexmap::IndexMap;
use duke::tree::annotation::{Annotation, ElementValue, ElementValuePair};
use duke::tree::class::{ClassFile, ClassName, ClassSignature, EnclosingMethod, InnerClass};
use duke::tree::field::{Field, FieldDescriptor, FieldRef, FieldSignature};
use duke::tree::method::{Method, MethodDescriptor, MethodParameter, MethodRef, MethodSignature};
use duke::tree::method::code::{Code, ConstantDynamic, Exception, Handle, Instruction, InstructionListEntry, InvokeDynamic, Loadable, Lv};
use duke::tree::type_annotation::TypeAnnotation;
use duke::visitor::method::code::{StackMapData, VerificationTypeInfo};
use quill::remapper::BRemapper;
use crate::{Jar, JarEntry, OpenedJar};
use crate::parsed::{ParsedJar, ParsedJarEntry};


// TODO: doc
pub fn remap(jar: impl Jar, remapper: impl BRemapper) -> Result<ParsedJar> {
	let mut opened = jar.open()?;

	let mut resulting_entries = IndexMap::new();

	for key in opened.entry_keys() {
		let entry = opened.by_entry_key(key)?;

		let name = entry.name().to_owned(); // TODO: also remap the entry name for a class!

		let entry = remap_jar_entry(entry.to_parsed_jar_entry()?, &remapper)?;

		resulting_entries.insert(name, entry);
	}

	Ok(ParsedJar { entries: resulting_entries })
}

pub fn remap_jar_entry_name(name: String, remapper: &impl BRemapper) -> Result<String> {
	return Ok(name);
	todo!("remap a name of a jar entry")
}

pub fn remap_jar_entry(entry: ParsedJarEntry, remapper: &impl BRemapper) -> Result<ParsedJarEntry> {
	Ok(match entry {
		ParsedJarEntry::Class { attr, class } => {
			let class = class.read()?.remap(remapper)?.into();
			ParsedJarEntry::Class { attr, class }
		},
		e @ ParsedJarEntry::Other { .. } => e,
		e @ ParsedJarEntry::Dir { .. } => e,
	})
}

trait Mappable<Output = Self>: Sized {
	fn remap(self, remapper: &impl BRemapper) -> Result<Output>;
}

trait MappableWithClassName<Output = Self>: Sized {
	fn remap_with_class_name(self, remapper: &impl BRemapper, this_class: &ClassName) -> Result<Output>;
}

impl<T> Mappable for T where for<'a> &'a T: Mappable<T> {
	fn remap(self, remapper: &impl BRemapper) -> Result<T> {
		(&self).remap(remapper)
	}
}

impl<T, U> Mappable<Option<U>> for Option<T> where T: Mappable<U> {
	fn remap(self, remapper: &impl BRemapper) -> Result<Option<U>> {
		self.map(|x| x.remap(remapper)).transpose()
	}
}

impl<T, U> Mappable<Vec<U>> for Vec<T> where T: Mappable<U> {
	fn remap(self, remapper: &impl BRemapper) -> Result<Vec<U>> {
		self.into_iter()
			.map(|i| i.remap(remapper))
			.collect()
	}
}
impl<T, U> MappableWithClassName<Vec<U>> for Vec<T> where T: MappableWithClassName<U> {
	fn remap_with_class_name(self, remapper: &impl BRemapper, this_class: &ClassName) -> Result<Vec<U>> {
		self.into_iter()
			.map(|i| i.remap_with_class_name(remapper, this_class))
			.collect()
	}
}

impl Mappable<ClassName> for &ClassName {
	fn remap(self, remapper: &impl BRemapper) -> Result<ClassName> {
		remapper.map_class(self)
	}
}

impl Mappable for ClassSignature {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		eprintln!("todo: impl remap class signature for {self:?}");
		return Ok(self);
		todo!("remap class signature for {self:?}")
	}
}

impl Mappable for ClassFile {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(ClassFile {
			version: self.version,
			access: self.access,
			name: (&self.name).remap(remapper)?,
			super_class: self.super_class.remap(remapper)?,
			interfaces: self.interfaces.remap(remapper)?,

			fields: self.fields.remap_with_class_name(remapper, &self.name)?,
			methods: self.methods.remap_with_class_name(remapper, &self.name)?,

			has_deprecated_attribute: self.has_deprecated_attribute,
			has_synthetic_attribute: self.has_synthetic_attribute,

			inner_classes: self.inner_classes.remap(remapper)?,
			enclosing_method: self.enclosing_method.remap(remapper)?,
			signature: self.signature.remap(remapper)?,

			source_file: self.source_file, // TODO
			source_debug_extension: self.source_debug_extension, // TODO

			runtime_visible_annotations: self.runtime_visible_annotations.remap(remapper)?,
			runtime_invisible_annotations: self.runtime_invisible_annotations.remap(remapper)?,
			runtime_visible_type_annotations: self.runtime_visible_type_annotations.remap(remapper)?,
			runtime_invisible_type_annotations: self.runtime_invisible_type_annotations.remap(remapper)?,

			module: None, // TODO
			module_packages: None, // TODO
			module_main_class: None, // TODO

			nest_host_class: None, // TODO
			nest_members: None, // TODO
			permitted_subclasses: None, // TODO

			record_components: Vec::new(), // TODO (takes in self.name as well)

			attributes: Vec::new(), // TODO
		})
	}
}

impl Mappable for FieldRef {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		remapper.map_field_ref(&self)
	}
}

impl Mappable for FieldDescriptor {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		remapper.map_field_desc(&self)
	}
}

impl Mappable for FieldSignature {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		eprintln!("todo: impl remap field signature for {self:?}");
		return Ok(self);
		todo!("remap field signature for {self:?}")
	}
}

impl Mappable for MethodRef {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		remapper.map_method_ref(&self)
	}
}

impl Mappable for MethodDescriptor {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		remapper.map_method_desc(&self)
	}
}

impl Mappable for MethodSignature {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		eprintln!("todo: impl remap method signature for {self:?}");
		return Ok(self);
		todo!("remap method signature for {self:?}")
	}
}

impl MappableWithClassName for Field {
	fn remap_with_class_name(self, remapper: &impl BRemapper, this_class: &ClassName) -> Result<Self> {
		let name_and_desc = remapper.map_field(this_class, &self.name, &self.descriptor)?;
		Ok(Field {
			access: self.access,
			name: name_and_desc.name,
			descriptor: name_and_desc.desc,

			has_deprecated_attribute: self.has_deprecated_attribute,
			has_synthetic_attribute: self.has_synthetic_attribute,

			constant_value: self.constant_value,
			signature: self.signature.remap(remapper)?,

			runtime_visible_annotations: self.runtime_visible_annotations.remap(remapper)?,
			runtime_invisible_annotations: self.runtime_invisible_annotations.remap(remapper)?,
			runtime_visible_type_annotations: self.runtime_visible_type_annotations.remap(remapper)?,
			runtime_invisible_type_annotations: self.runtime_invisible_type_annotations.remap(remapper)?,

			attributes: Vec::new(), // TODO
		})
	}
}

impl MappableWithClassName for Method {
	fn remap_with_class_name(self, remapper: &impl BRemapper, this_class: &ClassName) -> Result<Self> {
		let name_and_desc = remapper.map_method(this_class, &self.name, &self.descriptor)?;
		Ok(Method {
			access: self.access,
			name: name_and_desc.name,
			descriptor: name_and_desc.desc,

			has_deprecated_attribute: self.has_deprecated_attribute,
			has_synthetic_attribute: self.has_synthetic_attribute,

			code: self.code.remap(remapper)?,
			exceptions: self.exceptions.remap(remapper)?,
			signature: self.signature.remap(remapper)?,

			runtime_visible_annotations: self.runtime_visible_annotations.remap(remapper)?,
			runtime_invisible_annotations: self.runtime_invisible_annotations.remap(remapper)?,
			runtime_visible_type_annotations: self.runtime_visible_type_annotations.remap(remapper)?,
			runtime_invisible_type_annotations: self.runtime_invisible_type_annotations.remap(remapper)?,

			annotation_default: self.annotation_default.remap(remapper)?,
			method_parameters: self.method_parameters.remap(remapper)?,

			attributes: Vec::new(), // TODO:
		})
	}
}

impl Mappable for InnerClass {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		fn map_inner_class_name(remapper: &impl BRemapper, name: &ClassName, outer_class: Option<&ClassName>, inner_name: &String) -> Result<String> {
			return Ok(inner_name.clone());
			todo!()
		}

		Ok(InnerClass {
			inner_class: (&self.inner_class).remap(remapper)?,
			outer_class: self.outer_class.as_ref().remap(remapper)?,
			inner_name: self.inner_name.map(|inner_name| map_inner_class_name(
				remapper,
				&self.inner_class,
				self.outer_class.as_ref(),
				&inner_name
			)).transpose()?,
			flags: self.flags,
		})
	}
}

impl Mappable for EnclosingMethod {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(EnclosingMethod {
			class: remapper.map_class(&self.class)?,
			method: self.method.map(|method| remapper.map_method_name_and_desc(&self.class, &method)).transpose()?
		})
	}
}

impl Mappable for Annotation {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(Annotation {
			annotation_type: self.annotation_type.remap(remapper)?,
			element_value_pairs: self.element_value_pairs.remap(remapper)?,
		})
	}
}

impl<T> Mappable for TypeAnnotation<T> {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(TypeAnnotation {
			type_reference: self.type_reference,
			type_path: self.type_path,
			annotation: self.annotation.remap(remapper)?,
		})
	}
}

impl Mappable for ElementValuePair {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(ElementValuePair {
			name: self.name,
			value: self.value.remap(remapper)?,
		})
	}
}

impl Mappable for ElementValue {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		use ElementValue::*;
		Ok(match self {
			Object(x) => Object(x),
			Enum { type_name, const_name } => Enum { type_name, const_name }, // TODO: these two need remapping!
			Class(class_name) => Class(remapper.map_desc(&class_name)?), // this is enough for the return descriptor
			AnnotationInterface(annotation) => AnnotationInterface(annotation.remap(remapper)?),
			ArrayType(vec) => ArrayType(vec.remap(remapper)?),
		})
	}
}

impl Mappable for Code {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(Code {
			max_stack: self.max_stack,
			max_locals: self.max_locals,

			instructions: self.instructions.remap(remapper)?,
			exception_table: self.exception_table.remap(remapper)?,
			last_label: self.last_label,

			line_numbers: self.line_numbers,
			local_variables: self.local_variables.remap(remapper)?,

			runtime_visible_type_annotations: self.runtime_visible_type_annotations.remap(remapper)?,
			runtime_invisible_type_annotations: self.runtime_invisible_type_annotations.remap(remapper)?,

			attributes: Vec::new(), // TODO:
		})
	}
}

impl Mappable for InstructionListEntry {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(InstructionListEntry {
			label: self.label,
			frame: self.frame.remap(remapper)?,
			instruction: self.instruction.remap(remapper)?,
		})
	}
}

impl Mappable for StackMapData {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		use StackMapData::*;
		Ok(match self {
			Same => Same,
			SameLocals1StackItem { stack } => SameLocals1StackItem {
				stack: stack.remap(remapper)?,
			},
			Chop { k } => Chop { k },
			Append { locals } => Append {
				locals: locals.remap(remapper)?,
			},
			Full { locals, stack } => Full {
				locals: locals.remap(remapper)?,
				stack: stack.remap(remapper)?,
			},
		})
	}
}

impl Mappable for VerificationTypeInfo {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		use VerificationTypeInfo::*;
		Ok(match self {
			Top => Top,
			Integer => Integer,
			Float => Float,
			Long => Long,
			Double => Double,
			Null => Null,
			UninitializedThis => UninitializedThis,
			Object(name) => Object(name.remap(remapper)?),
			Uninitialized(label) => Uninitialized(label),
		})
	}
}

impl Mappable for Instruction {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		use Instruction::*;
		Ok(match self {
			Nop |
			AConstNull |
			IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 |
			LConst0 | LConst1 |
			FConst0 | FConst1 | FConst2 |
			DConst0 | DConst1 |
			BiPush(_) |
			SiPush(_) => self,
			Ldc(loadable) => Ldc(loadable.remap(remapper)?),
			ILoad(_) | LLoad(_) | FLoad(_) | DLoad(_) | ALoad(_) |
			IALoad | LALoad | FALoad | DALoad | AALoad | BALoad | CALoad | SALoad |
			IStore(_) | LStore(_) | FStore(_) | DStore(_) | AStore(_) |
			IAStore | LAStore | FAStore | DAStore | AAStore | BAStore | CAStore | SAStore |
			Pop | Pop2 |
			Dup | DupX1 | DupX2 |
			Dup2 | Dup2X1 | Dup2X2 |
			Swap |
			IAdd | LAdd | FAdd | DAdd |
			ISub | LSub | FSub | DSub |
			IMul | LMul | FMul | DMul |
			IDiv | LDiv | FDiv | DDiv |
			IRem | LRem | FRem | DRem |
			INeg | LNeg | FNeg | DNeg |
			IShl | LShl |
			IShr | LShr |
			IUShr | LUShr |
			IAnd | LAnd |
			IOr | LOr |
			IXor | LXor |
			IInc(_, _) |
			I2L | I2F | I2D |
			L2I | L2F | L2D |
			F2I | F2L | F2D |
			D2I | D2L | D2F |
			I2B | I2C | I2S |
			LCmp |
			FCmpL | FCmpG |
			DCmpL | DCmpG |
			IfEq(_) | IfNe(_) | IfLt(_) | IfGe(_) | IfGt(_) | IfLe(_) |
			IfICmpEq(_) | IfICmpNe(_) | IfICmpLt(_) | IfICmpGe(_) | IfICmpGt(_) | IfICmpLe(_) |
			IfACmpEq(_) | IfACmpNe(_) |
			Goto(_) |
			Jsr(_) |
			Ret(_) |
			TableSwitch { .. } |
			LookupSwitch { .. } |
			IReturn | LReturn | FReturn | DReturn | AReturn |
			Return => self,
			GetStatic(field_ref) => GetStatic(field_ref.remap(remapper)?),
			PutStatic(field_ref) => PutStatic(field_ref.remap(remapper)?),
			GetField(field_ref) => GetField(field_ref.remap(remapper)?),
			PutField(field_ref) => PutField(field_ref.remap(remapper)?),
			InvokeVirtual(method_ref) => InvokeVirtual(method_ref.remap(remapper)?),
			InvokeSpecial(method_ref, is_interface) => InvokeSpecial(method_ref.remap(remapper)?, is_interface),
			InvokeStatic(method_ref, is_interface) => InvokeStatic(method_ref.remap(remapper)?, is_interface),
			InvokeInterface(method_ref) => InvokeInterface(method_ref.remap(remapper)?),
			InvokeDynamic(invoke_dynamic) => InvokeDynamic(invoke_dynamic.remap(remapper)?),
			New(class_name) => New(class_name.remap(remapper)?),
			NewArray(_) => self,
			// TODO: the array operations need some checking -> possibly new method for remapping array class names?
			ANewArray(class_name) => ANewArray(class_name.remap(remapper)?),
			ArrayLength |
			AThrow => self,
			CheckCast(class_name) => CheckCast(class_name.remap(remapper)?),
			InstanceOf(class_name) => InstanceOf(class_name.remap(remapper)?),
			MonitorEnter | MonitorExit => self,
			MultiANewArray(class_name, dimensions) => MultiANewArray(class_name.remap(remapper)?, dimensions),
			IfNull(_) | IfNonNull(_) => self,
		})
	}
}

impl Mappable for Loadable {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		use Loadable::*;
		Ok(match self {
			Integer(_) | Float(_) | Long(_) | Double(_) => self,
			Class(class_name) => Class(class_name.remap(remapper)?),
			String(string) => String(string),
			MethodHandle(handle) => MethodHandle(handle.remap(remapper)?),
			MethodType(desc) => MethodType(desc.remap(remapper)?),
			Dynamic(constant_dynamic) => Dynamic(constant_dynamic.remap(remapper)?),
		})
	}
}

// TODO: I think some of these need the current class name to be properly remapped...
//  so convert them to the MappableWithClassName trait

impl Mappable for Handle {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for ConstantDynamic {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for InvokeDynamic {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for Exception {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(Exception {
			start: self.start,
			end: self.end,
			handler: self.handler,
			catch: self.catch.remap(remapper)?,
		})
	}
}

impl Mappable for Lv {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(Lv {
			range: self.range,
			name: self.name, // TODO: lv name remapping
			descriptor: self.descriptor.remap(remapper)?,
			signature: self.signature.remap(remapper)?,
			index: self.index,
		})
	}
}

impl Mappable for MethodParameter {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		// TODO: remapper doesn't support parameter names yet!
		Ok(self)
	}
}
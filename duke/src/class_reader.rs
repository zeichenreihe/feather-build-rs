use std::io::Cursor;
use std::ops::ControlFlow;
use anyhow::{anyhow, bail, Context, Result};
use crate::class_constants::{attribute, opcode, type_annotation};
use crate::class_reader::labels::Labels;
use crate::class_reader::pool::{BootstrapMethodRead, PoolRead};
use crate::{class_constants, ClassRead, jstring, OptionExpansion};
use crate::tree::annotation::Object;
use crate::tree::class::{ClassAccess, ClassSignature, EnclosingMethod, InnerClass};
use crate::tree::descriptor::ReturnDescriptor;
use crate::tree::field::{FieldAccess, FieldDescriptor, FieldName, FieldSignature};
use crate::tree::method::{MethodAccess, MethodDescriptor, MethodName, MethodParameter, MethodSignature, ParameterFlags};
use crate::tree::method::code::{ArrayType, Exception, Instruction, LocalVariableName, Lv, LvIndex};
use crate::tree::module::{Module, ModuleExports, ModuleOpens, ModuleProvides, ModuleRequires};
use crate::tree::record::RecordName;
use crate::tree::type_annotation::{TargetInfoClass, TargetInfoCode, TargetInfoField, TargetInfoMethod, TypePath, TypePathKind};
use crate::tree::version::Version;
use crate::visitor::annotation::{NamedElementValuesVisitor, UnnamedElementValuesVisitor, AnnotationsVisitor, TypeAnnotationsVisitor, UnnamedElementValueVisitor};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::class::ClassVisitor;
use crate::visitor::field::FieldVisitor;
use crate::visitor::method::code::{CodeVisitor, StackMapData, VerificationTypeInfo};
use crate::visitor::method::MethodVisitor;
use crate::visitor::MultiClassVisitor;
use crate::visitor::record::RecordComponentVisitor;

pub(crate) mod pool; // needs to be pub(crate) because of the UnknownAttributeVisitor
mod labels;

/// Skips the `attributes_count` and `attributes` items of the structs.
///
/// This is needed whenever we skip reading something, like a class, field, method.
fn skip_attributes(reader: &mut impl ClassRead) -> Result<()> {
	let attributes_count = reader.read_u16()?;

	for _ in 0..attributes_count {
		let _attribute_name_index = reader.read_u16()?;
		let attribute_length = reader.read_u32()?;

		// Skip the attribute data
		reader.skip(attribute_length as i64)?;
	}

	Ok(())
}

/// Reads a class file from a reader into the [`MultiClassVisitor`].
//TODO: MultiClassVisitor should be changed into a two part thing like with NamedElementValue**s**Visitor and NamedElementValue****Visitor
// this would allow us to have a visitor that "can return max 1 class" and a subtrait that also specifies "and can be called more often"
pub(crate) fn read<V: MultiClassVisitor>(reader: &mut impl ClassRead, visitor: V) -> Result<V> {
	let magic = reader.read_u32()?;
	if magic != class_constants::MAGIC {
		bail!("wrong magic: got {magic:#x}, expected 0xCAFEBABE");
	}

	let minor = reader.read_u16()?;
	let major = reader.read_u16()?;
	let version = Version::new(major, minor);

	if version > Version::V23 {
		bail!("unsupported class file version: {version:?}");
	}

	let pool_ = PoolRead::read(reader)?;
	let pool = &pool_;

	let access_flags: ClassAccess = reader.read_u16()?.into();
	let this_class = pool.get_class(reader.read_u16()?)?;
	let super_class = pool.get_optional(reader.read_u16()?, PoolRead::get_class)?;
	let interfaces = reader.read_vec(
		|r| r.read_u16_as_usize(),
		|r| pool.get_class(r.read_u16()?)
	)?;

	// We take a reference to the start of the fields and methods items so that we can read them after we've visited the attributes of the class itself.
	let fields_start = reader.marker()?;

	// We skip the fields.
	for _ in 0..reader.read_u16()? {
		// Per field we skip 2 bytes for the access flags, another 2 for the name, and another 2 for the descriptor.
		reader.skip(2 + 2 + 2)?;

		skip_attributes(reader)?;
	}
	// Methods have the same structure as fields.
	for _ in 0..reader.read_u16()? {
		reader.skip(2 + 2 + 2)?;

		skip_attributes(reader)?;
	}

	match visitor.visit_class(version, access_flags, this_class.clone(), super_class, interfaces)? {
		ControlFlow::Continue((visitor, mut class_visitor)) => {
			let interests = class_visitor.interests();

			// Important Note regarding bootstrap methods:
			// The BootstrapMethods_attribute must be parsed fully before any attempt at loading a loadable constant pool entry.
			// The attribute is used at the following locations:
			//  - in the arguments for the bootstrap methods,
			//  - for the ConstantValue_attribute of a field, and
			//  - The ldc, ldc_w, ldc2_l and invokedynamic instructions
			// This means that we need to first read the class attributes and then the fields and methods.
			// We also need to lazily deal with the bootstrap method arguments.

			// TODO: put in a limit into any recursive thing here (annotations??, bootstrap methods) (see other todos)

			let (mut is_deprecated, mut is_synthetic) = (false, false);

			let mut bootstrap_methods = None;

			let mut had_record_attribute = false;

			let attributes_count = reader.read_u16()?;
			for _ in 0..attributes_count {
				let attribute_name = pool.get_utf8_ref(reader.read_u16()?)?;
				let length = reader.read_u32()?;

				match attribute_name.as_java_str() {
					name if name == attribute::DEPRECATED => {
						is_deprecated = true;
					},
					name if name == attribute::SYNTHETIC => {
						is_synthetic = true;
					},
					name if name == attribute::INNER_CLASSES && !interests.inner_classes => reader.skip(length as i64)?,
					name if name == attribute::INNER_CLASSES => {
						let inner_classes = reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| {
								Ok(InnerClass {
									inner_class: pool.get_class(r.read_u16()?)?,
									outer_class: pool.get_optional(r.read_u16()?, PoolRead::get_class)?,
									inner_name: pool.get_optional(r.read_u16()?, PoolRead::get_utf8)?,
									flags: r.read_u16()?.into(),
								})
							}
						)?;
						class_visitor.visit_inner_classes(inner_classes)?;
					},
					name if name == attribute::ENCLOSING_METHOD && !interests.enclosing_method => reader.skip(length as i64)?,
					name if name == attribute::ENCLOSING_METHOD => {
						let class = pool.get_class(reader.read_u16()?)?;
						let method = pool.get_optional(reader.read_u16()?, PoolRead::get_method_name_and_type)?;
						let enclosing_method = EnclosingMethod { class, method };

						class_visitor.visit_enclosing_method(enclosing_method)?;
					},
					name if name == attribute::SIGNATURE && !interests.signature => reader.skip(length as i64)?,
					name if name == attribute::SIGNATURE => {
						let signature = ClassSignature::try_from(pool.get_utf8(reader.read_u16()?)?)?;
						class_visitor.visit_signature(signature)?;
					},
					name if name == attribute::SOURCE_FILE && !interests.source_file => reader.skip(length as i64)?,
					name if name == attribute::SOURCE_FILE => {
						let source_file = pool.get_utf8(reader.read_u16()?)?;
						class_visitor.visit_source_file(source_file)?;
					},
					name if name == attribute::SOURCE_DEBUG_EXTENSION && !interests.source_debug_extension => reader.skip(length as i64)?,
					name if name == attribute::SOURCE_DEBUG_EXTENSION => {
						let source_debug_extension = jstring::from_vec_to_string(reader.read_u8_vec(length as usize)?)?;
						class_visitor.visit_source_debug_extension(source_debug_extension)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS && !interests.runtime_visible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = class_visitor.visit_annotations(true)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						class_visitor = ClassVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS && !interests.runtime_invisible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = class_visitor.visit_annotations(false)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						class_visitor = ClassVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS && !interests.runtime_visible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = class_visitor.visit_type_annotations(true)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						class_visitor = ClassVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS && !interests.runtime_invisible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = class_visitor.visit_type_annotations(false)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						class_visitor = ClassVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					name if name == attribute::MODULE && !interests.module => reader.skip(length as i64)?,
					name if name == attribute::MODULE => {
						let module = read_module(reader, pool)?;
						class_visitor.visit_module(module)?;
					},
					name if name == attribute::MODULE_PACKAGES && !interests.module_packages => reader.skip(length as i64)?,
					name if name == attribute::MODULE_PACKAGES => {
						let module_packages = reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| pool.get_package(r.read_u16()?)
						)?;
						class_visitor.visit_module_packages(module_packages)?;
					},
					name if name == attribute::MODULE_MAIN_CLASS && !interests.module_main_class => reader.skip(length as i64)?,
					name if name == attribute::MODULE_MAIN_CLASS => {
						let module_main_class = pool.get_class(reader.read_u16()?)?;
						class_visitor.visit_module_main_class(module_main_class)?;
					},
					name if name == attribute::NEST_HOST && !interests.nest_host => reader.skip(length as i64)?,
					name if name == attribute::NEST_HOST => {
						let nest_host_class = pool.get_class(reader.read_u16()?)?;
						class_visitor.visit_nest_host_class(nest_host_class)?;
					},
					name if name == attribute::NEST_MEMBERS && !interests.nest_members => reader.skip(length as i64)?,
					name if name == attribute::NEST_MEMBERS => {
						let nest_members = reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| pool.get_class(r.read_u16()?)
						)?;
						class_visitor.visit_nest_members(nest_members)?;
					},
					name if name == attribute::PERMITTED_SUBCLASSES && !interests.permitted_subclasses => reader.skip(length as i64)?,
					name if name == attribute::PERMITTED_SUBCLASSES => {
						let permitted_subclasses = reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| pool.get_class(r.read_u16()?)
						)?;
						class_visitor.visit_permitted_subclasses(permitted_subclasses)?;
					},
					name if name == attribute::RECORD && !interests.record => reader.skip(length as i64)?,
					name if name == attribute::RECORD => {
						if had_record_attribute {
							bail!("only one Record attribute is allowed");
						}
						had_record_attribute = true;

						let components_length = reader.read_u16()?;
						for _ in 0..components_length {
							class_visitor = read_record_component(reader, class_visitor, pool)?;
						}
					},
					name if name == attribute::BOOTSTRAP_METHODS => {
						let methods = reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| Ok(BootstrapMethodRead {
								handle: pool.get_method_handle(r.read_u16()?)?,
								arguments: r.read_vec(|r| r.read_u16_as_usize(), |r| r.read_u16())?,
							})
						)?;
						bootstrap_methods.insert_if_empty(methods).context("only one BootstrapMethods attribute is allowed")?;
					},
					_ if !interests.unknown_attributes => reader.skip(length as i64)?,
					_ => {
						let vec = reader.read_u8_vec(length as usize)?;
						let attribute = UnknownAttributeVisitor::read(attribute_name.clone(), vec, pool)?;
						class_visitor.visit_unknown_attribute(attribute)?;
					}
				}
			}

			class_visitor.visit_deprecated_and_synthetic_attribute(is_deprecated, is_synthetic)?;

			// Visit the fields and methods. We jump back to the end of the class file to allow people to concat class files directly.
			reader.with_pos(fields_start, |reader| {
				let fields_count = reader.read_u16()?;
				for _ in 0..fields_count {
					class_visitor = read_field(reader, class_visitor, pool)
						.with_context(|| anyhow!("failed to read field of class {this_class:?}"))?;
				}

				let methods_count = reader.read_u16()?;
				for _ in 0..methods_count {
					class_visitor = read_method(reader, class_visitor, pool, &bootstrap_methods)
						.with_context(|| anyhow!("failed to read method of class {this_class:?}"))?;
				}

				MultiClassVisitor::finish_class(visitor, class_visitor)
			})
		},
		ControlFlow::Break(visitor) => {
			skip_attributes(reader)?;
			Ok(visitor)
		}
	}
}

fn read_field<C: ClassVisitor>(reader: &mut impl ClassRead, visitor: C, pool: &PoolRead) -> Result<C> {
	let access = FieldAccess::from(reader.read_u16()?);
	let name = FieldName::try_from(pool.get_utf8(reader.read_u16()?)?)?;
	let descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;

	match visitor.visit_field(access, name, descriptor)? {
		ControlFlow::Continue((visitor, mut field_visitor)) => {
			let interests = field_visitor.interests();

			let (mut is_deprecated, mut is_synthetic) = (false, false);

			let attributes_count = reader.read_u16()?;
			for _ in 0..attributes_count {
				let attribute_name = pool.get_utf8_ref(reader.read_u16()?)?;
				let length = reader.read_u32()?;

				match attribute_name.as_java_str() {
					name if name == attribute::DEPRECATED => {
						is_deprecated = true;
					},
					name if name == attribute::SYNTHETIC => {
						is_synthetic = true;
					},
					name if name == attribute::CONSTANT_VALUE && !interests.constant_value => reader.skip(length as i64)?,
					name if name == attribute::CONSTANT_VALUE => {
						let constant_value = pool.get_constant_value(reader.read_u16()?)?;
						field_visitor.visit_constant_value(constant_value)?;
					},
					name if name == attribute::SIGNATURE && !interests.signature => reader.skip(length as i64)?,
					name if name == attribute::SIGNATURE => {
						let signature = FieldSignature::try_from(pool.get_utf8(reader.read_u16()?)?)?;
						field_visitor.visit_signature(signature)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS && !interests.runtime_visible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = field_visitor.visit_annotations(true)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						field_visitor = FieldVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS && !interests.runtime_invisible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = field_visitor.visit_annotations(false)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						field_visitor = FieldVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS && !interests.runtime_visible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = field_visitor.visit_type_annotations(true)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						field_visitor = FieldVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS && !interests.runtime_invisible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = field_visitor.visit_type_annotations(false)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						field_visitor = FieldVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					_ if !interests.unknown_attributes => reader.skip(length as i64)?,
					_ => {
						let vec = reader.read_u8_vec(length as usize)?;
						let attribute = UnknownAttributeVisitor::read(attribute_name.clone(), vec, pool)?;
						field_visitor.visit_unknown_attribute(attribute)?;
					},
				}
			}

			field_visitor.visit_deprecated_and_synthetic_attribute(is_deprecated, is_synthetic)?;

			ClassVisitor::finish_field(visitor, field_visitor)
		},
		ControlFlow::Break(visitor) => {
			skip_attributes(reader)?;
			Ok(visitor)
		}
	}
}

fn read_method<C: ClassVisitor>(reader: &mut impl ClassRead, visitor: C, pool: &PoolRead, bootstrap_methods: &Option<Vec<BootstrapMethodRead>>) -> Result<C> {
	let access = MethodAccess::from(reader.read_u16()?);
	let name = MethodName::try_from(pool.get_utf8(reader.read_u16()?)?)?;
	let descriptor = MethodDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;

	match visitor.visit_method(access, name.clone(), descriptor.clone())? {
		ControlFlow::Continue((visitor, mut method_visitor)) => {
			let interests = method_visitor.interests();

			let (mut is_deprecated, mut is_synthetic) = (false, false);

			let attributes_count = reader.read_u16()?;
			for _ in 0..attributes_count {
				let attribute_name = pool.get_utf8_ref(reader.read_u16()?)?;
				let length = reader.read_u32()?;

				match attribute_name.as_java_str() {
					name if name == attribute::DEPRECATED => {
						is_deprecated = true;
					},
					name if name == attribute::SYNTHETIC => {
						is_synthetic = true;
					},
					name if name == attribute::CODE && !interests.code => reader.skip(length as i64)?,
					name if name == attribute::CODE => {
						if let Some(code_visitor) = method_visitor.visit_code()? {
							let code_visitor = read_code(reader, code_visitor, pool, bootstrap_methods)
								.with_context(|| anyhow!("failed to read code of method {name:?} {descriptor:?}"))?;
							method_visitor.finish_code(code_visitor)?;
						}
					},
					name if name == attribute::EXCEPTIONS && !interests.exceptions => reader.skip(length as i64)?,
					name if name == attribute::EXCEPTIONS => {
						let exceptions = reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| pool.get_class(r.read_u16()?)
						)?;
						method_visitor.visit_exceptions(exceptions)?;
					},
					name if name == attribute::SIGNATURE && !interests.signature => reader.skip(length as i64)?,
					name if name == attribute::SIGNATURE => {
						let signature = MethodSignature::try_from(pool.get_utf8(reader.read_u16()?)?)?;
						method_visitor.visit_signature(signature)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS && !interests.runtime_visible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = method_visitor.visit_annotations(true)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						method_visitor = MethodVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS && !interests.runtime_invisible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = method_visitor.visit_annotations(false)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						method_visitor = MethodVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS && !interests.runtime_visible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = method_visitor.visit_type_annotations(true)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						method_visitor = MethodVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS && !interests.runtime_invisible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = method_visitor.visit_type_annotations(false)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						method_visitor = MethodVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_PARAMETER_ANNOTATIONS && !interests.runtime_visible_parameter_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_PARAMETER_ANNOTATIONS => {
						// TODO: RuntimeVisibleParameterAnnotations
						reader.skip(length as i64)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_PARAMETER_ANNOTATIONS && !interests.runtime_invisible_parameter_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_PARAMETER_ANNOTATIONS => {
						// TODO: RuntimeInvisibleParameterAnnotations
						reader.skip(length as i64)?;
					},
					name if name == attribute::ANNOTATION_DEFAULT && !interests.annotation_default => reader.skip(length as i64)?,
					name if name == attribute::ANNOTATION_DEFAULT => {
						let (visitor, x) = method_visitor.visit_annotation_default()?;
						let x = read_element_value_unnamed(reader, pool, x)?;
						method_visitor = MethodVisitor::finish_annotation_default(visitor, x)?;
					},
					name if name == attribute::METHOD_PARAMETERS && !interests.method_parameters => reader.skip(length as i64)?,
					name if name == attribute::METHOD_PARAMETERS => {
						let method_parameters = reader.read_vec(
							|r| r.read_u8_as_usize(),
							|r| Ok(MethodParameter {
								name: pool.get_optional(r.read_u16()?, PoolRead::get_utf8)?.map(|x| x.try_into()).transpose()?,
								flags: ParameterFlags::from(r.read_u16()?),
							})
						)?;
						method_visitor.visit_parameters(method_parameters)?;
					},
					_ if !interests.unknown_attributes => reader.skip(length as i64)?,
					_ => {
						let vec = reader.read_u8_vec(length as usize)?;
						let attribute = UnknownAttributeVisitor::read(attribute_name.clone(), vec, pool)?;
						method_visitor.visit_unknown_attribute(attribute)?;
					},
				}
			}

			method_visitor.visit_deprecated_and_synthetic_attribute(is_deprecated, is_synthetic)?;

			ClassVisitor::finish_method(visitor, method_visitor)
		},
		ControlFlow::Break(visitor) => {
			skip_attributes(reader)?;
			Ok(visitor)
		}
	}
}

/// A helper trait for the [`read_code`] method.
trait CodeReadHelper: ClassRead {
	fn read_u8_as_local_variable(&mut self) -> Result<LvIndex> {
		Ok(LvIndex { index: self.read_u8()? as u16 })
	}
	fn read_u16_as_local_variable(&mut self) -> Result<LvIndex> {
		Ok(LvIndex { index: self.read_u16()? })
	}

	fn read_i16_as_branch_target_label(&mut self, opcode_pos: u16) -> Result<u16> {
		let branch = self.read_i16()
			.with_context(|| anyhow!("couldn't read i16 for branch based on opcode {opcode_pos:?}"))?;
		let target = opcode_pos.checked_add_signed(branch)
			.with_context(|| anyhow!("can't add branch offset (i16) of {branch:?} to opcode position (u16) {opcode_pos:?}"))?;
		Ok(target)
	}

	fn read_i32_as_branch_target_label(&mut self, opcode_pos: u16) -> Result<u16> {
		let branch = self.read_i32()
			.with_context(|| anyhow!("couldn't read i32 for branch based on opcode {opcode_pos:?}"))?;
		let target = (opcode_pos as u32).checked_add_signed(branch)
			.with_context(|| anyhow!("can't add branch offset (i32) of {branch:?} to opcode position (u16) {opcode_pos:?}"))?;
		let target: u16 = target.try_into()?;
		Ok(target)
	}
}

fn align_to_4_byte_boundary(reader: &mut impl ClassRead) -> Result<()> {
	match reader.marker()? & 0b11 {
		0 => {},
		1 => { reader.read_u8()?; reader.read_u8()?; reader.read_u8()?; },
		2 => { reader.read_u8()?; reader.read_u8()?; },
		3 => { reader.read_u8()?; },
		_ => unreachable!(),
	};
	Ok(())
}

impl<T: ClassRead> CodeReadHelper for T {}

fn read_code<C: CodeVisitor>(
	reader: &mut impl ClassRead,
	mut code_visitor: C,
	pool: &PoolRead,
	bootstrap_methods: &Option<Vec<BootstrapMethodRead>>
) -> Result<C> {
	let interests = code_visitor.interests();

	let max_stack = reader.read_u16()?;
	let max_locals = reader.read_u16()?;
	code_visitor.visit_max_stack_and_max_locals(max_stack, max_locals)?;

	let code_length = reader.read_u32()?;

	// This limit here is defined by the Java Virtual Machine Specification, and this allows us to store label offsets in an u16.
	if code_length == 0 || code_length > u16::MAX as u32 {
		bail!("`code_length` must be greater than zero and less than 65536, got {code_length:?}");
	}
	let code_length = code_length as u16; // can't fail, see checks above

	let mut labels = Labels::new(code_length);

	let bytecode = reader.read_u8_vec(code_length as usize)?;

	// Create all the labels referenced by any branching instruction.
	{
		// We do this so that we can't read more than the bytecode
		let mut r = Cursor::new(&bytecode);
		while !r.get_ref()[(r.position() as usize)..].is_empty() {
			// We may cast this to an u16, since we checked above that the length of the bytecode is less than 65536.
			// Note that the value of u16::MAX = 65535 is not even possible as a value here.
			let opcode_pos = r.position() as u16;

			(|| { // TODO: Make use of a try-block once it's stable
				match r.read_u8()? {
					opcode::NOP..=opcode::DCONST_1 |
					opcode::ILOAD_0..=opcode::SALOAD |
					opcode::ISTORE_0..=opcode::LXOR |
					opcode::I2L..=opcode::DCMPG |
					opcode::IRETURN..=opcode::RETURN |
					opcode::ARRAYLENGHT |
					opcode::ATHROW |
					opcode::MONITORENTER |
					opcode::MONITOREXIT => {
						// no op
					},
					opcode::BIPUSH |
					opcode::LDC |
					opcode::ILOAD..=opcode::ALOAD |
					opcode::ISTORE..=opcode::ASTORE |
					opcode::RET |
					opcode::NEWARRAY => {
						r.skip(1)?;
					},
					opcode::SIPUSH |
					opcode::LDC_W |
					opcode::LDC2_W |
					opcode::IINC |
					opcode::GETSTATIC..=opcode::INVOKESTATIC |
					opcode::NEW |
					opcode::ANEWARRAY |
					opcode::CHECKCAST |
					opcode::INSTANCEOF => {
						r.skip(2)?;
					},
					opcode::MULTIANEWARRAY => {
						r.skip(3)?;
					},
					opcode::INVOKEINTERFACE |
					opcode::INVOKEDYNAMIC => {
						r.skip(4)?;
					},
					opcode::WIDE => {
						match r.read_u8()? {
							opcode::ILOAD..=opcode::ALOAD |
							opcode::ISTORE..=opcode::ASTORE |
							opcode::RET => {
								r.skip(2)?;
							},
							opcode::IINC => {
								r.skip(4)?;
							},
							wide_opcode => {
								bail!("unknown wide opcode {wide_opcode:x?}");
							},
						}
					},
					opcode::IFEQ..=opcode::JSR |
					opcode::IFNULL |
					opcode::IFNONNULL => {
						labels.create(r.read_i16_as_branch_target_label(opcode_pos)?)?;
					},
					opcode::GOTO_W |
					opcode::JSR_W => {
						labels.create(r.read_i32_as_branch_target_label(opcode_pos)?)?;
					},
					opcode::TABLESWITCH => {
						align_to_4_byte_boundary(&mut r)?;

						labels.create(r.read_i32_as_branch_target_label(opcode_pos)?)?;

						let low = r.read_i32()?;
						let high = r.read_i32()?;

						if low > high { bail!("in tableswitch `low` must be lower or equal to `high`, it's low={low:?} and high={high:?}"); }

						let n = (high - low + 1) as u32; // always >= 1

						for _ in 0..n {
							labels.create(r.read_i32_as_branch_target_label(opcode_pos)?)?;
						}
					},
					opcode::LOOKUPSWITCH => {
						align_to_4_byte_boundary(&mut r)?;

						labels.create(r.read_i32_as_branch_target_label(opcode_pos)?)?;

						let n = r.read_i32()?;
						if n < 0 { bail!("in lookupswitch the `npairs` must be positive, it's npairs={n:?}"); }
						let n = n as u32;

						for _ in 0..n {
							let _key = r.read_i32()?;

							labels.create(r.read_i32_as_branch_target_label(opcode_pos)?)?;
						}
					},
					opcode::BREAKPOINT => bail!("unknown opcode breakpoint"),
					opcode::IMPDEP1 => bail!("unknown opcode impdep1"),
					opcode::IMPDEP2 => bail!("unknown opcode impdep2"),
					opcode => bail!("unknown opcode {opcode:x?}"),
				};
				Ok(())
			})()
				.with_context(|| anyhow!("at bytecode offset {}", opcode_pos))?;
		}
	}

	let exception_table = reader.read_vec(
		|r| r.read_u16_as_usize(),
		|r| Ok(Exception {
			start: labels.get_or_create(r.read_u16()?)?,
			end: labels.get_or_create(r.read_u16()?)?,
			handler: labels.get_or_create(r.read_u16()?)?,
			catch: pool.get_optional(r.read_u16()?, PoolRead::get_class)?,
		})
	)?;

	let mut stack_map_frame = None;

	let mut line_number_table = None;

	let mut local_variable_table = None;

	let attribute_count = reader.read_u16()?;
	for _ in 0..attribute_count {
		let attribute_name = pool.get_utf8_ref(reader.read_u16()?)?;
		let length = reader.read_u32()?;

		match attribute_name.as_java_str() {
			name if name == attribute::STACK_MAP_TABLE && !interests.stack_map_table => reader.skip(length as i64)?,
			name if name == attribute::STACK_MAP_TABLE => {
				let mut offset = 0;
				let number_of_entries = reader.read_u16_as_usize()?;
				let mut frames = std::collections::VecDeque::with_capacity(number_of_entries);
				for i in 0..number_of_entries {
					fn read_stack_map_frame(reader: &mut impl ClassRead, pool: &PoolRead, labels: &mut Labels) -> Result<(u16, StackMapData)> {
						Ok(match reader.read_u8()? {
							offset_delta @ 0..=63 => (offset_delta as u16, StackMapData::Same),
							frame_type @ 64..=127 => ((frame_type - 64) as u16, StackMapData::SameLocals1StackItem {
								stack: read_verification_type_info(reader, pool, labels)?,
							}),
							frame_type @ 128..=246 => bail!("unknown stack map frame type {frame_type}"),
							247 => (reader.read_u16()?, StackMapData::SameLocals1StackItem {
								stack: read_verification_type_info(reader, pool, labels)?,
							}),
							frame_type @ 248..=250 => (reader.read_u16()?, StackMapData::Chop {
								k: 251 - frame_type,
							}),
							251 => (reader.read_u16()?, StackMapData::Same),
							frame_type @ 252..=254 => {
								let offset_delta = reader.read_u16()?;
								let count = frame_type - 251;
								let locals = reader.read_vec(
									|_| Ok(count as usize),
									|r| read_verification_type_info(r, pool, labels),
								)?;
								(offset_delta, StackMapData::Append { locals })
							},
							255 => {
								let offset_delta = reader.read_u16()?;

								let locals = reader.read_vec(
									|r| r.read_u16_as_usize(),
									|r| read_verification_type_info(r, pool, labels),
								)?;
								let stack = reader.read_vec(
									|r| r.read_u16_as_usize(),
									|r| read_verification_type_info(r, pool, labels),
								)?;
								(offset_delta, StackMapData::Full { locals, stack })
							},
						})
					}

					let (offset_delta, frame_data) = read_stack_map_frame(reader, pool, &mut labels)?;

					offset += offset_delta + (if i == 0 { 0 } else { 1 });

					let label = labels.get_or_create(offset)?;

					frames.push_back((label, frame_data));
				}
				stack_map_frame.insert_if_empty(frames).context("only one StackMapTable attribute is allowed")?;
			},
			name if name == attribute::STACK_MAP && !interests.stack_map_table => reader.skip(length as i64)?, // Skip it as well, it's just "another format" of StackMapFrame
			name if name == attribute::STACK_MAP => {
				// See https://docs.oracle.com/javame/8.0/api/cldc/api/Appendix1-verifier.pdf for a definition of it.
				// Our bytecode length, our maximum number of local variables and our maximum size of the operand stack fit into an `u16`,
				// this means that `uoffset`, `ulocalvar` and `ustack` are all `u2` (for us `u16`).
				let number_of_entries = reader.read_u16_as_usize()?;
				let mut frames = Vec::with_capacity(number_of_entries);
				for _ in 0..number_of_entries {
					let offset = reader.read_u16()?;
					// Turns out to be the same as a StackMapTable frame of type 255.
					let locals = reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| read_verification_type_info(r, pool, &mut labels),
					)?;

					let stack = reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| read_verification_type_info(r, pool, &mut labels),
					)?;

					let frame_data = StackMapData::Full { locals, stack };

					let label = labels.get_or_create(offset)?;

					frames.push((label, frame_data));
				}

				// The format of the StackMap attribute doesn't guarantee ordered elements.
				frames.sort_by_key(|&(label, _)| label);

				// Later on, we want to quickly remove the first elements. A VecDeque is faster for this.
				let frames: std::collections::VecDeque<_> = frames.into();

				stack_map_frame.insert_if_empty(frames).context("only one StackMap attribute is allowed")?;
			},
			name if name == attribute::LINE_NUMBER_TABLE && !interests.line_number_table => reader.skip(length as i64)?,
			name if name == attribute::LINE_NUMBER_TABLE => {
				let table = line_number_table.get_or_insert_with(Vec::new);

				let line_number_table_length = reader.read_u16()?;
				for _ in 0..line_number_table_length {
					let start = labels.get_or_create(reader.read_u16()?)?;
					let line_number = reader.read_u16()?;

					table.push((start, line_number));
				}
			},
			name if name == attribute::LOCAL_VARIABLE_TABLE && !interests.local_variable_table => reader.skip(length as i64)?,
			name if name == attribute::LOCAL_VARIABLE_TABLE => {
				let table = local_variable_table.get_or_insert_with(Vec::new);

				let local_variable_table_length = reader.read_u16()?;
				for _ in 0..local_variable_table_length {
					let start_pc = reader.read_u16()?;
					let length = reader.read_u16()?;
					let range = labels.get_or_create_range(start_pc, length)?;
					let name = LocalVariableName::try_from(pool.get_utf8(reader.read_u16()?)?)?;
					let descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;
					let index = reader.read_u16_as_local_variable()?;
					table.push(Lv {
						range,
						name,
						descriptor: Some(descriptor),
						signature: None,
						index,
					});
				}
			},
			name if name == attribute::LOCAL_VARIABLE_TYPE_TABLE && !interests.local_variable_type_table => reader.skip(length as i64)?,
			name if name == attribute::LOCAL_VARIABLE_TYPE_TABLE => {
				let table = local_variable_table.get_or_insert_with(Vec::new);

				let local_variable_type_table_length = reader.read_u16()?;
				for _ in 0..local_variable_type_table_length {
					let start_pc = reader.read_u16()?;
					let length = reader.read_u16()?;
					let range = labels.get_or_create_range(start_pc, length)?;
					let name = LocalVariableName::try_from(pool.get_utf8(reader.read_u16()?)?)?;
					let signature = FieldSignature::try_from(pool.get_utf8(reader.read_u16()?)?)?;
					let index = reader.read_u16_as_local_variable()?;
					table.push(Lv {
						range,
						name,
						descriptor: None,
						signature: Some(signature),
						index,
					});
				}
			},
			name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS && !interests.runtime_visible_type_annotations => reader.skip(length as i64)?,
			name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS => {
				let (visitor, type_annotations_visitor) = code_visitor.visit_type_annotations(true)?;
				let type_annotations_visitor = read_type_annotations_attribute_code(reader, type_annotations_visitor, pool, &mut labels)?;
				code_visitor = CodeVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
			},
			name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS && !interests.runtime_invisible_type_annotations => reader.skip(length as i64)?,
			name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS => {
				let (visitor, type_annotations_visitor) = code_visitor.visit_type_annotations(false)?;
				let type_annotations_visitor = read_type_annotations_attribute_code(reader, type_annotations_visitor, pool, &mut labels)?;
				code_visitor = CodeVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
			},
			_ if !interests.unknown_attributes => reader.skip(length as i64)?,
			_ => {
				let vec = reader.read_u8_vec(length as usize)?;
				let attribute = UnknownAttributeVisitor::read(attribute_name.clone(), vec, pool)?;
				code_visitor.visit_unknown_attribute(attribute)?;
			},
		}
	}

	// At this point all the labels are stored:
	let labels = labels; // remove the mutability

	// We do this so that we can't read more than the bytecode
	let mut r = Cursor::new(&bytecode);

	while !r.get_ref()[(r.position() as usize)..].is_empty() {
		// See the comment above for why we may do this.
		let opcode_pos = r.position() as u16;

		// TODO: Make use of a try-block once it's stable
		let instruction = (|| Ok(match r.read_u8()? {
			opcode::NOP         => Instruction::Nop,
			opcode::ACONST_NULL => Instruction::AConstNull,
			opcode::ICONST_M1   => Instruction::IConstM1,
			opcode::ICONST_0    => Instruction::IConst0,
			opcode::ICONST_1    => Instruction::IConst1,
			opcode::ICONST_2    => Instruction::IConst2,
			opcode::ICONST_3    => Instruction::IConst3,
			opcode::ICONST_4    => Instruction::IConst4,
			opcode::ICONST_5    => Instruction::IConst5,
			opcode::LCONST_0    => Instruction::LConst0,
			opcode::LCONST_1    => Instruction::LConst1,
			opcode::FCONST_0    => Instruction::FConst0,
			opcode::FCONST_1    => Instruction::FConst1,
			opcode::FCONST_2    => Instruction::FConst2,
			opcode::DCONST_0    => Instruction::DConst0,
			opcode::DCONST_1    => Instruction::DConst1,
			opcode::BIPUSH      => Instruction::BiPush(r.read_i8()?),
			opcode::SIPUSH      => Instruction::SiPush(r.read_i16()?),
			opcode::LDC         => Instruction::Ldc(pool.get_loadable(r.read_u8()? as u16, bootstrap_methods)?),
			opcode::LDC_W       => Instruction::Ldc(pool.get_loadable(r.read_u16()?, bootstrap_methods)?),
			opcode::LDC2_W      => Instruction::Ldc(pool.get_loadable(r.read_u16()?, bootstrap_methods)?),
			opcode::ILOAD       => Instruction::ILoad(r.read_u8_as_local_variable()?),
			opcode::LLOAD       => Instruction::LLoad(r.read_u8_as_local_variable()?),
			opcode::FLOAD       => Instruction::FLoad(r.read_u8_as_local_variable()?),
			opcode::DLOAD       => Instruction::DLoad(r.read_u8_as_local_variable()?),
			opcode::ALOAD       => Instruction::ALoad(r.read_u8_as_local_variable()?),
			opcode @ opcode::ILOAD_0..=opcode::ALOAD_3 => { // 0x1a..=0x2d aka 26..=45
				let shifted = opcode - opcode::ILOAD_0; // 0..=19
				let index = shifted & 0b11; // 0, 1, 2 or 3
				let opcode = opcode::ILOAD + (shifted >> 2); // 21..=25

				let index = LvIndex { index: index as u16 };

				match opcode {
					opcode::ILOAD => Instruction::ILoad(index),
					opcode::LLOAD => Instruction::LLoad(index),
					opcode::FLOAD => Instruction::FLoad(index),
					opcode::DLOAD => Instruction::DLoad(index),
					opcode::ALOAD => Instruction::ALoad(index),
					_ => unreachable!(),
				}
			},
			opcode::IALOAD => Instruction::IALoad,
			opcode::LALOAD => Instruction::LALoad,
			opcode::FALOAD => Instruction::FALoad,
			opcode::DALOAD => Instruction::DALoad,
			opcode::AALOAD => Instruction::AALoad,
			opcode::BALOAD => Instruction::BALoad,
			opcode::CALOAD => Instruction::CALoad,
			opcode::SALOAD => Instruction::SALoad,
			opcode::ISTORE => Instruction::IStore(r.read_u8_as_local_variable()?),
			opcode::LSTORE => Instruction::LStore(r.read_u8_as_local_variable()?),
			opcode::FSTORE => Instruction::FStore(r.read_u8_as_local_variable()?),
			opcode::DSTORE => Instruction::DStore(r.read_u8_as_local_variable()?),
			opcode::ASTORE => Instruction::AStore(r.read_u8_as_local_variable()?),
			opcode @ opcode::ISTORE_0..=opcode::ASTORE_3 => { // 0x3b..=0x4e aka 59..=78
				let shifted = opcode - opcode::ISTORE_0; // 0..=19
				let index = shifted & 0b11; // 0, 1, 2 or 3
				let opcode = opcode::ISTORE + (shifted >> 2);// 54..=58

				let index = LvIndex { index: index as u16 };

				match opcode {
					opcode::ISTORE => Instruction::IStore(index),
					opcode::LSTORE => Instruction::LStore(index),
					opcode::FSTORE => Instruction::FStore(index),
					opcode::DSTORE => Instruction::DStore(index),
					opcode::ASTORE => Instruction::AStore(index),
					_ => unreachable!(),
				}
			},
			opcode::IASTORE => Instruction::IAStore,
			opcode::LASTORE => Instruction::LAStore,
			opcode::FASTORE => Instruction::FAStore,
			opcode::DASTORE => Instruction::DAStore,
			opcode::AASTORE => Instruction::AAStore,
			opcode::BASTORE => Instruction::BAStore,
			opcode::CASTORE => Instruction::CAStore,
			opcode::SASTORE => Instruction::SAStore,
			opcode::POP     => Instruction::Pop,
			opcode::POP2    => Instruction::Pop2,
			opcode::DUP     => Instruction::Dup,
			opcode::DUP_X1  => Instruction::DupX1,
			opcode::DUP_X2  => Instruction::DupX2,
			opcode::DUP2    => Instruction::Dup2,
			opcode::DUP2_X1 => Instruction::Dup2X1,
			opcode::DUP2_X2 => Instruction::Dup2X2,
			opcode::SWAP    => Instruction::Swap,
			opcode::IADD    => Instruction::IAdd,
			opcode::LADD    => Instruction::LAdd,
			opcode::FADD    => Instruction::FAdd,
			opcode::DADD    => Instruction::DAdd,
			opcode::ISUB    => Instruction::ISub,
			opcode::LSUB    => Instruction::LSub,
			opcode::FSUB    => Instruction::FSub,
			opcode::DSUB    => Instruction::DSub,
			opcode::IMUL    => Instruction::IMul,
			opcode::LMUL    => Instruction::LMul,
			opcode::FMUL    => Instruction::FMul,
			opcode::DMUL    => Instruction::DMul,
			opcode::IDIV    => Instruction::IDiv,
			opcode::LDIV    => Instruction::LDiv,
			opcode::FDIV    => Instruction::FDiv,
			opcode::DDIV    => Instruction::DDiv,
			opcode::IREM    => Instruction::IRem,
			opcode::LREM    => Instruction::LRem,
			opcode::FREM    => Instruction::FRem,
			opcode::DREM    => Instruction::DRem,
			opcode::INEG    => Instruction::INeg,
			opcode::LNEG    => Instruction::LNeg,
			opcode::FNEG    => Instruction::FNeg,
			opcode::DNEG    => Instruction::DNeg,
			opcode::ISHL    => Instruction::IShl,
			opcode::LSHL    => Instruction::LShl,
			opcode::ISHR    => Instruction::IShr,
			opcode::LSHR    => Instruction::LShr,
			opcode::IUSHR   => Instruction::IUShr,
			opcode::LUSHR   => Instruction::LUShr,
			opcode::IAND    => Instruction::IAnd,
			opcode::LAND    => Instruction::LAnd,
			opcode::IOR     => Instruction::IOr,
			opcode::LOR     => Instruction::LOr,
			opcode::IXOR    => Instruction::IXor,
			opcode::LXOR    => Instruction::LXor,
			opcode::IINC => {
				let index = r.read_u8_as_local_variable()?;
				let value = r.read_i8()?;
				Instruction::IInc(index, value as i16)
			},
			opcode::I2L   => Instruction::I2L,
			opcode::I2F   => Instruction::I2F,
			opcode::I2D   => Instruction::I2D,
			opcode::L2I   => Instruction::L2I,
			opcode::L2F   => Instruction::L2F,
			opcode::L2D   => Instruction::L2D,
			opcode::F2I   => Instruction::F2I,
			opcode::F2L   => Instruction::F2L,
			opcode::F2D   => Instruction::F2D,
			opcode::D2I   => Instruction::D2I,
			opcode::D2L   => Instruction::D2L,
			opcode::D2F   => Instruction::D2F,
			opcode::I2B   => Instruction::I2B,
			opcode::I2C   => Instruction::I2C,
			opcode::I2S   => Instruction::I2S,
			opcode::LCMP  => Instruction::LCmp,
			opcode::FCMPL => Instruction::FCmpL,
			opcode::FCMPG => Instruction::FCmpG,
			opcode::DCMPL => Instruction::DCmpL,
			opcode::DCMPG => Instruction::DCmpG,
			opcode::IFEQ      => Instruction::IfEq(    labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IFNE      => Instruction::IfNe(    labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IFLT      => Instruction::IfLt(    labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IFGE      => Instruction::IfGe(    labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IFGT      => Instruction::IfGt(    labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IFLE      => Instruction::IfLe(    labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ICMPEQ => Instruction::IfICmpEq(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ICMPNE => Instruction::IfICmpNe(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ICMPLT => Instruction::IfICmpLt(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ICMPGE => Instruction::IfICmpGe(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ICMPGT => Instruction::IfICmpGt(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ICMPLE => Instruction::IfICmpLe(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ACMPEQ => Instruction::IfACmpEq(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IF_ACMPNE => Instruction::IfACmpNe(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::GOTO      => Instruction::Goto(    labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::JSR       => Instruction::Jsr(     labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::RET       => Instruction::Ret(r.read_u8_as_local_variable()?),
			opcode::TABLESWITCH => {
				align_to_4_byte_boundary(&mut r)?;

				let default = labels.try_get(r.read_i32_as_branch_target_label(opcode_pos)?)?;
				let low = r.read_i32()?;
				let high = r.read_i32()?;

				if low > high { bail!("in tableswitch `low` must be lower or equal to `high`, it's low={low:?} and high={high:?}"); }

				let n = (high - low + 1) as u32; // always >= 1

				let mut table = Vec::with_capacity(n as usize);
				for _ in 0..n {
					let entry = labels.try_get(r.read_i32_as_branch_target_label(opcode_pos)?)?;
					table.push(entry);
				}

				Instruction::TableSwitch { default, low, high, table }
			},
			opcode::LOOKUPSWITCH => {
				align_to_4_byte_boundary(&mut r)?;

				let default = labels.try_get(r.read_i32_as_branch_target_label(opcode_pos)?)?;

				let n = r.read_i32()?;
				if n < 0 { bail!("in lookupswitch the `npairs` must be positive, it's npairs={n:?}"); }
				let n = n as u32;

				let mut pairs = Vec::with_capacity(n as usize);
				for _ in 0..n {
					let key = r.read_i32()?;
					let value = labels.try_get(r.read_i32_as_branch_target_label(opcode_pos)?)?;
					pairs.push((key, value));
				}

				Instruction::LookupSwitch { default, pairs }
			},
			opcode::IRETURN => Instruction::IReturn,
			opcode::LRETURN => Instruction::LReturn,
			opcode::FRETURN => Instruction::FReturn,
			opcode::DRETURN => Instruction::DReturn,
			opcode::ARETURN => Instruction::AReturn,
			opcode::RETURN  => Instruction::Return,
			opcode::GETSTATIC => Instruction::GetStatic(pool.get_field_ref(r.read_u16()?)?),
			opcode::PUTSTATIC => Instruction::PutStatic(pool.get_field_ref(r.read_u16()?)?),
			opcode::GETFIELD => Instruction::GetField(pool.get_field_ref(r.read_u16()?)?),
			opcode::PUTFIELD => Instruction::PutField(pool.get_field_ref(r.read_u16()?)?),
			opcode::INVOKEVIRTUAL => Instruction::InvokeVirtual(pool.get_method_ref(r.read_u16()?)?),
			opcode::INVOKESPECIAL => {
				let (method_ref, is_interface) = pool.get_method_ref_or_interface_method_ref(r.read_u16()?)?;
				Instruction::InvokeSpecial(method_ref, is_interface)
			},
			opcode::INVOKESTATIC => {
				let (method_ref, is_interface) = pool.get_method_ref_or_interface_method_ref(r.read_u16()?)?;
				Instruction::InvokeStatic(method_ref, is_interface)
			},
			opcode::INVOKEINTERFACE => {
				let method_ref = pool.get_interface_method_ref(r.read_u16()?)?;
				let _count = r.read_u8()?; // TODO: check the count?
				let _zero = r.read_u8()?;
				Instruction::InvokeInterface(method_ref)
			},
			opcode::INVOKEDYNAMIC => {
				let invoke_dynamic = pool.get_invoke_dynamic(r.read_u16()?, bootstrap_methods)?;
				let _zero = r.read_u8()?;
				let _zero = r.read_u8()?;
				Instruction::InvokeDynamic(invoke_dynamic)
			},
			opcode::NEW          => Instruction::New(pool.get_class(r.read_u16()?)?),
			opcode::NEWARRAY     => Instruction::NewArray(ArrayType::from_atype(r.read_u8()?)?),
			opcode::ANEWARRAY    => Instruction::ANewArray(pool.get_class(r.read_u16()?)?),
			opcode::ARRAYLENGHT  => Instruction::ArrayLength,
			opcode::ATHROW       => Instruction::AThrow,
			opcode::CHECKCAST    => Instruction::CheckCast(pool.get_class(r.read_u16()?)?),
			opcode::INSTANCEOF   => Instruction::InstanceOf(pool.get_class(r.read_u16()?)?),
			opcode::MONITORENTER => Instruction::MonitorEnter,
			opcode::MONITOREXIT  => Instruction::MonitorExit,
			opcode::WIDE => {
				match r.read_u8()? {
					opcode::ILOAD  => Instruction::ILoad( r.read_u16_as_local_variable()?),
					opcode::LLOAD  => Instruction::LLoad( r.read_u16_as_local_variable()?),
					opcode::FLOAD  => Instruction::FLoad( r.read_u16_as_local_variable()?),
					opcode::DLOAD  => Instruction::DLoad( r.read_u16_as_local_variable()?),
					opcode::ALOAD  => Instruction::ALoad( r.read_u16_as_local_variable()?),
					opcode::ISTORE => Instruction::IStore(r.read_u16_as_local_variable()?),
					opcode::LSTORE => Instruction::LStore(r.read_u16_as_local_variable()?),
					opcode::FSTORE => Instruction::FStore(r.read_u16_as_local_variable()?),
					opcode::DSTORE => Instruction::DStore(r.read_u16_as_local_variable()?),
					opcode::ASTORE => Instruction::AStore(r.read_u16_as_local_variable()?),
					opcode::RET    => Instruction::Ret(   r.read_u16_as_local_variable()?),
					opcode::IINC => {
						let index = r.read_u16_as_local_variable()?;
						let value = r.read_i16()?;

						Instruction::IInc(index, value)
					},
					wide_opcode => bail!("unknown wide opcode {wide_opcode:x?}"),
				}
			},
			opcode::MULTIANEWARRAY => Instruction::MultiANewArray(pool.get_class(r.read_u16()?)?, r.read_u8()?),
			opcode::IFNULL    => Instruction::IfNull(   labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::IFNONNULL => Instruction::IfNonNull(labels.try_get(r.read_i16_as_branch_target_label(opcode_pos)?)?),
			opcode::GOTO_W    => Instruction::Goto(     labels.try_get(r.read_i32_as_branch_target_label(opcode_pos)?)?),
			opcode::JSR_W     => Instruction::Jsr(      labels.try_get(r.read_i32_as_branch_target_label(opcode_pos)?)?),

			// Reserved, but may not appear in .class files
			opcode::BREAKPOINT => bail!("unknown opcode breakpoint"),
			opcode::IMPDEP1 => bail!("unknown opcode impdep1"),
			opcode::IMPDEP2 => bail!("unknown opcode impdep2"),

			opcode => bail!("unknown opcode {opcode:x?}"),
		} ))()
			.with_context(|| anyhow!("at bytecode offset {}", opcode_pos))?;

		let label = labels.get(opcode_pos);
		let mut frame = None;
		if let Some(stack_map_frame) = stack_map_frame.as_mut() {
			if let Some(label) = label.as_ref() {
				if stack_map_frame.front().is_some_and(|(frame_label, _)| frame_label == label) {
					let Some((_, frame_)) = stack_map_frame.pop_front() else {
						unreachable!("checked that it's Some above");
					};
					frame = Some(frame_);
				}
			}
		}

		code_visitor.visit_instruction(label, frame, instruction)?;
	}
	if let Some(last_label) = labels.get(bytecode.len() as u16) {
		code_visitor.visit_last_label(last_label)?;
	}

	code_visitor.visit_exception_table(exception_table)?;

	if let Some(table) = line_number_table {
		code_visitor.visit_line_numbers(table)?;
	}

	Ok(code_visitor)
}

fn read_verification_type_info(reader: &mut impl ClassRead, pool: &PoolRead, labels: &mut Labels) -> Result<VerificationTypeInfo> {
	Ok(match reader.read_u8()? {
		0 => VerificationTypeInfo::Top,
		1 => VerificationTypeInfo::Integer,
		2 => VerificationTypeInfo::Float,
		3 => VerificationTypeInfo::Double,
		4 => VerificationTypeInfo::Long,
		5 => VerificationTypeInfo::Null,
		6 => VerificationTypeInfo::UninitializedThis,
		7 => {
			let class = pool.get_class(reader.read_u16()?)?;
			VerificationTypeInfo::Object(class)
		},
		8 => {
			let label = labels.get_or_create(reader.read_u16()?)?;
			VerificationTypeInfo::Uninitialized(label)
		},
		tag => bail!("unknown verification_type_info tag {tag}"),
	})
}

fn read_record_component<C: ClassVisitor>(reader: &mut impl ClassRead, class_visitor: C, pool: &PoolRead) -> Result<C> {
	let name = RecordName::try_from(pool.get_utf8(reader.read_u16()?)?)?;
	let descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;

	match class_visitor.visit_record_component(name, descriptor)? {
		ControlFlow::Continue((visitor, mut record_component_visitor)) => {
			let interests = record_component_visitor.interests();

			let attributes_count = reader.read_u16()?;
			for _ in 0..attributes_count {
				let attribute_name = pool.get_utf8_ref(reader.read_u16()?)?;
				let length = reader.read_u32()?;

				match attribute_name.as_java_str() {
					name if name == attribute::SIGNATURE && !interests.signature => reader.skip(length as i64)?,
					name if name == attribute::SIGNATURE => {
						let signature = FieldSignature::try_from(pool.get_utf8(reader.read_u16()?)?)?;
						record_component_visitor.visit_signature(signature)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS && !interests.runtime_visible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = record_component_visitor.visit_annotations(true)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						record_component_visitor = RecordComponentVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS && !interests.runtime_invisible_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_ANNOTATIONS => {
						let (visitor, annotations_visitor) = record_component_visitor.visit_annotations(false)?;
						let annotations_visitor = read_annotations_attribute(reader, annotations_visitor, pool)?;
						record_component_visitor = RecordComponentVisitor::finish_annotations(visitor, annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS && !interests.runtime_visible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = record_component_visitor.visit_type_annotations(true)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						record_component_visitor = RecordComponentVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS && !interests.runtime_invisible_type_annotations => reader.skip(length as i64)?,
					name if name == attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS => {
						let (visitor, type_annotations_visitor) = record_component_visitor.visit_type_annotations(false)?;
						let type_annotations_visitor = read_type_annotations_attribute(reader, type_annotations_visitor, pool)?;
						record_component_visitor = RecordComponentVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
					},
					_ if !interests.unknown_attributes => reader.skip(length as i64)?,
					_ => {
						let vec = reader.read_u8_vec(length as usize)?;
						let attribute = UnknownAttributeVisitor::read(attribute_name.clone(), vec, pool)?;
						record_component_visitor.visit_unknown_attribute(attribute)?;
					}
				}
			}

			ClassVisitor::finish_record_component(visitor, record_component_visitor)
		},
		ControlFlow::Break(visitor) => {
			skip_attributes(reader)?;
			Ok(visitor)
		},
	}
}

fn read_annotations_attribute<A: AnnotationsVisitor>(reader: &mut impl ClassRead, mut annotations_visitor: A, pool: &PoolRead) -> Result<A> {
	let num_annotations = reader.read_u16()?;
	for _ in 0..num_annotations {
		let annotation_descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;

		let (visitor, named_element_values_visitor) = annotations_visitor.visit_annotation(annotation_descriptor)?;

		let named_element_values_visitor = read_element_values_named(reader, pool, named_element_values_visitor)?;

		annotations_visitor = AnnotationsVisitor::finish_annotation(visitor, named_element_values_visitor)?;
	}

	Ok(annotations_visitor)
}

fn read_element_values_named<A: NamedElementValuesVisitor>(reader: &mut impl ClassRead, pool: &PoolRead, mut outer: A) -> Result<A> {
	for _ in 0..reader.read_u16()? {
		let name = pool.get_utf8(reader.read_u16()?)?;
		match reader.read_u8()? {
			b'B' => {
				let const_value_index = reader.read_u16()?;
				let byte = pool.get_integer_as_byte(const_value_index)?;
				outer.visit(name, Object::Byte(byte))?;
			},
			b'C' => {
				let const_value_index = reader.read_u16()?;
				let char = pool.get_integer_as_char(const_value_index)?;
				outer.visit(name, Object::Char(char))?;
			},
			b'D' => {
				let const_value_index = reader.read_u16()?;
				let double = pool.get_double(const_value_index)?;
				outer.visit(name, Object::Double(double))?;
			},
			b'F' => {
				let const_value_index = reader.read_u16()?;
				let float = pool.get_float(const_value_index)?;
				outer.visit(name, Object::Float(float))?;
			},
			b'I' => {
				let const_value_index = reader.read_u16()?;
				let integer = pool.get_integer(const_value_index)?;
				outer.visit(name, Object::Integer(integer))?;
			},
			b'J' => {
				let const_value_index = reader.read_u16()?;
				let long = pool.get_long(const_value_index)?;
				outer.visit(name, Object::Long(long))?;
			},
			b'S' => {
				let const_value_index = reader.read_u16()?;
				let short = pool.get_integer_as_short(const_value_index)?;
				outer.visit(name, Object::Short(short))?;
			},
			b'Z' => {
				let const_value_index = reader.read_u16()?;
				let boolean = pool.get_integer_as_boolean(const_value_index)?;
				outer.visit(name, Object::Boolean(boolean))?;
			},
			b's' => {
				let const_value_index = reader.read_u16()?;
				let string = pool.get_utf8(const_value_index)?;
				outer.visit(name, Object::String(string))?;
			},
			b'e' => {
				let type_name = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;
				let const_name = pool.get_utf8(reader.read_u16()?)?;
				outer.visit_enum(name, type_name, const_name)?;
			},
			b'c' => {
				let class = ReturnDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;
				outer.visit_class(name, class)?;
			},
			b'@' => {
				let annotation_descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;
				let (visitor, inner) = outer.visit_annotation(name, annotation_descriptor)?;
				let inner = read_element_values_named(reader, pool, inner)?;
				outer = A::finish_annotation(visitor, inner)?;
			},
			b'[' => {
				let (visitor, inner) = outer.visit_array(name)?;
				let inner = read_element_values_unnamed(reader, pool, inner)?;
				outer = A::finish_array(visitor, inner)?;
			},
			tag => bail!("unknown element_value tag {tag:?}"),
		}
	}

	Ok(outer)
}

fn read_element_values_unnamed<A: UnnamedElementValuesVisitor>(reader: &mut impl ClassRead, pool: &PoolRead, mut outer: A) -> Result<A> {
	for _ in 0..reader.read_u16()? {
		outer = read_element_value_unnamed(reader, pool, outer)?;
	}

	Ok(outer)
}

fn read_element_value_unnamed<A: UnnamedElementValueVisitor>(reader: &mut impl ClassRead, pool: &PoolRead, mut outer: A) -> Result<A> {
	match reader.read_u8()? {
		b'B' => {
			let const_value_index = reader.read_u16()?;
			let byte = pool.get_integer_as_byte(const_value_index)?;
			outer.visit(Object::Byte(byte))?;
		},
		b'C' => {
			let const_value_index = reader.read_u16()?;
			let char = pool.get_integer_as_char(const_value_index)?;
			outer.visit(Object::Char(char))?;
		},
		b'D' => {
			let const_value_index = reader.read_u16()?;
			let double = pool.get_double(const_value_index)?;
			outer.visit(Object::Double(double))?;
		},
		b'F' => {
			let const_value_index = reader.read_u16()?;
			let float = pool.get_float(const_value_index)?;
			outer.visit(Object::Float(float))?;
		},
		b'I' => {
			let const_value_index = reader.read_u16()?;
			let integer = pool.get_integer(const_value_index)?;
			outer.visit(Object::Integer(integer))?;
		},
		b'J' => {
			let const_value_index = reader.read_u16()?;
			let long = pool.get_long(const_value_index)?;
			outer.visit(Object::Long(long))?;
		},
		b'S' => {
			let const_value_index = reader.read_u16()?;
			let short = pool.get_integer_as_short(const_value_index)?;
			outer.visit(Object::Short(short))?;
		},
		b'Z' => {
			let const_value_index = reader.read_u16()?;
			let boolean = pool.get_integer_as_boolean(const_value_index)?;
			outer.visit(Object::Boolean(boolean))?;
		},
		b's' => {
			let const_value_index = reader.read_u16()?;
			let string = pool.get_utf8(const_value_index)?;
			outer.visit(Object::String(string))?;
		},
		b'e' => {
			let type_name = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;
			let const_name = pool.get_utf8(reader.read_u16()?)?;
			outer.visit_enum(type_name, const_name)?;
		},
		b'c' => {
			let class = ReturnDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;
			outer.visit_class(class)?;
		},
		b'@' => {
			let annotation_descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;
			let (visitor, inner) = outer.visit_annotation(annotation_descriptor)?;
			let inner = read_element_values_named(reader, pool, inner)?;
			outer = A::finish_annotation(visitor, inner)?;
		},
		b'[' => {
			let (visitor, inner) = outer.visit_array()?;
			let inner = read_element_values_unnamed(reader, pool, inner)?;
			outer = A::finish_array(visitor, inner)?;
		},
		tag => bail!("unknown `element_value` tag {tag:?}"),
	}

	Ok(outer)
}

fn read_type_annotations_attribute<A: TypeAnnotationsVisitor<T>, T: TargetInfoRead>(
	reader: &mut impl ClassRead,
	mut type_annotations_visitor: A,
	pool: &PoolRead
) -> Result<A> {
	let num_annotations = reader.read_u16()?;
	for _ in 0..num_annotations {
		let type_reference = TargetInfoRead::read_type_reference(reader)?;
		let type_path = read_type_path(reader)?;
		let annotation_descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;

		let (visitor, named_element_values_visitor) = type_annotations_visitor.visit_type_annotation(type_reference, type_path, annotation_descriptor)?;

		let named_element_values_visitor = read_element_values_named(reader, pool, named_element_values_visitor)?;

		type_annotations_visitor = TypeAnnotationsVisitor::finish_type_annotation(visitor, named_element_values_visitor)?;
	}

	Ok(type_annotations_visitor)
}

fn read_type_annotations_attribute_code<A: TypeAnnotationsVisitor<TargetInfoCode>>(
	reader: &mut impl ClassRead,
	mut type_annotations_visitor: A,
	pool: &PoolRead,
	labels: &mut Labels,
) -> Result<A> {
	let num_annotations = reader.read_u16()?;
	for _ in 0..num_annotations {
		let type_reference = read_type_reference_code(reader, labels)?;
		let type_path = read_type_path(reader)?;
		let annotation_descriptor = FieldDescriptor::try_from(pool.get_utf8(reader.read_u16()?)?)?;

		let (visitor, named_element_values_visitor) = type_annotations_visitor.visit_type_annotation(type_reference, type_path, annotation_descriptor)?;

		let named_element_values_visitor = read_element_values_named(reader, pool, named_element_values_visitor)?;

		type_annotations_visitor = TypeAnnotationsVisitor::finish_type_annotation(visitor, named_element_values_visitor)?;
	}

	Ok(type_annotations_visitor)
}

trait TargetInfoRead: Sized {
	fn read_type_reference(reader: &mut impl ClassRead) -> Result<Self>;
}

impl TargetInfoRead for TargetInfoClass {
	fn read_type_reference(reader: &mut impl ClassRead) -> Result<Self> {
		Ok(match reader.read_u8()? {
			type_annotation::CLASS_TYPE_PARAMETER => TargetInfoClass::ClassTypeParameter { index: reader.read_u8()? },
			type_annotation::CLASS_EXTENDS => {
				let index = reader.read_u16()?;
				if index == u16::MAX {
					TargetInfoClass::Extends
				} else {
					TargetInfoClass::Implements { index }
				}
			},
			type_annotation::CLASS_TYPE_PARAMETER_BOUND => {
				let type_parameter_index = reader.read_u8()?;
				let bound_index = reader.read_u8()?;
				TargetInfoClass::ClassTypeParameterBound { type_parameter_index, bound_index }
			},
			tag => bail!("unknown type reference {tag} for class"),
		})
	}
}

impl TargetInfoRead for TargetInfoField {
	fn read_type_reference(reader: &mut impl ClassRead) -> Result<Self> {
		Ok(match reader.read_u8()? {
			type_annotation::FIELD => TargetInfoField::Field,
			tag => bail!("unknown type reference {tag} for field"),
		})
	}
}

impl TargetInfoRead for TargetInfoMethod {
	fn read_type_reference(reader: &mut impl ClassRead) -> Result<Self> {
		Ok(match reader.read_u8()? {
			type_annotation::METHOD_TYPE_PARAMETER => TargetInfoMethod::MethodTypeParameter { index: reader.read_u8()? },
			type_annotation::METHOD_TYPE_PARAMETER_BOUND => {
				let type_parameter_index = reader.read_u8()?;
				let bound_index = reader.read_u8()?;
				TargetInfoMethod::MethodTypeParameterBound { type_parameter_index, bound_index }
			},
			type_annotation::METHOD_RETURN => TargetInfoMethod::Return,
			type_annotation::METHOD_RECEIVER => TargetInfoMethod::Receiver,
			type_annotation::METHOD_FORMAL_PARAMETER => {
				TargetInfoMethod::FormalParameter { index: reader.read_u8()? }
			},
			type_annotation::THROWS => { // on method
				TargetInfoMethod::Throws { index: reader.read_u16()? }
			},
			tag => bail!("unknown type reference {tag} for method"),
		})
	}
}

fn read_type_reference_code(reader: &mut impl ClassRead, labels: &mut Labels) -> Result<TargetInfoCode> {
	Ok(match reader.read_u8()? {
		type_annotation::LOCAL_VARIABLE => {
			let mut table = Vec::new();
			let length = reader.read_u16()?;
			for _ in 0..length {
				let start_pc = reader.read_u16()?;
				let length = reader.read_u16()?;
				let range = labels.get_or_create_range(start_pc, length)?;
				let index = reader.read_u16_as_local_variable()?;
				table.push((range, index));
			}
			TargetInfoCode::LocalVariable { table }
		},
		type_annotation::RESOURCE_VARIABLE => {
			let mut table = Vec::new();
			for _ in 0..reader.read_u16()? {
				let start_pc = reader.read_u16()?;
				let length = reader.read_u16()?;
				let range = labels.get_or_create_range(start_pc, length)?;
				let index = reader.read_u16_as_local_variable()?;
				table.push((range, index));
			}
			TargetInfoCode::ResourceVariable { table }
		},
		type_annotation::EXCEPTION_PARAMETER => TargetInfoCode::ExceptionParameter { index: reader.read_u16()? },
		type_annotation::INSTANCE_OF => TargetInfoCode::InstanceOf(labels.get_or_create(reader.read_u16()?)?),
		type_annotation::NEW => TargetInfoCode::New(labels.get_or_create(reader.read_u16()?)?),
		type_annotation::CONSTRUCTOR_REFERENCE => TargetInfoCode::ConstructorReference(labels.get_or_create(reader.read_u16()?)?),
		type_annotation::METHOD_REFERENCE => TargetInfoCode::MethodReference(labels.get_or_create(reader.read_u16()?)?),
		type_annotation::CAST => {
			let label = labels.get_or_create(reader.read_u16()?)?;
			let index = reader.read_u8()?;
			TargetInfoCode::Cast { label, index }
		},
		type_annotation::CONSTRUCTOR_INVOCATION_TYPE_ARGUMENT => {
			let label = labels.get_or_create(reader.read_u16()?)?;
			let index = reader.read_u8()?;
			TargetInfoCode::ConstructorInvocationTypeArgument { label, index }
		},
		type_annotation::METHOD_INVOCATION_TYPE_ARGUMENT => {
			let label = labels.get_or_create(reader.read_u16()?)?;
			let index = reader.read_u8()?;
			TargetInfoCode::MethodInvocationTypeArgument { label, index }
		},
		type_annotation::CONSTRUCTOR_REFERENCE_TYPE_ARGUMENT => {
			let label = labels.get_or_create(reader.read_u16()?)?;
			let index = reader.read_u8()?;
			TargetInfoCode::ConstructorReferenceTypeArgument { label, index }
		},
		type_annotation::METHOD_REFERENCE_TYPE_ARGUMENT => {
			let label = labels.get_or_create(reader.read_u16()?)?;
			let index = reader.read_u8()?;
			TargetInfoCode::MethodReferenceTypeArgument { label, index }
		},
		tag => bail!("unknown type reference {tag} for inside of `Code` attribute"),
	})
}

fn read_type_path(reader: &mut impl ClassRead) -> Result<TypePath> {
	let mut vec = Vec::new();
	for _ in 0..reader.read_u8()? {
		let type_path_kind = reader.read_u8()?;
		let type_argument_index = reader.read_u8()?;
		let x = match type_path_kind {
			kind @ 0..=2 => {
				let x = match kind {
					0 => TypePathKind::ArrayDeeper,
					1 => TypePathKind::NestedDeeper,
					2 => TypePathKind::WildcardBound,
					_ => unreachable!(),
				};
				if type_argument_index != 0 {
					bail!("for {x:?}, type_argument_index must be zero, got {type_argument_index}");
				}
				x
			},
			3 => TypePathKind::TypeArgument { index: type_argument_index },
			kind => bail!("type_path_kind not in range from 0 to 3, got {kind}"),
		};
		vec.push(x);
	}
	Ok(TypePath { path: vec })
}

fn read_module(reader: &mut impl ClassRead, pool: &PoolRead) -> Result<Module> {
	Ok(Module {
		name: pool.get_module(reader.read_u16()?)?,
		flags: reader.read_u16()?.into(),
		version: pool.get_optional(reader.read_u16()?, PoolRead::get_utf8)?,
		requires: reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| Ok(ModuleRequires {
				name: pool.get_module(r.read_u16()?)?,
				flags: r.read_u16()?.into(),
				version: pool.get_optional(r.read_u16()?, PoolRead::get_utf8)?,
			})
		)?,
		exports: reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| Ok(ModuleExports {
				name: pool.get_package(r.read_u16()?)?,
				flags: r.read_u16()?.into(),
				exports_to: r.read_vec(
					|r| r.read_u16_as_usize(),
					|r| pool.get_module(r.read_u16()?)
				)?,
			})
		)?,
		opens: reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| Ok(ModuleOpens {
				name: pool.get_package(r.read_u16()?)?,
				flags: r.read_u16()?.into(),
				opens_to: r.read_vec(
					|r| r.read_u16_as_usize(),
					|r| pool.get_module(r.read_u16()?)
				)?,
			})
		)?,
		uses: reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| pool.get_class(r.read_u16()?)
		)?,
		provides: reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| Ok(ModuleProvides {
				name: pool.get_class(r.read_u16()?)?,
				provides_with: r.read_vec(
					|r| r.read_u16_as_usize(),
					|r| pool.get_class(r.read_u16()?)
				)?,
			})
		)?,
	})
}
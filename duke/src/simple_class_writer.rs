use std::collections::HashSet;
use anyhow::{anyhow, bail, Context, Result};
use crate::{class_constants, ClassWrite, jstring};
use crate::class_constants::{attribute, opcode, type_annotation};
use crate::simple_class_writer::labels::{Labels};
use crate::simple_class_writer::pool::PoolWrite;
use crate::tree::annotation::{Annotation, ElementValue, ElementValuePair, Object};
use crate::tree::class::ClassFile;
use crate::tree::field::Field;
use crate::tree::method::code::{Code, Instruction, Label, Loadable};
use crate::tree::method::Method;
use crate::tree::module::Module;
use crate::tree::record::RecordComponent;
use crate::tree::type_annotation::{TargetInfoClass, TargetInfoCode, TargetInfoField, TargetInfoMethod, TypeAnnotation, TypePath, TypePathKind};

mod pool;
mod labels;

// TODO: eventually make a "writer" that's like a visitor but keeps internal state to do this writing job without using any tree:: components...

// TODO: take a look at all write_usize_as_X methods uses and check if there's sufficient .context/.with_context on them

fn write_attribute<'a, 'b, F>(writer: &mut impl ClassWrite, pool: &mut PoolWrite<'a>, name: &'b str, f: F) -> Result<()>
where
	'b: 'a,
	F: FnOnce(&mut Vec<u8>, &mut PoolWrite<'a>) -> Result<()>,
{
	let mut buffer = Vec::new();
	f(&mut buffer, pool)?;
	writer.write_u16(pool.put_utf8(name)?)?;
	writer.write_usize_as_u32(buffer.len()).with_context(|| anyhow!("attribute {name:?} is too large"))?;
	writer.write_u8_slice(&buffer)
}

fn write_attribute_fix_length<'a, 'b: 'a>(writer: &mut impl ClassWrite, pool: &mut PoolWrite<'a>, name: &'b str, length: usize) -> Result<()> {
	writer.write_u16(pool.put_utf8(name)?)?;
	writer.write_usize_as_u32(length).with_context(|| anyhow!("attribute {name:?} is too large"))
}

pub(crate) fn write(class_writer: &mut impl ClassWrite, class: &ClassFile) -> Result<()> {
	class_writer.write_u32(class_constants::MAGIC)?;

	class_writer.write_u16(class.version.minor)?;
	class_writer.write_u16(class.version.major)?;

	// The constant pool. Any constant pool item is added to it.
	let mut pool_: PoolWrite = PoolWrite::new();
	let pool = &mut pool_;
	// The buffer for the rest of the class file.
	let mut writer = Vec::new();

	writer.write_u16(class.access.into())?;
	writer.write_u16(pool.put_class(&class.name)?)?;
	writer.write_u16(pool.put_optional(class.super_class.as_ref(), PoolWrite::put_class)?)?;
	writer.write_slice(
		&class.interfaces,
		|w, size| w.write_usize_as_u16(size).with_context(|| anyhow!("failed to write the number of interfaces of class {:?}", class.name)),
		|w, interface| w.write_u16(pool.put_class(interface)?)
	)?;

	// Important Note regarding bootstrap methods:
	// The BootstrapMethods_attribute must be written after all loadable constant pool entries.
	// The attribute is used at the following locations:
	//  - in the arguments for the bootstrap methods,
	//  - for the ConstantValue_attribute of a field, and
	//  - The ldc, ldc_w, ldc2_l and invokedynamic instructions
	// This means we need to first write the fields and methods.
	// We also need to lazily deal with the bootstrap method arguments.
	// The pool stores the bootstrap methods for us.

	writer.write_slice(
		&class.fields,
		|w, size| w.write_usize_as_u16(size).with_context(|| anyhow!("failed to write the number of fields of class {:?}", class.name)),
		|w, field| write_field(w, field, pool)
			.with_context(|| anyhow!("failed to write field of class {:?}", class.name))
	)?;

	writer.write_slice(
		&class.methods,
		|w, size| w.write_usize_as_u16(size).with_context(|| anyhow!("failed to write the number of methods of class {:?}", class.name)),
		|w, method| write_method(w, method, pool)
			.with_context(|| anyhow!("failed to write method of class {:?}", class.name))
	)?;

	// We write the attributes into a buffer and count them.
	let mut attribute_count = 0;
	let mut buffer = Vec::new();

	if class.has_deprecated_attribute {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::DEPRECATED, 0)?;
	}
	if class.has_synthetic_attribute {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SYNTHETIC, 0)?;
	}

	if let Some(inner_classes) = &class.inner_classes {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::INNER_CLASSES, |w, pool| {
			w.write_usize_as_u16(inner_classes.len()).context("too many inner classes")?;
			for inner_class in inner_classes {
				w.write_u16(pool.put_class(&inner_class.inner_class)?)?;
				w.write_u16(pool.put_optional(inner_class.outer_class.as_ref(), PoolWrite::put_class)?)?;
				w.write_u16(pool.put_optional(inner_class.inner_name.as_deref(), PoolWrite::put_utf8)?)?;
				w.write_u16(inner_class.flags.into())?;
			}
			Ok(())
		})?;
	}
	if let Some(enclosing_method) = &class.enclosing_method {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::ENCLOSING_METHOD, 4)?;
		buffer.write_u16(pool.put_class(&enclosing_method.class)?)?;
		buffer.write_u16(pool.put_optional(enclosing_method.method.as_ref(), |pool, x| pool.put_name_and_type(&x.name, &x.desc))?)?;
	}
	if let Some(signature) = &class.signature {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SIGNATURE, 2)?;
		buffer.write_u16(pool.put_utf8(signature.as_str())?)?;
	}

	if let Some(source_file) = &class.source_file {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SOURCE_FILE, 2)?;
		buffer.write_u16(pool.put_utf8(source_file)?)?;
	}
	if let Some(source_debug_extension) = &class.source_debug_extension {
		attribute_count += 1;
		buffer.write_u16(pool.put_utf8(attribute::SOURCE_DEBUG_EXTENSION)?)?;
		let vec = jstring::from_string_to_vec(source_debug_extension);
		buffer.write_usize_as_u32(vec.len()).with_context(|| anyhow!("attribute {:?} is too large", attribute::SOURCE_DEBUG_EXTENSION))?;
		buffer.write_u8_slice(&vec)?;
	}

	if !class.runtime_visible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &class.runtime_visible_annotations)
		})?;
	}
	if !class.runtime_invisible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &class.runtime_invisible_annotations)
		})?;
	}
	if !class.runtime_visible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &class.runtime_visible_type_annotations)
		})?;
	}
	if !class.runtime_invisible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &class.runtime_invisible_type_annotations)
		})?;
	}

	if let Some(module) = &class.module {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::MODULE, |w, pool| {
			write_module(w, pool, module)
		})?;
	}
	if let Some(module_packages) = &class.module_packages {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::MODULE_PACKAGES, |w, pool| {
			w.write_slice(module_packages,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, package| w.write_u16(pool.put_package(package)?)
			)
		})?;
	}
	if let Some(module_main_class) = &class.module_main_class {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::MODULE_MAIN_CLASS, 2)?;
		buffer.write_u16(pool.put_class(module_main_class)?)?;
	}

	if let Some(nest_host_class) = &class.nest_host_class {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::NEST_HOST, 2)?;
		buffer.write_u16(pool.put_class(nest_host_class)?)?;
	}
	if let Some(nest_members) = &class.nest_members {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::NEST_MEMBERS, |w, pool| {
			w.write_usize_as_u16(nest_members.len())?; // TODO: .context,
			// TODO: possibly convert some of the attributes written with write_attribute to write_attribute_fix_length?
			for member in nest_members {
				w.write_u16(pool.put_class(member)?)?;
			}
			Ok(())
		})?;
	}
	if let Some(permitted_subclasses) = &class.permitted_subclasses {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::PERMITTED_SUBCLASSES, |w, pool| {
			w.write_usize_as_u16(permitted_subclasses.len())?; // TODO: .context, + convert this to write_attribute_fix_length?
			for subclass in permitted_subclasses {
				w.write_u16(pool.put_class(subclass)?)?;
			}
			Ok(())
		})?;
	}

	if !class.record_components.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RECORD, |w, pool| {
			w.write_usize_as_u16(class.record_components.len())?; // TODO: .context
			for record_component in &class.record_components {
				write_record_component(w, record_component, pool)?;
			}
			Ok(())
		})?;
	}

	if let Some((bootstrap_methods, _)) = pool.bootstrap_methods.take() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::BOOTSTRAP_METHODS, |w, pool| {
			w.write_usize_as_u16(bootstrap_methods.len())?; // TODO: .context
			for bootstrap_method in bootstrap_methods {
				w.write_u16(pool.put_method_handle(bootstrap_method.handle)?)?;
				w.write_slice(&bootstrap_method.arguments,
					|w, len| w.write_usize_as_u16(len),
					|w, &argument| w.write_u16(argument),
				)?;
			}
			Ok(())
		})?;
	}

	for attribute in &class.attributes {
		attribute_count += 1;
		buffer.write_u16(pool.put_utf8(&attribute.name)?)?;
		buffer.write_usize_as_u32(attribute.bytes.len()).with_context(|| anyhow!("unknown attribute {:?} is too large", attribute.name))?;
		buffer.write_u8_slice(&attribute.bytes)?;
	}

	// Write the attribute count and then put the buffer containing the attributes.
	writer.write_usize_as_u16(attribute_count).context("too many attributes on method")?;
	writer.write_u8_slice(&buffer)?;

	// IMPORTANT: Write the pool as the last thing, as any other writing can add pool entries.
	pool_.write(class_writer)?;
	// The rest of the class file comes after the constant pool.
	class_writer.write_u8_slice(&writer)?;

	Ok(())
}

fn write_field<'a, 'b: 'a>(writer: &mut impl ClassWrite, field: &'b Field, pool: &mut PoolWrite<'a>) -> Result<()> {
	writer.write_u16(field.access.into())?;
	writer.write_u16(pool.put_utf8(field.name.as_str())?)?;
	writer.write_u16(pool.put_utf8(field.descriptor.as_str())?)?;

	// We write the attributes into a buffer and count them.
	let mut attribute_count = 0;
	let mut buffer = Vec::new();

	if field.has_deprecated_attribute {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::DEPRECATED, 0)?;
	}
	if field.has_synthetic_attribute {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SYNTHETIC, 0)?;
	}

	if let Some(constant_value) = &field.constant_value {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::CONSTANT_VALUE, 2)?;
		buffer.write_u16(pool.put_constant_value(constant_value)?)?;
	}
	if let Some(signature) = &field.signature {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SIGNATURE, 2)?;
		buffer.write_u16(pool.put_utf8(signature.as_str())?)?;
	}

	if !field.runtime_visible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &field.runtime_visible_annotations)
		})?;
	}
	if !field.runtime_invisible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &field.runtime_invisible_annotations)
		})?;
	}
	if !field.runtime_visible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &field.runtime_visible_type_annotations)
		})?;
	}
	if !field.runtime_invisible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &field.runtime_invisible_type_annotations)
		})?;
	}

	for attribute in &field.attributes {
		attribute_count += 1;
		buffer.write_u16(pool.put_utf8(&attribute.name)?)?;
		buffer.write_usize_as_u32(attribute.bytes.len()).with_context(|| anyhow!("unknown attribute {:?} is too large", attribute.name))?;
		buffer.write_u8_slice(&attribute.bytes)?;
	}

	// Write the attribute count and then put the buffer containing the attributes.
	writer.write_usize_as_u16(attribute_count).context("too many attributes on field")?;
	writer.write_u8_slice(&buffer)?;

	Ok(())
}

fn write_method<'a, 'b: 'a>(writer: &mut impl ClassWrite, method: &'b Method, pool: &mut PoolWrite<'a>) -> Result<()> {
	writer.write_u16(method.access.into())?;
	writer.write_u16(pool.put_utf8(method.name.as_str())?)?;
	writer.write_u16(pool.put_utf8(method.descriptor.as_str())?)?;

	// We write the attributes into a buffer and count them.
	let mut attribute_count = 0;
	let mut buffer = Vec::new();

	if method.has_deprecated_attribute {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::DEPRECATED, 0)?;
	}
	if method.has_synthetic_attribute {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SYNTHETIC, 0)?;
	}

	if let Some(code) = &method.code {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::CODE, |w, pool| {
			write_code(w, code, pool)
				.with_context(|| anyhow!("failed to write `Code` attribute of method {:?} {:?}", method.name, method.descriptor))
		})?;
	}
	if let Some(exceptions) = &method.exceptions {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::EXCEPTIONS, |w, pool| {
			w.write_slice(exceptions,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, exception| w.write_u16(pool.put_class(exception)?)
			)
		})?;
	}
	if let Some(signature) = &method.signature {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SIGNATURE, 2)?;
		buffer.write_u16(pool.put_utf8(signature.as_str())?)?;
	}

	if !method.runtime_visible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &method.runtime_visible_annotations)
		})?;
	}
	if !method.runtime_invisible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &method.runtime_invisible_annotations)
		})?;
	}
	if !method.runtime_visible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &method.runtime_visible_type_annotations)
		})?;
	}
	if !method.runtime_invisible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &method.runtime_invisible_type_annotations)
		})?;
	}
	// TODO: RuntimeVisibleParameterAnnotations, RuntimeInvisibleParameterAnnotations

	if let Some(annotation_default) = &method.annotation_default {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::ANNOTATION_DEFAULT, |w, pool| {
			write_element_value_unnamed(w, pool, annotation_default)
		})?;
	}
	if let Some(method_parameters) = &method.method_parameters {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::METHOD_PARAMETERS, |w, pool| {
			w.write_slice(method_parameters,
				|w, len| w.write_usize_as_u8(len), // TODO: .context
				|w, parameter| {
					w.write_u16(pool.put_optional(parameter.name.as_ref().map(|x| x.as_str()), PoolWrite::put_utf8)?)?;
					w.write_u16(parameter.flags.into())
				}
			)
		})?;
	}

	for attribute in &method.attributes {
		attribute_count += 1;
		buffer.write_u16(pool.put_utf8(&attribute.name)?)?;
		buffer.write_usize_as_u32(attribute.bytes.len()).with_context(|| anyhow!("unknown attribute {:?} is too large", attribute.name))?;
		buffer.write_u8_slice(&attribute.bytes)?;
	}

	// Write the attribute count and then put the buffer containing the attributes.
	writer.write_usize_as_u16(attribute_count).context("too many attributes on method")?;
	writer.write_u8_slice(&buffer)?;

	Ok(())
}

fn align_to_4_byte_boundary(writer: &mut Vec<u8>) -> Result<()> {
	match writer.len() & 0b11 {
		0 => { Ok(()) },
		1 => { writer.write_u8_slice(&[0, 0, 0]) },
		2 => { writer.write_u8_slice(&[0, 0]) },
		3 => { writer.write_u8_slice(&[0]) },
		_ => unreachable!(),
	}
}

fn compute_signed_offset(opcode_pos: u16, target: u16) -> i32 {
	(target as i32) - (opcode_pos as i32)
}

/// Stores the information necessary for later inserting a [`Label`] as an [`i16`] or [`i32`].
struct UnwrittenLabel<'a> {
	/// The bytecode position of the instruction containing the label.
	///
	/// Based on this we compute the branch offset.
	opcode_pos: u16,
	/// The index of the instruction being written.
	///
	/// This index is in the instruction list input.
	instruction_index: usize,
	/// The label that needs to be written there.
	label: &'a Label,
	/// The position to put the resolved label [`i16`] or [`i32`] at.
	label_write_pos: usize,
	/// If true, use an [`i32`], if false use an [`i16`] for the label.
	wide: bool,
}


#[allow(clippy::too_many_arguments)]
fn if_helper<'a, 'b: 'a>(w: &mut Vec<u8>,
	 labels: &Labels, wide: &HashSet<usize>, unwritten: &mut Vec<UnwrittenLabel<'a>>,
	 opcode_pos: u16, instruction_index: usize,
	 label: &'b Label,
	 opcode: u8, opposite_opcode: u8
) -> Result<()> {
	if let Some(target) = labels.get(label) {
		let branch = compute_signed_offset(opcode_pos, target);

		if let Ok(branch) = i16::try_from(branch) {
			w.write_u8(opcode)?;
			w.write_i16(branch)?;
		} else {
			// +1 for the opcode, +2 for the branch
			let branch = compute_signed_offset(opcode_pos + 1 + 2, target);

			w.write_u8(opposite_opcode)?;
			// target the instruction after the GOTO_W
			// +1 for the opcode, +2 for the this branch, +1 for the GOTO_W opcode, +4 for that branch
			w.write_i16(1 + 2 + 1 + 4)?;
			w.write_u8(opcode::GOTO_W)?;
			w.write_i32(branch)?;
		}
	} else if wide.contains(&instruction_index) {
		unwritten.push(UnwrittenLabel {
			// target the goto_w instruction: +1 for the opposite_opcode, +2 for the branch
			opcode_pos: opcode_pos + 1 + 2,
			instruction_index,
			label,
			// target the branch of the goto_w instruction:
			// +1 for this opposite_opcode, +2 for that branch, +1 for the GOTO_w opcode
			label_write_pos: opcode_pos as usize + 1 + 2 + 1,
			wide: true,
		});

		w.write_u8(opposite_opcode)?;
		// target the instruction after the GOTO_W
		// +1 for the opcode, +2 for the this branch, +1 for the GOTO_W opcode, +4 for that branch
		w.write_i16(1 + 2 + 1 + 4)?;
		w.write_u8(opcode::GOTO_W)?;
		w.write_i32(i32::MAX)?;
	} else {
		unwritten.push(UnwrittenLabel {
			opcode_pos,
			instruction_index,
			label,
			// target the branch:
			// +1 for this opcode
			label_write_pos: opcode_pos as usize + 1,
			wide: false,
		});

		w.write_u8(opcode)?;
		w.write_i16(i16::MAX)?;
	}
	Ok(())
}

#[allow(clippy::too_many_arguments)]
fn goto_helper<'a, 'b: 'a>(w: &mut Vec<u8>,
   labels: &Labels, wide: &HashSet<usize>, unwritten: &mut Vec<UnwrittenLabel<'a>>,
   opcode_pos: u16, instruction_index: usize,
   label: &'b Label,
   opcode: u8, wide_opcode: u8
) -> Result<()> {
	if let Some(target) = labels.get(label) {
		let branch = compute_signed_offset(opcode_pos, target);

		if let Ok(branch) = i16::try_from(branch) {
			w.write_u8(opcode)?;
			w.write_i16(branch)?;
		} else {
			w.write_u8(wide_opcode)?;
			w.write_i32(branch)?;
		}
	} else if wide.contains(&instruction_index) {
		unwritten.push(UnwrittenLabel {
			opcode_pos,
			instruction_index,
			label,
			// target the branch:
			// +1 for this opcode
			label_write_pos: opcode_pos as usize + 1,
			wide: true,
		});

		w.write_u8(wide_opcode)?;
		w.write_i32(i32::MAX)?;
	} else {
		unwritten.push(UnwrittenLabel {
			opcode_pos,
			instruction_index,
			label,
			// target the branch:
			// +1 for this opcode
			label_write_pos: opcode_pos as usize + 1,
			wide: false,
		});

		w.write_u8(opcode)?;
		w.write_i16(i16::MAX)?;
	}

	Ok(())
}

/// Writes a [`Label`] as an [`i32`], storing it in `unwritten` if we don't know have the label resolved yet.
///
/// Note: uses call to `w.len()` to get current position.
fn switch_helper<'a, 'b: 'a>(w: &mut Vec<u8>,
	 labels: &Labels, unwritten: &mut Vec<UnwrittenLabel<'a>>,
	 opcode_pos: u16, instruction_index: usize,
	 label: &'b Label,
) -> Result<()> {
	let branch = if let Some(target) = labels.get(label) {
		compute_signed_offset(opcode_pos, target)
	} else {
		unwritten.push(UnwrittenLabel {
			opcode_pos,
			instruction_index,
			label,
			label_write_pos: w.len(),
			wide: true,
		});

		i32::MAX
	};
	w.write_i32(branch)
}

/// Writes the content of the `Code` attribute to the writer.
///
/// # Branch offset algorithm
///
/// ## Motivation
/// The maximum size of a bytecode array is [`u16::MAX`].
/// Instructions like `goto` or `ifeq` store branch offsets as an [`i16`].
///
/// Now imagine a jump from the front to the end of a very large method (branch offset larger than [`i16::MAX`]).
/// In such a case we can't write the branch offset into an [`i16`], since it doesn't fit.
///
/// There are however two instructions that help in that case: `goto_w` and `jsr_w`, which both hold an [`i32`] as
/// the branch offset.
///
/// In case of an `if` instruction, there's no `_w` type of it. So for `if` instructions with too large branch offset,
/// we need to use a different approach.
///
/// The `lookupswitch` and `tableswitch` instructions are not affected by this at all, since their branch offsets are always
/// stored as an [`i32`].
///
/// ## Approach to `if` instructions with too large branch offset
/// Consider the following bytecode sequence, where `if_x` denotes some `if` instruction.
/// ```txt,ignore
/// L1: ... // sequence A
/// L2: if_x Lx
/// L3: ... // sequence B
/// ```
/// If the offset required for the `Lx` label at bytecode position `L2` doesn't fit into an [`i16`], we fix it by replacing the `if_x`
/// instruction with:
/// ```txt,ignore
/// L1: ...
/// L2: if_not_x L3
///     goto_w Lx
/// L3: ...
/// ```
/// `if_not_x` denotes the instruction that has the branching and not-branching swapped to the `if_x` instruction.
///
/// Note that this replacement is longer than the original, which shifts the sequence B to higher opcode positions. This can, if for example sequence A
/// contains a jump to sequence B (or the other way around), also cause the jumps there to require replacements.
///
/// Therefore we must keep information about where we placed such a replacement, to adjust branch offsets in other places.
///
/// ## Implementation
/// We just try writing out the bytecode. When we encounter a label, we try to resolve it, and write it out, either using
/// the way given above, or just the regular bytecode sequence. Whenever we don't know a label yet, we add the current position
/// to a list of labels to write at the end. We either reserve a normal or a wide space in the bytecode, depending on if we
/// already tried it without using the wide variant.
///
/// We then try this writing out as often as necessary, each time adding in the instruction the branch size exceeded the [`i16`]
/// bounds.
///
fn write_code<'a, 'b: 'a>(writer: &mut impl ClassWrite, code: &'b Code, pool: &mut PoolWrite<'a>) -> Result<()> {
	if let (Some(max_stack), Some(max_locals)) = (code.max_stack, code.max_locals) {
		writer.write_u16(max_stack)?;
		writer.write_u16(max_locals)?;
	} else {
		bail!("no `max_stack` and `max_locals` given");
	}

	// All instruction indices that need to use the "wide" format.
	// Note that instruction indices != bytecode offsets.
	// These are the indices of our input, as these are constant over multiple write attempts.
	let mut wide: HashSet<usize> = HashSet::new();

	// Here we store all the frames encountered.
	let mut frames: Vec<(u16, _)> = Vec::new();
	let mut labels = Labels::new();

	let mut w = Vec::new();

	// Each run here is one attempt.
	'a: loop {
		// The so far unwritten labels, for this attempt.
		let mut unwritten: Vec<UnwrittenLabel> = Vec::new();

		for (instruction_index, instruction) in code.instructions.iter().enumerate() {

			let opcode_pos = u16::try_from(w.len())
				.with_context(|| anyhow!("cannot write code: code size exceeded u16::MAX: {}", w.len()))?;

			labels.add_instruction(instruction_index, opcode_pos);
			if let Some(label) = instruction.label {
				labels.add_opcode_pos_label(label, opcode_pos);
			}

			if let Some(frame) = &instruction.frame {
				frames.push((opcode_pos, frame));
			}

			// TODO: Make use of a try-block once it's stable
			(|| -> Result<()> {
				match &instruction.instruction {
					Instruction::Nop => w.write_u8(opcode::NOP)?,
					Instruction::AConstNull => w.write_u8(opcode::ACONST_NULL)?,
					Instruction::IConstM1 => w.write_u8(opcode::ICONST_M1)?,
					Instruction::IConst0 => w.write_u8(opcode::ICONST_0)?,
					Instruction::IConst1 => w.write_u8(opcode::ICONST_1)?,
					Instruction::IConst2 => w.write_u8(opcode::ICONST_2)?,
					Instruction::IConst3 => w.write_u8(opcode::ICONST_3)?,
					Instruction::IConst4 => w.write_u8(opcode::ICONST_4)?,
					Instruction::IConst5 => w.write_u8(opcode::ICONST_5)?,
					Instruction::LConst0 => w.write_u8(opcode::LCONST_0)?,
					Instruction::LConst1 => w.write_u8(opcode::LCONST_1)?,
					Instruction::FConst0 => w.write_u8(opcode::FCONST_0)?,
					Instruction::FConst1 => w.write_u8(opcode::FCONST_1)?,
					Instruction::FConst2 => w.write_u8(opcode::FCONST_2)?,
					Instruction::DConst0 => w.write_u8(opcode::DCONST_0)?,
					Instruction::DConst1 => w.write_u8(opcode::DCONST_1)?,
					&Instruction::BiPush(byte) => {
						w.write_u8(opcode::BIPUSH)?;
						w.write_i8(byte)?;
					},
					&Instruction::SiPush(short) => {
						w.write_u8(opcode::SIPUSH)?;
						w.write_i16(short)?;
					},
					Instruction::Ldc(loadable) => {
						let is_long_or_double = match loadable {
							Loadable::Double(_) | Loadable::Long(_) => true,
							Loadable::Dynamic(x) => x.descriptor.as_str().starts_with('D') || x.descriptor.as_str().starts_with('J'),
							_ => false,
						};

						let index = pool.put_loadable(loadable)?;
						if is_long_or_double {
							w.write_u8(opcode::LDC2_W)?;
							w.write_u16(index)?;
						} else if let Ok(index) = u8::try_from(index) {
							w.write_u8(opcode::LDC)?;
							w.write_u8(index)?;
						} else {
							w.write_u8(opcode::LDC_W)?;
							w.write_u16(index)?;
						}
					},
					instruction @ (Instruction::ILoad(_) | Instruction::LLoad(_) | Instruction::FLoad(_) | Instruction::DLoad(_) | Instruction::ALoad(_)) => {
						let (opcode, index) = match instruction {
							Instruction::ILoad(index) => (opcode::ILOAD, index),
							Instruction::LLoad(index) => (opcode::LLOAD, index),
							Instruction::FLoad(index) => (opcode::FLOAD, index),
							Instruction::DLoad(index) => (opcode::DLOAD, index),
							Instruction::ALoad(index) => (opcode::ALOAD, index),
							_ => unreachable!(),
						};
						let index = index.index;

						if index < 4 {
							let index = index as u8;
							let opcode = ((opcode - opcode::ILOAD) << 2 | index) + opcode::ILOAD_0;
							w.write_u8(opcode)?;
						} else if let Ok(index) = u8::try_from(index) {
							w.write_u8(opcode)?;
							w.write_u8(index)?;
						} else {
							w.write_u8(opcode::WIDE)?;
							w.write_u8(opcode)?;
							w.write_u16(index)?;
						}
					},
					Instruction::IALoad => w.write_u8(opcode::IALOAD)?,
					Instruction::LALoad => w.write_u8(opcode::LALOAD)?,
					Instruction::FALoad => w.write_u8(opcode::FALOAD)?,
					Instruction::DALoad => w.write_u8(opcode::DALOAD)?,
					Instruction::AALoad => w.write_u8(opcode::AALOAD)?,
					Instruction::BALoad => w.write_u8(opcode::BALOAD)?,
					Instruction::CALoad => w.write_u8(opcode::CALOAD)?,
					Instruction::SALoad => w.write_u8(opcode::SALOAD)?,
					instruction @ (Instruction::IStore(_) | Instruction::LStore(_) | Instruction::FStore(_) | Instruction::DStore(_) | Instruction::AStore(_)) => {
						let (opcode, index) = match instruction {
							Instruction::IStore(index) => (opcode::ISTORE, index),
							Instruction::LStore(index) => (opcode::LSTORE, index),
							Instruction::FStore(index) => (opcode::FSTORE, index),
							Instruction::DStore(index) => (opcode::DSTORE, index),
							Instruction::AStore(index) => (opcode::ASTORE, index),
							_ => unreachable!(),
						};
						let index = index.index;

						if index < 4 {
							let index = index as u8;
							let opcode = ((opcode - opcode::ISTORE) << 2 | index) + opcode::ISTORE_0;
							w.write_u8(opcode)?;
						} else if let Ok(index) = u8::try_from(index) {
							w.write_u8(opcode)?;
							w.write_u8(index)?;
						} else {
							w.write_u8(opcode::WIDE)?;
							w.write_u8(opcode)?;
							w.write_u16(index)?;
						}
					},
					Instruction::IAStore => w.write_u8(opcode::IASTORE)?,
					Instruction::LAStore => w.write_u8(opcode::LASTORE)?,
					Instruction::FAStore => w.write_u8(opcode::FASTORE)?,
					Instruction::DAStore => w.write_u8(opcode::DASTORE)?,
					Instruction::AAStore => w.write_u8(opcode::AASTORE)?,
					Instruction::BAStore => w.write_u8(opcode::BASTORE)?,
					Instruction::CAStore => w.write_u8(opcode::CASTORE)?,
					Instruction::SAStore => w.write_u8(opcode::SASTORE)?,
					Instruction::Pop     => w.write_u8(opcode::POP)?,
					Instruction::Pop2    => w.write_u8(opcode::POP2)?,
					Instruction::Dup     => w.write_u8(opcode::DUP)?,
					Instruction::DupX1   => w.write_u8(opcode::DUP_X1)?,
					Instruction::DupX2   => w.write_u8(opcode::DUP_X2)?,
					Instruction::Dup2    => w.write_u8(opcode::DUP2)?,
					Instruction::Dup2X1  => w.write_u8(opcode::DUP2_X1)?,
					Instruction::Dup2X2  => w.write_u8(opcode::DUP2_X2)?,
					Instruction::Swap    => w.write_u8(opcode::SWAP)?,
					Instruction::IAdd    => w.write_u8(opcode::IADD)?,
					Instruction::LAdd    => w.write_u8(opcode::LADD)?,
					Instruction::FAdd    => w.write_u8(opcode::FADD)?,
					Instruction::DAdd    => w.write_u8(opcode::DADD)?,
					Instruction::ISub    => w.write_u8(opcode::ISUB)?,
					Instruction::LSub    => w.write_u8(opcode::LSUB)?,
					Instruction::FSub    => w.write_u8(opcode::FSUB)?,
					Instruction::DSub    => w.write_u8(opcode::DSUB)?,
					Instruction::IMul    => w.write_u8(opcode::IMUL)?,
					Instruction::LMul    => w.write_u8(opcode::LMUL)?,
					Instruction::FMul    => w.write_u8(opcode::FMUL)?,
					Instruction::DMul    => w.write_u8(opcode::DMUL)?,
					Instruction::IDiv    => w.write_u8(opcode::IDIV)?,
					Instruction::LDiv    => w.write_u8(opcode::LDIV)?,
					Instruction::FDiv    => w.write_u8(opcode::FDIV)?,
					Instruction::DDiv    => w.write_u8(opcode::DDIV)?,
					Instruction::IRem    => w.write_u8(opcode::IREM)?,
					Instruction::LRem    => w.write_u8(opcode::LREM)?,
					Instruction::FRem    => w.write_u8(opcode::FREM)?,
					Instruction::DRem    => w.write_u8(opcode::DREM)?,
					Instruction::INeg    => w.write_u8(opcode::INEG)?,
					Instruction::LNeg    => w.write_u8(opcode::LNEG)?,
					Instruction::FNeg    => w.write_u8(opcode::FNEG)?,
					Instruction::DNeg    => w.write_u8(opcode::DNEG)?,
					Instruction::IShl    => w.write_u8(opcode::ISHL)?,
					Instruction::LShl    => w.write_u8(opcode::LSHL)?,
					Instruction::IShr    => w.write_u8(opcode::ISHR)?,
					Instruction::LShr    => w.write_u8(opcode::LSHR)?,
					Instruction::IUShr   => w.write_u8(opcode::IUSHR)?,
					Instruction::LUShr   => w.write_u8(opcode::LUSHR)?,
					Instruction::IAnd    => w.write_u8(opcode::IAND)?,
					Instruction::LAnd    => w.write_u8(opcode::LAND)?,
					Instruction::IOr     => w.write_u8(opcode::IOR)?,
					Instruction::LOr     => w.write_u8(opcode::LOR)?,
					Instruction::IXor    => w.write_u8(opcode::IXOR)?,
					Instruction::LXor    => w.write_u8(opcode::LXOR)?,
					&Instruction::IInc(index, value) => {
						if let (Ok(index), Ok(value)) = (u8::try_from(index.index), i8::try_from(value)) {
							w.write_u8(opcode::IINC)?;
							w.write_u8(index)?;
							w.write_i8(value)?;
						} else {
							w.write_u8(opcode::WIDE)?;
							w.write_u8(opcode::IINC)?;
							w.write_u16(index.index)?;
							w.write_i16(value)?;
						}
					},
					Instruction::I2L   => w.write_u8(opcode::I2L)?,
					Instruction::I2F   => w.write_u8(opcode::I2F)?,
					Instruction::I2D   => w.write_u8(opcode::I2D)?,
					Instruction::L2I   => w.write_u8(opcode::L2I)?,
					Instruction::L2F   => w.write_u8(opcode::L2F)?,
					Instruction::L2D   => w.write_u8(opcode::L2D)?,
					Instruction::F2I   => w.write_u8(opcode::F2I)?,
					Instruction::F2L   => w.write_u8(opcode::F2L)?,
					Instruction::F2D   => w.write_u8(opcode::F2D)?,
					Instruction::D2I   => w.write_u8(opcode::D2I)?,
					Instruction::D2L   => w.write_u8(opcode::D2L)?,
					Instruction::D2F   => w.write_u8(opcode::D2F)?,
					Instruction::I2B   => w.write_u8(opcode::I2B)?,
					Instruction::I2C   => w.write_u8(opcode::I2C)?,
					Instruction::I2S   => w.write_u8(opcode::I2S)?,
					Instruction::LCmp  => w.write_u8(opcode::LCMP)?,
					Instruction::FCmpL => w.write_u8(opcode::FCMPL)?,
					Instruction::FCmpG => w.write_u8(opcode::FCMPG)?,
					Instruction::DCmpL => w.write_u8(opcode::DCMPL)?,
					Instruction::DCmpG => w.write_u8(opcode::DCMPG)?,
					Instruction::IfEq(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFEQ, opcode::IFNE)?;
					},
					Instruction::IfNe(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFNE, opcode::IFEQ)?;
					},
					Instruction::IfLt(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFLT, opcode::IFGE)?;
					},
					Instruction::IfGe(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFGE, opcode::IFLT)?;
					},
					Instruction::IfGt(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFGT, opcode::IFLE)?;
					},
					Instruction::IfLe(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFLE, opcode::IFGT)?;
					},
					Instruction::IfICmpEq(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ICMPEQ, opcode::IF_ICMPNE)?;
					},
					Instruction::IfICmpNe(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ICMPNE, opcode::IF_ICMPEQ)?;
					},
					Instruction::IfICmpLt(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ICMPLT, opcode::IF_ICMPGE)?;
					},
					Instruction::IfICmpGe(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ICMPGE, opcode::IF_ICMPLT)?;
					},
					Instruction::IfICmpGt(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ICMPGT, opcode::IF_ICMPLE)?;
					},
					Instruction::IfICmpLe(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ICMPLE, opcode::IF_ICMPGT)?;
					},
					Instruction::IfACmpEq(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ACMPEQ, opcode::IF_ACMPNE)?;
					},
					Instruction::IfACmpNe(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IF_ACMPNE, opcode::IF_ACMPEQ)?;
					},
					Instruction::Goto(label) => {
						goto_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::GOTO, opcode::GOTO_W)?;
					},
					Instruction::Jsr(label) => {
						goto_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::JSR, opcode::JSR_W)?;
					},
					&Instruction::Ret(index) => {
						if let Ok(index) = u8::try_from(index.index) {
							w.write_u8(opcode::RET)?;
							w.write_u8(index)?;
						} else {
							w.write_u8(opcode::WIDE)?;
							w.write_u8(opcode::RET)?;
							w.write_u16(index.index)?;
						}
					},
					&Instruction::TableSwitch { ref default, low, high, ref table } => {
						w.write_u8(opcode::TABLESWITCH)?;
						align_to_4_byte_boundary(&mut w)?;

						switch_helper(&mut w, &labels, &mut unwritten, opcode_pos, instruction_index, default)?;

						if low > high { bail!("`low` must be lower or equal to `high`"); }

						let n = (high - low + 1) as usize;
						if table.len() != n {
							bail!("`low` and `high` bounds don't span a rage of the size of the table: table has {}, high and low define {n}", table.len());
						}

						w.write_i32(low)?;
						w.write_i32(high)?;

						for entry in table {
							switch_helper(&mut w, &labels, &mut unwritten, opcode_pos, instruction_index, entry)?;
						}
					},
					Instruction::LookupSwitch { default, pairs } => {
						w.write_u8(opcode::LOOKUPSWITCH)?;
						align_to_4_byte_boundary(&mut w)?;

						// TODO: use the pairs.is_sorted_by_key(|x| x.0) when it's stable
						let sorted = pairs.windows(2).all(|x| x[0].0.partial_cmp(&x[1].0).map_or(false, std::cmp::Ordering::is_le));
						if !sorted {
							bail!("`pairs` must be sorted by key");
						}

						switch_helper(&mut w, &labels, &mut unwritten, opcode_pos, instruction_index, default)?;

						let n = i32::try_from(pairs.len())
							.with_context(|| anyhow!("`npairs` doesn't fit in i32, it's {:?}", pairs.len()))?;
						w.write_i32(n)?;

						for &(key, ref value) in pairs {
							w.write_i32(key)?;
							switch_helper(&mut w, &labels, &mut unwritten, opcode_pos, instruction_index, value)?;
						}
					},
					Instruction::IReturn => w.write_u8(opcode::IRETURN)?,
					Instruction::LReturn => w.write_u8(opcode::LRETURN)?,
					Instruction::FReturn => w.write_u8(opcode::FRETURN)?,
					Instruction::DReturn => w.write_u8(opcode::DRETURN)?,
					Instruction::AReturn => w.write_u8(opcode::ARETURN)?,
					Instruction::Return  => w.write_u8(opcode::RETURN)?,
					Instruction::GetStatic(field_ref) => {
						w.write_u8(opcode::GETSTATIC)?;
						w.write_u16(pool.put_field_ref(field_ref)?)?;
					},
					Instruction::PutStatic(field_ref) => {
						w.write_u8(opcode::PUTSTATIC)?;
						w.write_u16(pool.put_field_ref(field_ref)?)?;
					},
					Instruction::GetField(field_ref) => {
						w.write_u8(opcode::GETFIELD)?;
						w.write_u16(pool.put_field_ref(field_ref)?)?;
					},
					Instruction::PutField(field_ref) => {
						w.write_u8(opcode::PUTFIELD)?;
						w.write_u16(pool.put_field_ref(field_ref)?)?;
					},
					Instruction::InvokeVirtual(method_ref) => {
						w.write_u8(opcode::INVOKEVIRTUAL)?;
						w.write_u16(pool.put_method_ref(method_ref)?)?;
					},
					&Instruction::InvokeSpecial(ref method_ref, is_interface) => {
						w.write_u8(opcode::INVOKESPECIAL)?;
						w.write_u16(pool.put_method_ref_or_interface_method_ref((method_ref, is_interface))?)?;
					},
					&Instruction::InvokeStatic(ref method_ref, is_interface) => {
						w.write_u8(opcode::INVOKESTATIC)?;
						w.write_u16(pool.put_method_ref_or_interface_method_ref((method_ref, is_interface))?)?;
					},
					Instruction::InvokeInterface(method_ref) => {
						w.write_u8(opcode::INVOKEINTERFACE)?;
						w.write_u16(pool.put_interface_method_ref(method_ref)?)?;
						w.write_u8(method_ref.desc.get_arguments_size()?)?;
						w.write_u8(0)?; // zero
					},
					Instruction::InvokeDynamic(invoke_dynamic) => {
						w.write_u8(opcode::INVOKEDYNAMIC)?;
						w.write_u16(pool.put_invoke_dynamic(invoke_dynamic)?)?;
						w.write_u8(0)?; // zero
						w.write_u8(0)?; // zero
					},
					Instruction::New(class) => {
						w.write_u8(opcode::NEW)?;
						w.write_u16(pool.put_class(class)?)?;
					},
					Instruction::NewArray(atype) => {
						w.write_u8(opcode::NEWARRAY)?;
						w.write_u8(atype.to_atype())?;
					},
					Instruction::ANewArray(class) => {
						w.write_u8(opcode::ANEWARRAY)?;
						w.write_u16(pool.put_class(class)?)?;
					},
					Instruction::ArrayLength => w.write_u8(opcode::ARRAYLENGHT)?,
					Instruction::AThrow      => w.write_u8(opcode::ATHROW)?,
					Instruction::CheckCast(class) => {
						w.write_u8(opcode::CHECKCAST)?;
						w.write_u16(pool.put_class(class)?)?;
					},
					Instruction::InstanceOf(class) => {
						w.write_u8(opcode::INSTANCEOF)?;
						w.write_u16(pool.put_class(class)?)?;
					},
					Instruction::MonitorEnter => w.write_u8(opcode::MONITORENTER)?,
					Instruction::MonitorExit  => w.write_u8(opcode::MONITOREXIT)?,
					&Instruction::MultiANewArray(ref class, dimensions) => {
						w.write_u8(opcode::MULTIANEWARRAY)?;
						w.write_u16(pool.put_class(class)?)?;
						w.write_u8(dimensions)?;
					},
					Instruction::IfNull(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFNULL, opcode::IFNONNULL)?;
					},
					Instruction::IfNonNull(label) => {
						if_helper(&mut w, &labels, &wide, &mut unwritten, opcode_pos, instruction_index, label, opcode::IFNONNULL, opcode::IFNULL)?;
					},
				};
				Ok(())
			})()
				.with_context(|| anyhow!("while writing the instruction {instruction:?}"))?;
		}

		if let Some(last_label) = code.last_label {
			labels.add_opcode_pos_label(last_label, w.len() as u16);
		}

		for unwritten in unwritten {
			fn put_i16_at(writer: &mut [u8], pos: usize, value: i16) {
				let [a, b] = value.to_be_bytes();
				writer[pos] = a;
				writer[pos + 1] = b;
			}

			fn put_i32_at(writer: &mut [u8], pos: usize, value: i32) {
				let [a, b, c, d] = value.to_be_bytes();
				writer[pos] = a;
				writer[pos + 1] = b;
				writer[pos + 2] = c;
				writer[pos + 3] = d;
			}

			let target = labels.try_get(unwritten.label).context("no instruction has the label")?;
			let branch = compute_signed_offset(unwritten.opcode_pos, target);

			if unwritten.wide {
				put_i32_at(&mut w, unwritten.label_write_pos, branch);
			} else if let Ok(branch) = i16::try_from(branch) {
				put_i16_at(&mut w, unwritten.label_write_pos, branch);
			} else {
				// The branch bytes don't fit into the space we've reserved for them,
				// try again, with writing this jump with a wide index as well.
				wide.insert(unwritten.instruction_index);

				labels.next_attempt();
				w = Vec::with_capacity(w.len());
				continue 'a;
			}
		}

		break;
	}

	let code_length = w.len() as u32;
	if code_length == 0 || code_length > u16::MAX as u32 {
		bail!("`code_length` must be greater than zero and less than 65536, got {code_length:?}");
	}
	writer.write_u32(code_length)?;
	writer.write_u8_slice(&w)?;

	writer.write_slice(&code.exception_table,
		|w, len| w.write_usize_as_u16(len), // TODO: .context
		|w, exception| {
			w.write_u16(labels.try_get(&exception.start)?)?;
			w.write_u16(labels.try_get(&exception.end)?)?;
			w.write_u16(labels.try_get(&exception.handler)?)?;
			w.write_u16(pool.put_optional(exception.catch.as_ref(), PoolWrite::put_class)?)
		}
	)?;

	// We write the attributes into a buffer and count them.
	let mut attribute_count = 0;
	let mut buffer = Vec::new();

	if !frames.is_empty() {
		// TODO: write stack map table
	}

	if let Some(line_number_table) = &code.line_numbers {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::LINE_NUMBER_TABLE, |w, _| {
			w.write_slice(line_number_table,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, &(ref start, line_number)| {
					w.write_u16(labels.try_get(start)?)?;
					w.write_u16(line_number)
				}
			)
		})?;
	}

	if let Some(local_variables) = &code.local_variables {
		let mut desc = 0usize;
		let mut sign = 0usize;
		for lv in local_variables {
			if lv.descriptor.is_some() { desc += 1 }
			if lv.signature.is_some() { sign += 1 }
		}

		if desc > 0 {
			attribute_count += 1;
			write_attribute(&mut buffer, pool, attribute::LOCAL_VARIABLE_TABLE, |w, pool| {
				w.write_usize_as_u16(desc)?; // TODO: .context
				for lv in local_variables {
					if let Some(descriptor) = &lv.descriptor {
						let (start, length) = labels.try_get_range(&lv.range)?;
						w.write_u16(start)?;
						w.write_u16(length)?;
						w.write_u16(pool.put_utf8(lv.name.as_str())?)?;
						w.write_u16(pool.put_utf8(descriptor.as_str())?)?;
						w.write_u16(lv.index.index)?;
					}
				}
				Ok(())
			})?;
		}
		if sign > 0 {
			attribute_count += 1;
			write_attribute(&mut buffer, pool, attribute::LOCAL_VARIABLE_TYPE_TABLE, |w, pool| {
				w.write_usize_as_u16(sign)?; // TODO: .context
				for lv in local_variables {
					if let Some(signature) = &lv.signature {
						let (start, length) = labels.try_get_range(&lv.range)?;
						w.write_u16(start)?;
						w.write_u16(length)?;
						w.write_u16(pool.put_utf8(lv.name.as_str())?)?;
						w.write_u16(pool.put_utf8(signature.as_str())?)?;
						w.write_u16(lv.index.index)?;
					}
				}
				Ok(())
			})?;
		}
	}

	if !code.runtime_visible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute_code(w, pool, &code.runtime_visible_type_annotations, &labels)
		})?;
	}
	if !code.runtime_invisible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute_code(w, pool, &code.runtime_invisible_type_annotations, &labels)
		})?;
	}

	// Write the attribute count and then put the buffer containing the attributes.
	writer.write_usize_as_u16(attribute_count).context("too many attributes on code")?; // TODO: improved message...
	writer.write_u8_slice(&buffer)?;

	Ok(())
}

fn write_record_component<'a: 'b, 'b>(writer: &mut impl ClassWrite, record_component: &'a RecordComponent, pool: &mut PoolWrite<'b>) -> Result<()> {
	writer.write_u16(pool.put_utf8(record_component.name.as_str())?)?;
	writer.write_u16(pool.put_utf8(record_component.descriptor.as_str())?)?;

	// We write the attributes into a buffer and count them.
	let mut attribute_count = 0;
	let mut buffer = Vec::new();

	if let Some(signature) = &record_component.signature {
		attribute_count += 1;
		write_attribute_fix_length(&mut buffer, pool, attribute::SIGNATURE, 2)?;
		buffer.write_u16(pool.put_utf8(signature.as_str())?)?;
	}

	if !record_component.runtime_visible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &record_component.runtime_visible_annotations)
		})?;
	}
	if !record_component.runtime_invisible_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_ANNOTATIONS, |w, pool| {
			write_annotations_attribute(w, pool, &record_component.runtime_invisible_annotations)
		})?;
	}
	if !record_component.runtime_visible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_VISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &record_component.runtime_visible_type_annotations)
		})?;
	}
	if !record_component.runtime_invisible_type_annotations.is_empty() {
		attribute_count += 1;
		write_attribute(&mut buffer, pool, attribute::RUNTIME_INVISIBLE_TYPE_ANNOTATIONS, |w, pool| {
			write_type_annotations_attribute(w, pool, &record_component.runtime_invisible_type_annotations)
		})?;
	}

	for attribute in &record_component.attributes {
		attribute_count += 1;
		buffer.write_u16(pool.put_utf8(&attribute.name)?)?;
		buffer.write_usize_as_u32(attribute.bytes.len()).with_context(|| anyhow!("unknown attribute {:?} is too large", attribute.name))?;
		buffer.write_u8_slice(&attribute.bytes)?;
	}

	// Write the attribute count and then put the buffer containing the attributes.
	writer.write_usize_as_u16(attribute_count).context("too many attributes on record component")?; // TODO: improved message...
	writer.write_u8_slice(&buffer)?;

	Ok(())
}

fn write_annotations_attribute<'a: 'b, 'b>(writer: &mut impl ClassWrite, pool: &mut PoolWrite<'b>, annotations: &'a Vec<Annotation>) -> Result<()> {
	writer.write_usize_as_u16(annotations.len())?; // TODO: .context()

	for annotation in annotations {
		writer.write_u16(pool.put_utf8(annotation.annotation_type.as_str())?)?;

		write_element_values_named(writer, pool, &annotation.element_value_pairs)?;
	}

	Ok(())
}

fn write_element_values_named<'a: 'b, 'b>(writer: &mut impl ClassWrite, pool: &mut PoolWrite<'b>, pairs: &'a Vec<ElementValuePair>) -> Result<()> {
	writer.write_usize_as_u16(pairs.len())?; // TODO: .context()

	for pair in pairs {
		writer.write_u16(pool.put_utf8(&pair.name)?)?;
		write_element_value_unnamed(writer, pool, &pair.value)?;
	}

	Ok(())
}

fn write_element_values_unnamed<'a: 'b, 'b>(writer: &mut impl ClassWrite, pool: &mut PoolWrite<'b>, element_values: &'a Vec<ElementValue>) -> Result<()> {
	writer.write_usize_as_u16(element_values.len())?; // TODO: .context()

	for value in element_values {
		write_element_value_unnamed(writer, pool, value)?;
	}

	Ok(())
}

fn write_element_value_unnamed<'a: 'b, 'b>(writer: &mut impl ClassWrite, pool: &mut PoolWrite<'b>, value: &'a ElementValue) -> Result<()> {
	match value {
		ElementValue::Object(object) => {
			match object {
				&Object::Byte(byte) => {
					writer.write_u8(b'B')?;
					writer.write_u16(pool.put_byte_as_integer(byte)?)
				},
				&Object::Char(char) => {
					writer.write_u8(b'C')?;
					writer.write_u16(pool.put_char_as_integer(char)?)
				},
				&Object::Double(double) => {
					writer.write_u8(b'D')?;
					writer.write_u16(pool.put_double(double)?)
				},
				&Object::Float(float) => {
					writer.write_u8(b'F')?;
					writer.write_u16(pool.put_float(float)?)
				},
				&Object::Integer(integer) => {
					writer.write_u8(b'I')?;
					writer.write_u16(pool.put_integer(integer)?)
				},
				&Object::Long(long) => {
					writer.write_u8(b'J')?;
					writer.write_u16(pool.put_long(long)?)
				},
				&Object::Short(short) => {
					writer.write_u8(b'S')?;
					writer.write_u16(pool.put_short_as_integer(short)?)
				},
				&Object::Boolean(boolean) => {
					writer.write_u8(b'Z')?;
					writer.write_u16(pool.put_boolean_as_integer(boolean)?)
				},
				Object::String(string) => {
					writer.write_u8(b's')?;
					writer.write_u16(pool.put_utf8(string)?)
				},
			}
		},
		ElementValue::Enum { type_name, const_name } => {
			writer.write_u8(b'e')?;
			writer.write_u16(pool.put_utf8(type_name.as_str())?)?;
			writer.write_u16(pool.put_utf8(const_name)?)
		},
		ElementValue::Class(class) => {
			writer.write_u8(b'c')?;
			writer.write_u16(pool.put_utf8(class.as_str())?)
		},
		ElementValue::AnnotationInterface(annotation) => {
			writer.write_u8(b'@')?;
			writer.write_u16(pool.put_utf8(annotation.annotation_type.as_str())?)?;

			write_element_values_named(writer, pool, &annotation.element_value_pairs)
		},
		ElementValue::ArrayType(element_values) => {
			writer.write_u8(b'[')?;
			write_element_values_unnamed(writer, pool, element_values)
		},
	}
}

fn write_type_annotations_attribute<'a: 'b, 'b, T: TargetInfoWrite>(
	writer: &mut impl ClassWrite,
	pool: &mut PoolWrite<'b>,
	type_annotations: &'a Vec<TypeAnnotation<T>>
) -> Result<()> {
	writer.write_usize_as_u16(type_annotations.len())?; // TODO: .context()

	for type_annotation in type_annotations {
		TargetInfoWrite::write_type_reference(writer, &type_annotation.type_reference)?;
		write_type_path(writer, &type_annotation.type_path)?;
		writer.write_u16(pool.put_utf8(type_annotation.annotation.annotation_type.as_str())?)?;

		write_element_values_named(writer, pool, &type_annotation.annotation.element_value_pairs)?;
	}

	Ok(())
}

fn write_type_annotations_attribute_code<'a: 'b, 'b>(
	writer: &mut impl ClassWrite,
	pool: &mut PoolWrite<'b>,
	type_annotations: &'a Vec<TypeAnnotation<TargetInfoCode>>,
	labels: &Labels
) -> Result<()> {
	writer.write_usize_as_u16(type_annotations.len())?; // TODO: .context()

	for type_annotation in type_annotations {
		write_type_reference_code(writer, &type_annotation.type_reference, labels)?;
		write_type_path(writer, &type_annotation.type_path)?;
		writer.write_u16(pool.put_utf8(type_annotation.annotation.annotation_type.as_str())?)?;

		write_element_values_named(writer, pool, &type_annotation.annotation.element_value_pairs)?;
	}

	Ok(())
}

trait TargetInfoWrite {
	fn write_type_reference(writer: &mut impl ClassWrite, type_reference: &Self) -> Result<()>;
}

impl TargetInfoWrite for TargetInfoClass {
	fn write_type_reference(writer: &mut impl ClassWrite, type_reference: &Self) -> Result<()> {
		match type_reference {
			&TargetInfoClass::ClassTypeParameter { index } => {
				writer.write_u8(type_annotation::CLASS_TYPE_PARAMETER)?;
				writer.write_u8(index)
			},
			TargetInfoClass::Extends => {
				writer.write_u8(type_annotation::CLASS_EXTENDS)?;
				writer.write_u16(u16::MAX)
			},
			&TargetInfoClass::Implements { index } => {
				writer.write_u8(type_annotation::CLASS_EXTENDS)?;
				writer.write_u16(index)
			},
			&TargetInfoClass::ClassTypeParameterBound { type_parameter_index, bound_index } => {
				writer.write_u8(type_annotation::CLASS_TYPE_PARAMETER_BOUND)?;
				writer.write_u8(type_parameter_index)?;
				writer.write_u8(bound_index)
			},
		}
	}
}

impl TargetInfoWrite for TargetInfoField {
	fn write_type_reference(writer: &mut impl ClassWrite, type_reference: &Self) -> Result<()> {
		match type_reference {
			TargetInfoField::Field => {
				writer.write_u8(type_annotation::FIELD)
			},
		}
	}
}

impl TargetInfoWrite for TargetInfoMethod {
	fn write_type_reference(writer: &mut impl ClassWrite, type_reference: &Self) -> Result<()> {
		match type_reference {
			&TargetInfoMethod::MethodTypeParameter { index } => {
				writer.write_u8(type_annotation::METHOD_TYPE_PARAMETER)?;
				writer.write_u8(index)
			},
			&TargetInfoMethod::MethodTypeParameterBound { type_parameter_index, bound_index } => {
				writer.write_u8(type_annotation::METHOD_TYPE_PARAMETER_BOUND)?;
				writer.write_u8(type_parameter_index)?;
				writer.write_u8(bound_index)
			},
			TargetInfoMethod::Return => {
				writer.write_u8(type_annotation::METHOD_RETURN)
			},
			TargetInfoMethod::Receiver => {
				writer.write_u8(type_annotation::METHOD_RECEIVER)
			},
			&TargetInfoMethod::FormalParameter { index } => {
				writer.write_u8(type_annotation::METHOD_FORMAL_PARAMETER)?;
				writer.write_u8(index)
			},
			&TargetInfoMethod::Throws { index } => {
				writer.write_u8(type_annotation::THROWS)?;
				writer.write_u16(index)
			},
		}
	}
}

fn write_type_reference_code(writer: &mut impl ClassWrite, type_reference: &TargetInfoCode, labels: &Labels) -> Result<()> {
	match type_reference {
		TargetInfoCode::LocalVariable { table } => {
			writer.write_u8(type_annotation::LOCAL_VARIABLE)?;
			writer.write_slice(table,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, (range, index)| {
					let (start_pc, length) = labels.try_get_range(range)?;
					w.write_u16(start_pc)?;
					w.write_u16(length)?;
					w.write_u16(index.index)
				}
			)
		},
		TargetInfoCode::ResourceVariable { table } => {
			writer.write_u8(type_annotation::RESOURCE_VARIABLE)?;
			writer.write_slice(table,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, (range, index)| {
					let (start_pc, length) = labels.try_get_range(range)?;
					w.write_u16(start_pc)?;
					w.write_u16(length)?;
					w.write_u16(index.index)
				}
			)
		},
		&TargetInfoCode::ExceptionParameter { index } => {
			writer.write_u8(type_annotation::EXCEPTION_PARAMETER)?;
			writer.write_u16(index)
		},
		TargetInfoCode::InstanceOf(label) => {
			writer.write_u8(type_annotation::INSTANCE_OF)?;
			writer.write_u16(labels.try_get(label)?)
		},
		TargetInfoCode::New(label) => {
			writer.write_u8(type_annotation::NEW)?;
			writer.write_u16(labels.try_get(label)?)
		},
		TargetInfoCode::ConstructorReference(label) => {
			writer.write_u8(type_annotation::CONSTRUCTOR_REFERENCE)?;
			writer.write_u16(labels.try_get(label)?)
		},
		TargetInfoCode::MethodReference(label) => {
			writer.write_u8(type_annotation::METHOD_REFERENCE)?;
			writer.write_u16(labels.try_get(label)?)
		},
		&TargetInfoCode::Cast { ref label, index } => {
			writer.write_u8(type_annotation::CAST)?;
			writer.write_u16(labels.try_get(label)?)?;
			writer.write_u8(index)
		},
		&TargetInfoCode::ConstructorInvocationTypeArgument { ref label, index } => {
			writer.write_u8(type_annotation::CONSTRUCTOR_INVOCATION_TYPE_ARGUMENT)?;
			writer.write_u16(labels.try_get(label)?)?;
			writer.write_u8(index)
		}
		&TargetInfoCode::MethodInvocationTypeArgument { ref label, index } => {
			writer.write_u8(type_annotation::METHOD_INVOCATION_TYPE_ARGUMENT)?;
			writer.write_u16(labels.try_get(label)?)?;
			writer.write_u8(index)
		},
		&TargetInfoCode::ConstructorReferenceTypeArgument { ref label, index } => {
			writer.write_u8(type_annotation::CONSTRUCTOR_REFERENCE_TYPE_ARGUMENT)?;
			writer.write_u16(labels.try_get(label)?)?;
			writer.write_u8(index)
		},
		&TargetInfoCode::MethodReferenceTypeArgument { ref label, index } => {
			writer.write_u8(type_annotation::METHOD_REFERENCE_TYPE_ARGUMENT)?;
			writer.write_u16(labels.try_get(label)?)?;
			writer.write_u8(index)
		},
	}
}

fn write_type_path(writer: &mut impl ClassWrite, type_path: &TypePath) -> Result<()> {
	writer.write_usize_as_u8(type_path.path.len())?; // TODO: .context
	for i in &type_path.path {
		let (type_path_kind, type_argument_index) = match i {
			TypePathKind::ArrayDeeper => (0, 0),
			TypePathKind::NestedDeeper => (1, 0),
			TypePathKind::WildcardBound => (2, 0),
			&TypePathKind::TypeArgument { index } => (3, index),
		};
		writer.write_u8(type_path_kind)?;
		writer.write_u8(type_argument_index)?;
	}
	Ok(())
}

fn write_module<'a, 'b: 'a>(writer: &mut impl ClassWrite, pool: &mut PoolWrite<'a>, module: &'b Module) -> Result<()> {
	writer.write_u16(pool.put_module(&module.name)?)?;
	writer.write_u16(module.flags.into())?;
	writer.write_u16(pool.put_optional(module.version.as_deref(), PoolWrite::put_utf8)?)?;
	writer.write_slice(&module.requires,
		|w, len| w.write_usize_as_u16(len), // TODO: .context
		|w, requires| {
			w.write_u16(pool.put_module(&requires.name)?)?;
			w.write_u16(requires.flags.into())?;
			w.write_u16(pool.put_optional(requires.version.as_deref(), PoolWrite::put_utf8)?)
		}
	)?;
	writer.write_slice(&module.exports,
		|w, len| w.write_usize_as_u16(len), // TODO: .context
		|w, exports| {
			w.write_u16(pool.put_package(&exports.name)?)?;
			w.write_u16(exports.flags.into())?;
			w.write_slice(&exports.exports_to,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, to| w.write_u16(pool.put_module(to)?)
			)
		}
	)?;
	writer.write_slice(&module.opens,
		|w, len| w.write_usize_as_u16(len), // TODO: .context
		|w, opens| {
			w.write_u16(pool.put_package(&opens.name)?)?;
			w.write_u16(opens.flags.into())?;
			w.write_slice(&opens.opens_to,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, to| w.write_u16(pool.put_module(to)?)
			)
		}
	)?;
	writer.write_slice(&module.uses,
		|w, len| w.write_usize_as_u16(len), // TODO: .context
		|w, uses| w.write_u16(pool.put_class(uses)?)
	)?;
	writer.write_slice(&module.provides,
		|w, len| w.write_usize_as_u16(len), // TODO: .context
		|w, provides| {
			w.write_u16(pool.put_class(&provides.name)?)?;
			w.write_slice(&provides.provides_with,
				|w, len| w.write_usize_as_u16(len), // TODO: .context
				|w, provides_with| w.write_u16(pool.put_class(provides_with)?)
			)
		}
	)
}
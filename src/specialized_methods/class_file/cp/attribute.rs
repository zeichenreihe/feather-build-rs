use std::io::Read;

use anyhow::bail;
use anyhow::Result;

use crate::specialized_methods::class_file::access::ParameterAccess;
use crate::specialized_methods::class_file::cp::Pool;
use crate::specialized_methods::class_file::instruction::Instructions;
//use crate::specialized_methods::class_file::instruction::Instructions;
use crate::specialized_methods::class_file::MyRead;

fn check_attribute_length(reader: &mut impl Read, length: u32) -> Result<()> {
	let len = reader.read_u32()?;
	if len == length {
		Ok(())
	} else {
		bail!("attribute has wrong length: expected {length}, got {len}")
	}
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeAttribute {
	pub(crate) max_stack: u16,
	pub(crate) max_locals: u16,
	pub(crate) code: Instructions,
	pub(crate) exception_table: Vec<ExceptionTableEntry>,
	pub(crate) attributes: Vec<AttributeInfo>,
}

impl CodeAttribute {
	fn parse(reader: &mut impl Read, pool: &Pool) -> Result<CodeAttribute> {
		let _attribute_length = reader.read_u32()?;

		let max_stack = reader.read_u16()?;
		let max_locals = reader.read_u16()?;

		let code_bytes = reader.read_vec(
			|r| r.read_u32_as_usize(),
			|r| r.read_u8()
		)?;
		Instructions::parse(&code_bytes[..])?;

		let exception_table = reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| {
				Ok(ExceptionTableEntry {
					start_pc: r.read_u16_as_usize()?,
					end_pc: r.read_u16_as_usize()?,
					handler_pc: r.read_u16_as_usize()?,
					catch_type_index: r.read_u16_as_usize()?,
				})
			}
		)?;

		let attributes = reader.read_vec(
		   |r| r.read_u16_as_usize(),
		   |r| AttributeInfo::parse(r, pool)
		)?;

		Ok(CodeAttribute {
			max_stack,
			max_locals,
			code: Instructions,
			exception_table,
			attributes,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExceptionTableEntry {
	pub(crate) start_pc: usize,
	pub(crate) end_pc: usize,
	pub(crate) handler_pc: usize,
	pub(crate) catch_type_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StackMapTableAttribute {
	pub(crate) entries: Vec<StackMapFrame>,
}
impl StackMapTableAttribute {
	fn parse(reader: &mut impl Read) -> Result<StackMapTableAttribute> {
		let _attribute_length = reader.read_u32()?;

		let mut is_first_explicit_frame = true;
		let mut last_bytecode_position = 0;
		let count = reader.read_u16_as_usize()?;
		let mut entries = Vec::with_capacity(count);
		for _ in 0..count {
			let (frame, new_bytecode_position) = StackMapFrame::parse(reader, last_bytecode_position, is_first_explicit_frame)?;
			entries.push(frame);
			
			is_first_explicit_frame = false;
			last_bytecode_position = new_bytecode_position;
		}

		Ok(StackMapTableAttribute {
			entries,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VerificationTypeInfo {
	Top,
	Integer,
	Float,
	Long,
	Double,
	Null,
	UninitializedThis,
	Object { index: usize },
	Uninitialized { bytecode_offset: usize },
}

impl VerificationTypeInfo {
	fn parse(reader: &mut impl Read) -> Result<VerificationTypeInfo> {
		match reader.read_u8()? {
			0 => Ok(Self::Top),
			1 => Ok(Self::Integer),
			2 => Ok(Self::Float),
			3 => Ok(Self::Double),
			4 => Ok(Self::Long),
			5 => Ok(Self::Null),
			6 => Ok(Self::UninitializedThis),
			7 => Ok(Self::Object { index: reader.read_u16_as_usize()? }),
			8 => Ok(Self::Uninitialized { bytecode_offset: reader.read_u16_as_usize()? }),
			tag => bail!("unknown verification type info tag {tag}"),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StackMapFrame {
	Same {
		bytecode_offset: usize,
	},
	SameLocals1StackItem {
		bytecode_offset: usize,
		stack: VerificationTypeInfo,
	},
	Chop {
		bytecode_offset: usize,
		k: u8,
	},
	Append {
		bytecode_offset: usize,
		locals: Vec<VerificationTypeInfo>,
	},
	Full {
		bytecode_offset: usize,
		locals: Vec<VerificationTypeInfo>,
		stack: Vec<VerificationTypeInfo>,
	}
}

impl StackMapFrame {
	fn parse(reader: &mut impl Read, last_bytecode_position: usize, is_first_explicit_frame: bool) ->
			Result<(StackMapFrame, usize)> {
		let frame_type = reader.read_u8()?;
		
		let delta_to_position = |offset_delta| if is_first_explicit_frame {
			last_bytecode_position + offset_delta
		} else {
			last_bytecode_position + offset_delta + 1
		};

		match frame_type {
			offset_delta @ 0..=63 => {
				let bytecode_offset = delta_to_position(offset_delta as usize);
				Ok((Self::Same { bytecode_offset }, bytecode_offset))
			},
			frame_type @ 64..=127 => {
				let bytecode_offset = delta_to_position(frame_type as usize - 64);
				Ok((
					Self::SameLocals1StackItem {
						bytecode_offset,
						stack: VerificationTypeInfo::parse(reader)?,
					},
					bytecode_offset
				))
			},
			128..=246 => bail!("unknown stack map frame type {frame_type}"),
			247 => {
				let bytecode_offset = delta_to_position(reader.read_u16_as_usize()?);
				Ok((
					Self::SameLocals1StackItem {
						bytecode_offset,
						stack: VerificationTypeInfo::parse(reader)?,
					},
					bytecode_offset
				))
			},
			frame_type @ 248..=250 => {
				let bytecode_offset = delta_to_position(reader.read_u16_as_usize()?);
				Ok((
					Self::Chop {
						bytecode_offset,
						k: 251 - frame_type,
					},
					bytecode_offset
				))
			},
			251 => {
				let bytecode_offset = delta_to_position(reader.read_u16_as_usize()?);
				Ok((Self::Same { bytecode_offset }, bytecode_offset))
			},
			frame_type @ 252..=254 => {
				let bytecode_offset = delta_to_position(reader.read_u16_as_usize()?);
				Ok((
					Self::Append {
						bytecode_offset,
						locals: reader.read_vec(
							|_| Ok::<usize, _>((frame_type - 251) as usize),
							|r| VerificationTypeInfo::parse(r)
						)?,
					},
					bytecode_offset
				))
			},
			255 => {
				let bytecode_offset = delta_to_position(reader.read_u16_as_usize()?);
				Ok((
					Self::Full {
						bytecode_offset,
						locals: reader.read_vec(
							|r| r.read_u16_as_usize(),
								|r| VerificationTypeInfo::parse(r)
						)?,
						stack: reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| VerificationTypeInfo::parse(r)
						)?,
					},
					bytecode_offset
				))
			},
		}
	}
	pub(crate) fn get_bytecode_offset(&self) -> usize {
		match self {
			StackMapFrame::Same { bytecode_offset, .. } => *bytecode_offset,
			StackMapFrame::SameLocals1StackItem { bytecode_offset, .. } => *bytecode_offset,
			StackMapFrame::Chop { bytecode_offset, .. } => *bytecode_offset,
			StackMapFrame::Append { bytecode_offset, .. } => *bytecode_offset,
			StackMapFrame::Full { bytecode_offset, .. } => *bytecode_offset,
		}
	}
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InnerClassesAttributeClassesElement {
	inner_class_index: usize,
	outer_class_index: usize,
	inner_name_index: usize,
	inner_class_access_flags: u16,
}





#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalVariableTableEntry {
	start_pc: usize,
	length: usize,
	name_index: usize,
	descriptor_index: usize,
	lv_index: u16,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalVariableTypeTableEntry {
	start_pc: usize,
	length: usize,
	name_index: usize,
	signature_index: usize,
	lv_index: usize,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeVisibleAnnotationsAttribute {
}
impl RuntimeVisibleAnnotationsAttribute {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Annotation {
	annotation_type_index: usize,
	element_value_pairs: Vec<AnnotationElementValuePair>,
}
impl Annotation {
	fn parse(reader: &mut impl Read) -> Result<Annotation> {
		Ok(Annotation {
			annotation_type_index: reader.read_u16_as_usize()?,
			element_value_pairs: reader.read_vec(
				|r| r.read_u16_as_usize(),
				|r| AnnotationElementValuePair::parse(r)
			)?,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnnotationElementValuePair {
	element_name_index: usize,
	value: AnnotationElementValue,
}
impl AnnotationElementValuePair {
	fn parse(reader: &mut impl Read) -> Result<AnnotationElementValuePair> {
		Ok(AnnotationElementValuePair {
			element_name_index: reader.read_u16_as_usize()?,
			value: AnnotationElementValue::parse(reader)?,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AnnotationElementValue {
	ConstantValueIndex {
		const_value_index: u16,
	},
	EnumConstValue {
		type_name_index: u16,
		const_name_index: u16,
	},
	ClassInfoIndex {
		class_info_index: u16,
	},
	AnnotationValue {
		annotation_value: Annotation,
	},
	ArrayValue {
		values: Vec<AnnotationElementValue>,
	}
}
impl AnnotationElementValue {
	fn parse(reader: &mut impl Read) -> Result<Self> {
		let tag = reader.read_u8()?;

		Ok(match tag {
			b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's' => Self::ConstantValueIndex {
				const_value_index: reader.read_u16()?,
			},
			b'e' => Self::EnumConstValue {
				type_name_index: reader.read_u16()?,
				const_name_index: reader.read_u16()?,
			},
			b'c' => Self::ClassInfoIndex {
				class_info_index: reader.read_u16()?,
			},
			b'@' => Self::AnnotationValue {
				annotation_value: Annotation::parse(reader)?,
			},
			b'[' => {
				Self::ArrayValue {
					values: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| AnnotationElementValue::parse(r)
					)?,
				}
			},
			tag => bail!("unknown annotation element value tag: {tag}"),
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeInvisibleAnnotationsAttribute {
}
impl RuntimeInvisibleAnnotationsAttribute {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeVisibleParameterAnnotationsAttribute {
}
impl RuntimeVisibleParameterAnnotationsAttribute {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParameterAnnotationPair {
	annotations: Vec<Annotation>,
}
impl ParameterAnnotationPair {
	fn parse(reader: &mut impl Read) -> Result<ParameterAnnotationPair> {
		Ok(ParameterAnnotationPair {
			annotations: reader.read_vec(
				|r| r.read_u16_as_usize(),
				|r| Annotation::parse(r)
			)?,
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeInvisibleParameterAnnotationsAttribute {
}
impl RuntimeInvisibleParameterAnnotationsAttribute {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnnotationDefaultAttribute {
}
impl AnnotationDefaultAttribute {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BootstrapMethodsAttributeEntry {
	boostrap_method_index: usize,
	bootstrap_arguments_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BootstrapMethodArgument {
	index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MethodParameterEntry {
	name_index: usize,
	access_flags: ParameterAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineNumberTableEntry {
	start_pc: usize,
	line_number: usize,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttributeInfo {
	ConstantValue { index: usize },
	Code(CodeAttribute),
	StackMapTable(StackMapTableAttribute),
	Exceptions { exception_table: Vec<usize> },
	InnerClasses { classes: Vec<InnerClassesAttributeClassesElement> },
	EnclosingMethod { class_index: usize, method_index: usize },
	Synthetic,
	Signature { signature_index: usize },
	SourceFile { sourcefile_index: usize },
	SourceDebugExtension { debug_extension: Vec<u8> },
	LineNumberTable { line_number_table: Vec<LineNumberTableEntry> },
	LocalVariableTable { local_variable_table: Vec<LocalVariableTableEntry> },
	LocalVariableTypeTable { local_variable_type_table: Vec<LocalVariableTypeTableEntry> },
	Deprecated,
	RuntimeVisibleAnnotations { annotations: Vec<Annotation> },
	RuntimeInvisibleAnnotations { annotations: Vec<Annotation> },
	RuntimeVisibleParameterAnnotations { parameter_annotations: Vec<ParameterAnnotationPair> },
	RuntimeInvisibleParameterAnnotations { parameter_annotations: Vec<ParameterAnnotationPair> },
	AnnotationDefault { default_value: AnnotationElementValue },
	BootstrapMethods { bootstrap_methods: Vec<BootstrapMethodsAttributeEntry> },
    MethodParameters { parameters: Vec<MethodParameterEntry> },
	Unknown {
		name: Vec<u8>,
		info: Vec<u8>,
	},
}

impl AttributeInfo {
	pub(crate) fn parse<'a, R: Read>(reader: &mut R, pool: &Pool) -> Result<Self> {
		Ok(match pool.get_utf8_info(reader.read_u16_as_usize()?)?.as_slice() {
			b"ConstantValue" => {
				check_attribute_length(reader, 2)?;
				Self::ConstantValue {
					index: reader.read_u16_as_usize()?
				}
			},
			b"Code" => Self::Code(CodeAttribute::parse(reader, pool)?),
			b"StackMapTable" => Self::StackMapTable(StackMapTableAttribute::parse(reader)?),
			b"Exceptions" => {
				let _attribute_length = reader.read_u32()?;
				Self::Exceptions {
					exception_table: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| r.read_u16_as_usize()
					)?,
				}
			},
			b"InnerClasses" => {
				let _attribute_length = reader.read_u32()?;
				Self::InnerClasses {
					classes: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| Ok(InnerClassesAttributeClassesElement {
							inner_class_index: r.read_u16_as_usize()?,
							outer_class_index: r.read_u16_as_usize()?,
							inner_name_index: r.read_u16_as_usize()?,
							inner_class_access_flags: r.read_u16()?,
						})
					)?,
				}
			},
			b"EnclosingMethod" => {
				check_attribute_length(reader, 4)?;
				Self::EnclosingMethod {
					class_index: reader.read_u16_as_usize()?,
					method_index: reader.read_u16_as_usize()?,
				}
			},
			b"Synthetic" => {
				check_attribute_length(reader, 0)?;
				Self::Synthetic
			},
			b"Signature" => {
				check_attribute_length(reader, 2)?;
				Self::Signature {
					signature_index: reader.read_u16_as_usize()?,
				}
			},
			b"SourceFile" => {
				check_attribute_length(reader, 2)?;
				Self::SourceFile {
					sourcefile_index: reader.read_u16_as_usize()?,
				}
			},
			b"SourceDebugExtension" => Self::SourceDebugExtension {
				debug_extension: reader.read_vec(
					|r| r.read_u32_as_usize(),
					|r| r.read_u8()
				)?,
			},
			b"LineNumberTable" => {
				let _attribute_length = reader.read_u32()?;
				Self::LineNumberTable {
					line_number_table: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| {
							Ok(LineNumberTableEntry {
								start_pc: r.read_u16_as_usize()?,
								line_number: r.read_u16_as_usize()?,
							})
						}
					)?,
				}
			},
			b"LocalVariableTable" => {
				let _attribute_length = reader.read_u32()?;
				Self::LocalVariableTable {
					local_variable_table: {
						reader.read_vec(
							|r| r.read_u16_as_usize(),
							|r| Ok(LocalVariableTableEntry {
								start_pc: r.read_u16_as_usize()?,
								length: r.read_u16_as_usize()?,
								name_index: r.read_u16_as_usize()?,
								descriptor_index: r.read_u16_as_usize()?,
								lv_index: r.read_u16()?,
							})
						)?
					}
				}
			},
			b"LocalVariableTypeTable" => {
				Self::LocalVariableTypeTable {
					local_variable_type_table: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| Ok(LocalVariableTypeTableEntry {
							start_pc: r.read_u16_as_usize()?,
							length: r.read_u16_as_usize()?,
							name_index: r.read_u16_as_usize()?,
							signature_index: r.read_u16_as_usize()?,
							lv_index: r.read_u16_as_usize()?,
						})
					)?,
				}
			},
			b"Deprecated" => {
				check_attribute_length(reader, 0)?;
				Self::Deprecated
			},
			b"RuntimeVisibleAnnotations" => {
				let _attribute_length = reader.read_u32()?;
				Self::RuntimeVisibleAnnotations {
					annotations: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| Annotation::parse(r),
					)?,
				}
			}
			b"RuntimeInvisibleAnnotations" => {
				let _attribute_length = reader.read_u32()?;
				Self::RuntimeInvisibleAnnotations {
					annotations: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| Annotation::parse(r)
					)?,
				}
			}
			b"RuntimeVisibleParameterAnnotations" => {
				let _attribute_length = reader.read_u32()?;
				Self::RuntimeVisibleParameterAnnotations {
					parameter_annotations: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| ParameterAnnotationPair::parse(r)
					)?,
				}
			}
			b"RuntimeInvisibleParameterAnnotations" => {
				let _attribute_length = reader.read_u32()?;
				Self::RuntimeInvisibleParameterAnnotations {
					parameter_annotations: reader.read_vec(
						|r| r.read_u8_as_usize(),
						|r| ParameterAnnotationPair::parse(r)
					)?,
				}
			}
			b"AnnotationDefault" => {
				let _attribute_length = reader.read_u32()?;
				Self::AnnotationDefault {
					default_value: AnnotationElementValue::parse(reader)?,
				}
			}
			b"BootstrapMethods" => {
				let _attribute_length = reader.read_u32()?;
				Self::BootstrapMethods {
					bootstrap_methods: reader.read_vec(
						|r| r.read_u16_as_usize(),
						|r| Ok(BootstrapMethodsAttributeEntry {
							boostrap_method_index: r.read_u16_as_usize()?,
							bootstrap_arguments_indices: r.read_vec(
								|r| r.read_u16_as_usize(),
								|r| r.read_u16_as_usize()
							)?
						})
					)?,
				}
			}
			b"MethodParameters" => {
				let _attribute_length = reader.read_u32()?;
				Self::MethodParameters {
					parameters: reader.read_vec(
						|r| r.read_u8_as_usize(),
						|r| Ok(MethodParameterEntry {
							name_index: r.read_u16_as_usize()?,
							access_flags: ParameterAccess::parse(r.read_u16()?),
						})
					)?,
				}
			}
			name => {
				let info = reader.read_vec(
					|r| r.read_u32_as_usize(),
					|r| r.read_u8()
				)?;
				eprintln!("WARN: unknown attr: {name:?}: {info:?}");
				Self::Unknown { name: pool.get_utf8_info(reader.read_u16_as_usize()?)?.clone(), info }
			},
		})
	}
}

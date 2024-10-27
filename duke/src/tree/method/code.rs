use std::fmt::{Display, Formatter};
use anyhow::{bail, Result};
use java_string::{JavaStr, JavaString};
use crate::class_constants::atype;
use crate::macros::{make_display, make_string_str_like};
use crate::tree::attribute::Attribute;
use crate::tree::class::ClassName;
use crate::tree::field::{FieldDescriptor, FieldName, FieldRef, FieldSignature};
use crate::tree::method::{MethodDescriptor, MethodName, MethodRef};
use crate::tree::type_annotation::{TargetInfoCode, TypeAnnotation};
use crate::visitor::attribute::UnknownAttributeVisitor;
use crate::visitor::method::code::{CodeVisitor, StackMapData};
use crate::visitor::method::MethodVisitor;

#[derive(Debug, Clone, PartialEq)]
pub struct InstructionListEntry {
	pub label: Option<Label>,
	pub frame: Option<StackMapData>,
	pub instruction: Instruction,
}

/// Represents the code of a method.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Code {
	pub max_stack: Option<u16>,
	pub max_locals: Option<u16>,

	pub instructions: Vec<InstructionListEntry>,
	pub exception_table: Vec<Exception>,
	pub last_label: Option<Label>,

	pub line_numbers: Option<Vec<(Label, u16)>>,
	pub local_variables: Option<Vec<Lv>>,

	pub runtime_visible_type_annotations: Vec<TypeAnnotation<TargetInfoCode>>,
	pub runtime_invisible_type_annotations: Vec<TypeAnnotation<TargetInfoCode>>,

	pub attributes: Vec<Attribute>,
}

impl Code {
	pub(crate) fn accept<M>(self, mut visitor: M) -> Result<M>
	where
		M: MethodVisitor,
	{
		if let Some(mut code_visitor) = visitor.visit_code()? {
			let interests = code_visitor.interests();

			if let (Some(max_stack), Some(max_locals)) = (self.max_stack, self.max_locals) {
				code_visitor.visit_max_stack_and_max_locals(max_stack, max_locals)?;
			}

			// TODO: interests.stack_map_table
			//  also the question: should CodeInterests have a field for "instructions"?
			for instruction in self.instructions {
				code_visitor.visit_instruction(instruction.label, instruction.frame, instruction.instruction)?;
			}
			code_visitor.visit_exception_table(self.exception_table)?;
			if let Some(last_label) = self.last_label {
				code_visitor.visit_last_label(last_label)?;
			}

			if interests.line_number_table {
				if let Some(line_number_table) = self.line_numbers {
					code_visitor.visit_line_numbers(line_number_table)?;
				}
			}
			if interests.local_variable_table || interests.local_variable_type_table {
				if let Some(local_variables) = self.local_variables {
					code_visitor.visit_local_variables(local_variables)?;
				}
			}

			if interests.runtime_visible_type_annotations && !self.runtime_visible_type_annotations.is_empty() {
				let (visitor, mut type_annotations_visitor) = code_visitor.visit_type_annotations(true)?;
				for annotation in self.runtime_visible_type_annotations {
					type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
				}
				code_visitor = CodeVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
			}
			if interests.runtime_invisible_type_annotations && !self.runtime_invisible_type_annotations.is_empty() {
				let (visitor, mut type_annotations_visitor) = code_visitor.visit_type_annotations(false)?;
				for annotation in self.runtime_invisible_type_annotations {
					type_annotations_visitor = annotation.accept(type_annotations_visitor)?;
				}
				code_visitor = CodeVisitor::finish_type_annotations(visitor, type_annotations_visitor)?;
			}

			if interests.unknown_attributes {
				for attribute in self.attributes {
					if let Some(attribute) = UnknownAttributeVisitor::from_attribute(attribute)? {
						code_visitor.visit_unknown_attribute(attribute)?;
					}
				}
			}

			visitor.finish_code(code_visitor)?;
		}
		Ok(visitor)
	}
}

make_string_str_like!(
	pub LocalVariableName(JavaString);
	pub LocalVariableNameSlice(JavaStr);
);
make_display!(LocalVariableName, LocalVariableNameSlice);

impl LocalVariableName {
	fn check_valid(inner: &JavaStr) -> Result<()> {
		if crate::tree::names::is_valid_unqualified_name(inner) {
			Ok(())
		} else {
			bail!("invalid local variable name: must be non-empty and not contain any of `.`, `;`, `[` and `/`")
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exception {
	pub start: Label,
	pub end: Label,
	pub handler: Label,
	pub catch: Option<ClassName>,
}

/// Represents an index of a local variable.
///
/// If the local variable is of type `double` or `long`, it also occupies
/// the [`LvIndex`] with `index = index + 1`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LvIndex {
	pub index: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lv {
	pub range: LabelRange,
	pub name: LocalVariableName,
	pub descriptor: Option<FieldDescriptor>,
	pub signature: Option<FieldSignature>,
	pub index: LvIndex,
}

/// Represents a bytecode offset of an opcode using a method-local id.
///
/// Since the `code` array must have a size that fits in an `u16`, and each bytecode offset can at maximum be an instruction,
/// a label id also fits in an `u16`.
///
/// For example, take this piece of bytecode:
/// ```txt,ignore
/// 0x19 0x03 0xb1
/// ```
/// Javap would output this as (`0x19` is `aload`, and `0xb0` is `return`):
/// ```txt,ignore
/// 0: aload 3
/// 2: return
/// ```
/// Here only `0` and `2` are valid bytecode offsets, `1` would be the offset of the operand of the `aload` instruction.
/// Therefore, there can only be labels for these two offsets.
///
/// Note that the length of the bytecode is also a "valid" bytecode offset (as it's used in various places).
/// See also [`CodeVisitor::visit_last_label`].
///
/// The id stored in the `id` field does **not** correspond to the bytecode offset in any direct way. When reading or writing,
/// that id is used to uniquely identify a bytecode offset.
///
/// Also note that the implementation of [`Eq`] doesn't consider that the structure is important and not the actual value.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label {
	pub(crate) id: u16,
}

/// Represents a range of bytecode offsets.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelRange {
	/// The start label, inclusive.
	pub(crate) start: Label,
	/// The end label, exclusive.
	pub(crate) end: Label,
}

/// Represents an instruction of the JVM.
///
/// Each instruction can either:
/// - hold no additional data, like [`Instruction::Nop`],
/// - hold some immediate value, like [`Instruction::BiPush`],
/// - hold a [local variable index][LvIndex], like [`Instruction::ILoad`] (note that this also represents the `iload_0` instruction for example),
/// - hold a [`Label`] for jumps, like [`Instruction::IfEq`],
/// - or hold other data the instruction needs.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
	Nop,
	AConstNull,
	IConstM1, IConst0, IConst1, IConst2, IConst3, IConst4, IConst5,
	LConst0, LConst1,
	FConst0, FConst1, FConst2,
	DConst0, DConst1,
	BiPush(i8),
	SiPush(i16),
	Ldc(Loadable),
	ILoad(LvIndex), LLoad(LvIndex), FLoad(LvIndex), DLoad(LvIndex), ALoad(LvIndex),
	IALoad, LALoad, FALoad, DALoad, AALoad, BALoad, CALoad, SALoad,
	IStore(LvIndex), LStore(LvIndex), FStore(LvIndex), DStore(LvIndex), AStore(LvIndex),
	IAStore, LAStore, FAStore, DAStore, AAStore, BAStore, CAStore, SAStore,
	Pop, Pop2,
	Dup, DupX1, DupX2,
	Dup2, Dup2X1, Dup2X2,
	Swap,
	IAdd, LAdd, FAdd, DAdd,
	ISub, LSub, FSub, DSub,
	IMul, LMul, FMul, DMul,
	IDiv, LDiv, FDiv, DDiv,
	IRem, LRem, FRem, DRem,
	INeg, LNeg, FNeg, DNeg,
	IShl, LShl,
	IShr, LShr,
	IUShr, LUShr,
	IAnd, LAnd,
	IOr, LOr,
	IXor, LXor,
	IInc(LvIndex, i16),
	I2L, I2F, I2D,
	L2I, L2F, L2D,
	F2I, F2L, F2D,
	D2I, D2L, D2F,
	I2B, I2C, I2S,
	LCmp,
	FCmpL, FCmpG,
	DCmpL, DCmpG,
	IfEq(Label), IfNe(Label), IfLt(Label), IfGe(Label), IfGt(Label), IfLe(Label),
	IfICmpEq(Label), IfICmpNe(Label), IfICmpLt(Label), IfICmpGe(Label), IfICmpGt(Label), IfICmpLe(Label),
	IfACmpEq(Label), IfACmpNe(Label),
	Goto(Label),
	Jsr(Label),
	Ret(LvIndex),
	//TODO: consider putting a Box<...> here [TableSwitch, and LookupSwitch] to make the enum smaller
	TableSwitch {
		default: Label,
		low: i32,
		high: i32,
		table: Vec<Label>,
	},
	LookupSwitch {
		default: Label,
		/// Note that these must be ordered.
		pairs: Vec<(i32, Label)>
	},
	IReturn, LReturn, FReturn, DReturn, AReturn,
	Return,
	GetStatic(FieldRef),
	PutStatic(FieldRef),
	GetField(FieldRef),
	PutField(FieldRef),
	InvokeVirtual(MethodRef),
	/// The bool is `true` iff it's on an interface, so if it referenced an `InterfaceMethodRef` constant pool entry.
	InvokeSpecial(MethodRef, bool),
	/// The bool is `true` iff it's on an interface, so if it referenced an `InterfaceMethodRef` constant pool entry.
	InvokeStatic(MethodRef, bool),
	/// `invokeinterface` also uses an `InterfaceMethodRef` constant pool entry.
	// TODO: better docs here in general
	InvokeInterface(MethodRef),
	InvokeDynamic(InvokeDynamic),
	New(ClassName),
	NewArray(ArrayType),
	ANewArray(ClassName),
	ArrayLength,
	AThrow,
	CheckCast(ClassName),
	InstanceOf(ClassName),
	MonitorEnter, MonitorExit,
	MultiANewArray(ClassName, u8),
	IfNull(Label), IfNonNull(Label),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Loadable {
	Integer(i32),
	Float(f32),
	Long(i64),
	Double(f64),
	Class(ClassName),
	String(JavaString),
	MethodHandle(Handle),
	MethodType(MethodDescriptor),
	Dynamic(ConstantDynamic),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Handle {
	GetField(FieldRef),
	GetStatic(FieldRef),
	PutField(FieldRef),
	PutStatic(FieldRef),
	// TODO: document what is what (which here is interface_method_ref and which is just method_ref)
	InvokeVirtual(MethodRef),
	InvokeStatic(MethodRef, bool),
	InvokeSpecial(MethodRef, bool),
	NewInvokeSpecial(MethodRef),
	InvokeInterface(MethodRef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstantDynamic {
	pub name: FieldName,
	pub descriptor: FieldDescriptor,
	pub handle: Handle,
	pub arguments: Vec<Loadable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvokeDynamic {
	pub name: MethodName,
	pub descriptor: MethodDescriptor,
	pub handle: Handle,
	pub arguments: Vec<Loadable>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ArrayType {
	Boolean,
	Char,
	Float,
	Double,
	Byte,
	Short,
	Int,
	Long,
}

impl ArrayType {
	pub(crate) fn from_atype(atype: u8) -> Result<ArrayType> {
		match atype {
			atype::T_BOOLEAN => Ok(ArrayType::Boolean),
			atype::T_CHAR    => Ok(ArrayType::Char),
			atype::T_FLOAT   => Ok(ArrayType::Float),
			atype::T_DOUBLE  => Ok(ArrayType::Double),
			atype::T_BYTE    => Ok(ArrayType::Byte),
			atype::T_SHORT   => Ok(ArrayType::Short),
			atype::T_INT     => Ok(ArrayType::Int),
			atype::T_LONG    => Ok(ArrayType::Long),
			_ => bail!("unknown array type {atype:x}"),
		}
	}

	pub(crate) fn to_atype(self) -> u8 {
		match self {
			ArrayType::Boolean => atype::T_BOOLEAN,
			ArrayType::Char    => atype::T_CHAR,
			ArrayType::Float   => atype::T_FLOAT,
			ArrayType::Double  => atype::T_DOUBLE,
			ArrayType::Byte    => atype::T_BYTE,
			ArrayType::Short   => atype::T_SHORT,
			ArrayType::Int     => atype::T_INT,
			ArrayType::Long    => atype::T_LONG,
		}
	}
}

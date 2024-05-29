use anyhow::Result;
use crate::tree::class::ClassName;
use crate::tree::method::code::{Exception, Instruction, Label, Lv};
use crate::tree::type_annotation::TargetInfoCode;
use crate::visitor::annotation::TypeAnnotationsVisitor;
use crate::visitor::attribute::UnknownAttributeVisitor;

pub trait CodeVisitor
where
	Self: Sized,
	Self::TypeAnnotationsVisitor: TypeAnnotationsVisitor<TargetInfoCode>,
	Self::UnknownAttribute: UnknownAttributeVisitor,
{
	type TypeAnnotationsVisitor;
	type TypeAnnotationsResidual;
	type UnknownAttribute;

	fn interests(&self) -> CodeInterests;

	fn visit_max_stack_and_max_locals(&mut self, max_stack: u16, max_locals: u16) -> Result<()>;

	fn visit_exception_table(&mut self, exception_table: Vec<Exception>) -> Result<()>;
	fn visit_instruction(&mut self,
		label: Option<Label>,
		frame: Option<StackMapData>,
		instruction: Instruction,
	) -> Result<()> {
		// TODO: finalize this api...
		Ok(())
	}
	/// Visits the last label.
	///
	/// We need to visit the "last" label (the one that's one after the end of the method),
	/// as [`LabelRange`]s can reference this label, because they use an exclusive index for the end.
	fn visit_last_label(&mut self, last_label: Label) -> Result<()>;

	fn visit_line_numbers(&mut self, line_number_table: Vec<(Label, u16)>) -> Result<()>;
	fn visit_local_variables(&mut self, local_variables: Vec<Lv>) -> Result<()>;

	fn visit_type_annotations(self, visible: bool) -> Result<(Self::TypeAnnotationsResidual, Self::TypeAnnotationsVisitor)>;
	fn finish_type_annotations(this: Self::TypeAnnotationsResidual, type_annotations_visitor: Self::TypeAnnotationsVisitor) -> Result<Self>;

	fn visit_unknown_attribute(&mut self, unknown_attribute: Self::UnknownAttribute) -> Result<()>;
}


#[derive(Default)]
pub struct CodeInterests {
	// Attributes of the `Code` attribute:
	pub stack_map_table: bool,

	pub line_number_table: bool,

	pub local_variable_table: bool,
	pub local_variable_type_table: bool,

	pub runtime_visible_type_annotations: bool,
	pub runtime_invisible_type_annotations: bool,

	pub unknown_attributes: bool,
}

impl CodeInterests {
	pub fn none() -> CodeInterests {
		Self::default()
	}
	pub fn all() -> CodeInterests {
		CodeInterests {
			stack_map_table: true,

			line_number_table: true,

			local_variable_table: true,
			local_variable_type_table: true,

			runtime_visible_type_annotations: true,
			runtime_invisible_type_annotations: true,

			unknown_attributes: true,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationTypeInfo {
	Top,
	Integer,
	Float,
	Long,
	Double,
	Null,
	UninitializedThis,
	Object(ClassName),
	Uninitialized(Label),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StackMapData {
	Same,
	SameLocals1StackItem {
		stack: VerificationTypeInfo,
	},
	Chop {
		k: u8,
	},
	Append {
		locals: Vec<VerificationTypeInfo>
	},
	Full {
		locals: Vec<VerificationTypeInfo>,
		stack: Vec<VerificationTypeInfo>,
	},
}
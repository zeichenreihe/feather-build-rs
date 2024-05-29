use std::collections::HashMap;
use anyhow::{anyhow, Context, Result};
use crate::tree::method::code::{Label, LabelRange};

/// A helper struct for writing [`Label`]s as bytecode offsets.
pub(crate) struct Labels {
	/// From an "instruction index" (so index into the input instructions list) to the
	/// bytecode offset.
	index_to_offset: HashMap<usize, u16>,
	/// [`Label`] to bytecode offsets mapping.
	labels: HashMap<Label, u16>,
}

impl Labels {
	pub(crate) fn new() -> Labels {
		Labels {
			index_to_offset: HashMap::new(),
			labels: HashMap::new(),
		}
	}

	pub(crate) fn add_instruction(&mut self, instruction_index: usize, opcode_pos: u16) {
		self.index_to_offset.insert(instruction_index, opcode_pos);
	}

	/// Adds a known [`Label`] to opcode position mapping for this writing attempt.
	pub(crate) fn add_opcode_pos_label(&mut self, label: Label, opcode_pos: u16) {
		self.labels.insert(label, opcode_pos);
	}

	pub(crate) fn get(&self, target: &Label) -> Option<u16> {
		self.labels.get(target).copied()
	}
	pub(crate) fn try_get(&self, target: &Label) -> Result<u16> {
		self.get(target).with_context(|| anyhow!("no bytecode offset for label {target:?}"))
	}

	pub(crate) fn try_get_range(&self, range: &LabelRange) -> Result<(u16, u16)> {
		let start = self.try_get(&range.start)?;
		let end = self.try_get(&range.end)?;
		Ok((start, end - start))
	}

	pub(crate) fn next_attempt(&mut self) {
		self.index_to_offset = HashMap::with_capacity(self.index_to_offset.len());
		self.labels = HashMap::with_capacity(self.labels.len());
	}
}
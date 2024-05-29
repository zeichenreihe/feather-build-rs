use std::collections::{HashMap};
use anyhow::{anyhow, bail, Context, Result};
use crate::tree::method::code::{Label, LabelRange};

/// A helper struct for reading bytecode offsets into [`Label`]s.
pub(crate) struct Labels {
	code_length: u16,
	labels: HashMap<u16, Label>,
	max_id: u16,
}

impl Labels {
	pub(crate) fn new(code_length: u16) -> Labels {
		Labels {
			code_length,
			labels: HashMap::with_capacity(code_length as usize / 3),
			max_id: 0,
		}
	}

	fn get_or_add_unchecked(&mut self, pc: u16) -> &mut Label {
		self.labels.entry(pc).or_insert_with(|| {
			let label = Label { id: self.max_id };
			self.max_id += 1;
			label
		})
	}

	pub(crate) fn create(&mut self, pc: u16) -> Result<()> {
		if pc >= self.code_length {
			bail!("label for bytecode offset {pc:?} out of bounds for code length {:?}", self.code_length);
		}

		self.get_or_add_unchecked(pc);
		Ok(())
	}

	pub(crate) fn get_or_create(&mut self, pc: u16) -> Result<Label> {
		if pc >= self.code_length {
			bail!("label for bytecode offset {pc:?} out of bounds for code length {:?}", self.code_length);
		}

		Ok(*self.get_or_add_unchecked(pc))
	}

	fn get_or_create_check_exclusive(&mut self, pc: u16) -> Result<Label> {
		//TODO: consider making Label an enum that contains a "end-of-code" variant instead of doing that
		// last_label stuff, tho only when the size of a Label can remain the size of an u16
		if pc > self.code_length {
			bail!("label for bytecode offset {pc:?} out of bounds for code length {:?}", self.code_length);
		}

		Ok(*self.get_or_add_unchecked(pc))
	}

	pub(crate) fn get_or_create_range(&mut self, start_pc: u16, length: u16) -> Result<LabelRange> {
		Ok(LabelRange {
			start: self.get_or_create(start_pc)?,
			end: self.get_or_create_check_exclusive(start_pc + length)?,
		})
	}

	pub(crate) fn try_get(&self, pc: u16) -> Result<Label> {
		self.get(pc).with_context(|| anyhow!("no label at bytecode offset {pc:?}"))
	}

	pub(crate) fn get(&self, pc: u16) -> Option<Label> {
		self.labels.get(&pc).copied()
	}
}
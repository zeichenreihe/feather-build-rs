use std::io::Read;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use crate::specialized_methods::class_file::MyRead;

#[derive(Debug)]
pub(crate) struct Pool(IndexMap<usize, PoolEntry>);
impl Pool {
	pub(crate) fn parse(reader: &mut impl Read) -> Result<Pool> {
		let count = reader.read_u16_as_usize()?;
		let mut map = IndexMap::new();

		for i in 1..count { // indexing is from 1
			let entry = PoolEntry::parse(reader)
				.with_context(|| format!("Failed to parse constant pool entry number {i}"))?;

			if let Some(entry) = entry {
				map.insert(i, entry);
			}
		}

		Ok(Pool(map))
	}

	pub(crate) fn get_utf8_info(&self, index: usize) -> Result<String> {
		let entry = self.0.get(&index).ok_or_else(|| anyhow!("constant pool index out of bounds: {index} for pool size {}", self.0.len()))?;
		let PoolEntry::Utf8(vec) = entry else {
			bail!("Entry isn't Utf8, we got: {entry:?}");
		};
		String::from_utf8(vec.clone())
			.context("We can only work with pure utf8")
	}

	pub(crate) fn get_class_name(&self, index: usize) -> Result<String> {
		let entry = self.0.get(&index).ok_or_else(|| anyhow!("constant pool index out of bounds: {index} for pool size {}", self.0.len()))?;
		let PoolEntry::Class(index) = entry else {
			bail!("Entry isn't Class, we got: {entry:?}");
		};
		self.get_utf8_info(*index)
	}

	pub(crate) fn get_super_class(&self, index: usize) -> Result<Option<String>> {
		if index == 0 {
			Ok(None)
		} else {
			let entry = self.0.get(&index).ok_or_else(|| anyhow!("constant pool index out of bounds: {index} for pool size {}", self.0.len()))?;
			let PoolEntry::Class(index) = entry else {
				bail!("Entry isn't Class, we got: {entry:?}");
			};
			Ok(Some(self.get_utf8_info(*index)?))
		}
	}
}

#[derive(Debug)]
pub(crate) enum PoolEntry {
	Utf8(Vec<u8>),
	Class(usize),
}
impl PoolEntry {
	fn parse(reader: &mut impl Read) -> Result<Option<PoolEntry>> {
		Ok(match reader.read_u8()? {
			1 => Some(Self::Utf8(reader.read_vec(
				|r| r.read_u16_as_usize(),
				|r| r.read_u8()
			)?)),
			3 | 4 | 9 | 10 | 11 | 12 | 18 => {
				reader.read_u32()?;
				None
			},
			5 | 6 => {
				reader.read_u32()?;
				reader.read_u32()?;
				None
			},
			7 => Some(Self::Class(reader.read_u16_as_usize()?)),
			8 | 16 => {
				reader.read_u16()?;
				None
			},
			15 => {
				reader.read_u16()?;
				reader.read_u8()?;
				None
			},
			tag => bail!("Unknown constant pool tag {tag}"),
		})
	}
}
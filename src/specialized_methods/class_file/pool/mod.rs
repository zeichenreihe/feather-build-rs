use std::io::Read;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use crate::specialized_methods::class_file::MyRead;
use crate::tree::descriptor::{FieldDescriptor, MethodDescriptor};

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

	fn get(&self, index: usize) -> Result<&PoolEntry> {
		self.0.get(&index)
			.ok_or_else(|| anyhow!("Constant pool index {index} out of bounds for pool size {}", self.0.len()))
	}

	pub(crate) fn get_utf8_info(&self, index: usize) -> Result<String> {
		let entry = self.get(index)?;
		let PoolEntry::Utf8(vec) = entry else {
			bail!("Entry isn't Utf8, we got: {entry:?}");
		};
		String::from_utf8(vec.clone())
			.context("We can only work with pure utf8")
	}

	pub(crate) fn get_class_name(&self, index: usize) -> Result<String> {
		let entry = self.get(index)?;
		let PoolEntry::Class(index) = entry else {
			bail!("Entry isn't Class, we got: {entry:?}");
		};
		self.get_utf8_info(*index)
	}

	pub(crate) fn get_super_class(&self, index: usize) -> Result<Option<String>> {
		if index == 0 {
			Ok(None)
		} else {
			let entry = self.get(index)?;
			let PoolEntry::Class(index) = entry else {
				bail!("Entry isn't Class, we got: {entry:?}");
			};
			Ok(Some(self.get_utf8_info(*index)?))
		}
	}

	pub(crate) fn get_field_ref(&self, index: usize) -> Result<(String, String, FieldDescriptor)> {
		let entry = self.get(index)?;
		let PoolEntry::FieldRef(class_index, name_and_type_index) = entry else {
			bail!("Entry isn't FieldRef, we got: {entry:?}");
		};

		let class = self.get_class_name(*class_index)?;

		let entry = self.get(*name_and_type_index)?;
		let PoolEntry::NameAndType(name_index, descriptor_index) = entry else {
			bail!("Entry isn't NameAndType, we got: {entry:?}");
		};

		let name = self.get_utf8_info(*name_index)?;
		let descriptor = self.get_utf8_info(*descriptor_index)?.try_into()?;

		Ok((class, name, descriptor))
	}

	pub(crate) fn get_method_ref(&self, index: usize) -> Result<(String, String, MethodDescriptor)> {
		let entry = self.get(index)?;
		let PoolEntry::MethodRef(class_index, name_and_type_index) = entry else {
			bail!("Entry isn't MethodRef, we got: {entry:?}");
		};

		let class = self.get_class_name(*class_index)?;

		let entry = self.get(*name_and_type_index)?;
		let PoolEntry::NameAndType(name_index, descriptor_index) = entry else {
			bail!("Entry isn't NameAndType, we got: {entry:?}");
		};

		let name = self.get_utf8_info(*name_index)?;
		let descriptor = self.get_utf8_info(*descriptor_index)?.try_into()?;

		Ok((class, name, descriptor))
	}
}

/// This graph shows what depends (has an index to of a type) on what:
/// ```txt
/// Long  Double  Utf8  Integer  Float
///      __________/\_______________
///     /      /     \    \         \
/// String  Class  NameAndType  MethodType
///           |      |      \
///           FieldRef    InvokeDynamic
///           MethodRef
///       InterfaceMethodRef
///              |
///         MethodHandle
/// ```
#[derive(Debug)]
pub(crate) enum PoolEntry {
	Utf8(Vec<u8>),
	FieldRef(usize, usize),
	/// used for both `MethodRefInfo` (`10`) and `InterfaceMethodRefInfo` (`11`)
	MethodRef(usize, usize),
	NameAndType(usize, usize),
	Class(usize),
}
impl PoolEntry {
	fn parse(reader: &mut impl Read) -> Result<Option<PoolEntry>> {
		Ok(match reader.read_u8()? {
			1 => Some(Self::Utf8(reader.read_vec(
				|r| r.read_u16_as_usize(),
				|r| r.read_u8()
			)?)),
			9 => {
				let class_index = reader.read_u16_as_usize()?;
				let name_and_type_index = reader.read_u16_as_usize()?;

				Some(Self::FieldRef(class_index, name_and_type_index))
			},
			3 | 4 | 18 => {
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
			10 | 11 => {
				let class_index = reader.read_u16_as_usize()?;
				let name_and_type_index = reader.read_u16_as_usize()?;

				Some(Self::MethodRef(class_index, name_and_type_index))
			},
			12 => {
				let name_index = reader.read_u16_as_usize()?;
				let descriptor_index = reader.read_u16_as_usize()?;

				Some(Self::NameAndType(name_index, descriptor_index))
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
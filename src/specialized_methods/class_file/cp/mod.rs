use anyhow::{anyhow, bail, Context, Result};
use crate::specialized_methods::class_file::MyRead;
use crate::specialized_methods::class_file::name::{ClassName, FieldDescriptor, FieldName, MethodDescriptor, MethodName};

pub(crate) mod attribute;

#[derive(Debug)]
pub(crate) struct Pool(Vec<PoolEntry>);
impl Pool {
	pub(crate) fn parse(reader: &mut impl MyRead) -> Result<Pool> {
		let count = reader.read_u16_as_usize()?;
		let mut vec = Vec::with_capacity(count);

		vec.push(PoolEntry::None); // constant pool indices are based on 0

		for i in 1..count { // indexing is from 1
			let entry = PoolEntry::parse(reader)
				.with_context(|| format!("Failed to parse constant pool entry number {i}"))?;

			vec.push(entry);
		}
		Ok(Pool(vec))
	}

	pub(crate) fn get_utf8_info(&self, index: usize) -> Result<&Vec<u8>> {
		let entry = self.0.get(index).ok_or_else(|| anyhow!("constant pool index out of bounds: {index} for pool size {}", self.0.len()))?;
		let PoolEntry::Utf8(vec) = entry else {
			bail!("Entry isn't Utf8, we got: {entry:?}");
		};
		Ok(&vec)
	}


	pub(crate) fn get_class_name(&self, index: usize) -> Result<ClassName> {
		let entry = self.0.get(index).ok_or_else(|| anyhow!("constant pool index out of bounds: {index} for pool size {}", self.0.len()))?;
		let PoolEntry::Class(index) = entry else {
			bail!("Entry isn't Class, we got: {entry:?}");
		};
		let string = String::from_utf8(self.get_utf8_info(*index)?.clone()).context("We can only work with utf8 class names")?;
		Ok(ClassName(string))
	}

	pub(crate) fn get_super_class(&self, index: usize) -> Result<Option<ClassName>> {
		let entry = self.0.get(index).ok_or_else(|| anyhow!("constant pool index out of bounds: {index} for pool size {}", self.0.len()))?;
		if entry == &PoolEntry::None {
			return Ok(None);
		}
		let PoolEntry::Class(index) = entry else {
			bail!("Entry isn't Class, we got: {entry:?}");
		};
		let string = String::from_utf8(self.get_utf8_info(*index)?.clone()).context("We can only work with utf8 class names")?;
		Ok(Some(ClassName(string)))
	}

	pub(crate) fn get_field_descriptor(&self, index: usize) -> Result<FieldDescriptor> {
		let string = String::from_utf8(self.get_utf8_info(index)?.clone()).context("We can only work with utf8 field descriptors")?;
		Ok(FieldDescriptor(string))
	}

	pub(crate) fn get_field_name(&self, index: usize) -> Result<FieldName> {
		let string = String::from_utf8(self.get_utf8_info(index)?.clone()).context("We can only work with utf8 field names")?;
		Ok(FieldName(string))
	}


	pub(crate) fn get_method_descriptor(&self, index: usize) -> Result<MethodDescriptor> {
		let string = String::from_utf8(self.get_utf8_info(index)?.clone()).context("We can only work with utf8 method descriptors")?;
		Ok(MethodDescriptor(string))
	}

	pub(crate) fn get_method_name(&self, index: usize) -> Result<MethodName> {
		let string = String::from_utf8(self.get_utf8_info(index)?.clone()).context("We can only work with utf8 method names")?;
		Ok(MethodName(string))
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
#[derive(Debug, PartialEq)]
pub(crate) enum PoolEntry {
	None, // used for index = 0
	Utf8(Vec<u8>),
	Integer(u32),
	Float(u32),
	Long { high: u32, low: u32 },
	Double { high: u32, low: u32 },
	Class(usize),
	String(usize),
	FieldRef {
		class_index: usize,
		name_and_type_index: usize,
	},
	MethodRef {
		class_index: usize,
		name_and_type_index: usize,
	},
	InterfaceMethodRef {
		class_index: usize,
		name_and_type_index: usize,
	},
	NameAndType {
		name_index: usize,
		descriptor_index: usize,
	},
	MethodHandle(u8, usize),
	MethodType(usize),
	InvokeDynamic {
		bootstrap_method_attribute_index: u16,
		name_and_type_index: usize,
	},
}
impl PoolEntry {
	fn parse<R: MyRead>(reader: &mut R) -> Result<PoolEntry> {
		Ok(match reader.read_u8()? {
			1 => Self::Utf8(reader.read_vec(
				|r| r.read_u16_as_usize(),
				|r| r.read_u8()
			)?),
			3 => Self::Integer(reader.read_u32()?),
			4 => Self::Float(reader.read_u32()?),
			5 => Self::Long {
				high: reader.read_u32()?,
				low: reader.read_u32()?,
			},
			6 => Self::Double {
				high: reader.read_u32()?,
				low: reader.read_u32()?,
			},
			7 => Self::Class(reader.read_u16_as_usize()?),
			8 => Self::String(reader.read_u16_as_usize()?),
			9 => Self::FieldRef {
				class_index: reader.read_u16_as_usize()?,
				name_and_type_index: reader.read_u16_as_usize()?,
			},
			10 => Self::MethodRef {
				class_index: reader.read_u16_as_usize()?,
				name_and_type_index: reader.read_u16_as_usize()?,
			},
			11 => Self::InterfaceMethodRef {
				class_index: reader.read_u16_as_usize()?,
				name_and_type_index: reader.read_u16_as_usize()?,
			},
			12 => Self::NameAndType {
				name_index: reader.read_u16_as_usize()?,
				descriptor_index: reader.read_u16_as_usize()?,
			},
			15 => Self::MethodHandle(reader.read_u8()?, reader.read_u16_as_usize()?),
			16 => Self::MethodType(reader.read_u16_as_usize()?),
			18 => Self::InvokeDynamic {
				bootstrap_method_attribute_index: reader.read_u16()?,
				name_and_type_index: reader.read_u16_as_usize()?,
			},
			tag => bail!("Unknown constant pool tag {tag}"),
		})
	}
}
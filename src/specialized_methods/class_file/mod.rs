use anyhow::{bail, Context, Result};
use std::fmt::Debug;
use std::io::Read;
use crate::specialized_methods::class_file::access::{ClassInfoAccess, FieldInfoAccess, MethodInfoAccess};
use crate::specialized_methods::class_file::cp::Pool;
use crate::specialized_methods::class_file::name::{ClassName, FieldDescriptor, FieldName, MethodDescriptor, MethodName};


pub(crate) mod name;
pub(crate) mod access;
pub(crate) mod cp;

pub(crate) trait MyRead: Read {
	fn read_n<const N: usize>(&mut self) -> Result<[u8; N]> {
		let mut buf = [0u8; N];
		let length = self.read(&mut buf)?;
		if length == N {
			Ok(buf)
		} else {
			bail!("unexpected data end")
		}
	}
	fn read_u8(&mut self) -> Result<u8> {
		Ok(u8::from_be_bytes(self.read_n()?))
	}
	fn read_u16(&mut self) -> Result<u16> {
		Ok(u16::from_be_bytes(self.read_n()?))
	}
	fn read_u32(&mut self) -> Result<u32> {
		Ok(u32::from_be_bytes(self.read_n()?))
	}
	fn read_u8_as_usize(&mut self) -> Result<usize> {
		Ok(self.read_u8()? as usize)
	}
	fn read_u16_as_usize(&mut self) -> Result<usize> {
		Ok(self.read_u16()? as usize)
	}
	fn read_u32_as_usize(&mut self) -> Result<usize> {
		Ok(self.read_u32()? as usize)
	}
	fn read_i8(&mut self) -> Result<i8> {
		Ok(self.read_u8()? as i8)
	}
	fn read_i16(&mut self) -> Result<i16> {
		Ok(self.read_u16()? as i16)
	}
	fn read_i32(&mut self) -> Result<i32> {
		Ok(self.read_u32()? as i32)
	}
	fn read_vec<T, S, E>(&mut self, get_size: S, get_element: E) -> Result<Vec<T>>
	where
		S: FnOnce(&mut Self) -> Result<usize>,
		E: Fn(&mut Self) -> Result<T>
	{
		let size = get_size(self)?;
		let mut vec = Vec::with_capacity(size);
		for _ in 0..size {
			vec.push(get_element(self)?);
		}
		Ok(vec)
	}
}
impl<T: Read> MyRead for T {}


fn nom_attribute(reader: &mut impl Read) -> Result<()> {
	let _ = reader.read_u16_as_usize()?;
	let attribute_length = reader.read_u32()?;


	let _ = reader.read_vec( // TODO: _
		|r| Ok(attribute_length as usize),
		|r| r.read_u8()
	)?;
	return Ok(());
}

#[derive(Debug, Clone)]
pub(crate) struct FieldInfo {
	pub(crate) access_flags: FieldInfoAccess,
	pub(crate) name: FieldName,
	pub(crate) descriptor: FieldDescriptor,
}

impl FieldInfo {
	fn parse(reader: &mut impl Read, pool: &Pool) -> Result<Self> {
		let access_flags = reader.read_u16()?.into();

		let name = pool.get_field_name(reader.read_u16_as_usize()?)
			.with_context(|| "Failed to get field name from constant pool")?;

		let descriptor = pool.get_field_descriptor(reader.read_u16_as_usize()?)
			.with_context(|| "Failed to get field descriptor from constant pool")?;

		let attributes = reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| nom_attribute(r)
				.with_context(|| "Failed to parse field attribute")
		)?;

		Ok(FieldInfo { access_flags, name, descriptor })
	}
}

#[derive(Debug, Clone)]
pub(crate) struct MethodInfo {
	pub(crate) access_flags: MethodInfoAccess,
	pub(crate) name: MethodName,
	pub(crate) descriptor: MethodDescriptor,
}

impl MethodInfo {
	fn parse(reader: &mut impl Read, pool: &Pool) -> Result<Self> {
		let access_flags = reader.read_u16()?.into();

		let name = pool.get_method_name(reader.read_u16_as_usize()?)
			.with_context(|| "Failed to get method name from constant pool")?;

		let descriptor = pool.get_method_descriptor(reader.read_u16_as_usize()?)
			.with_context(|| "Failed to get method descriptor from constant pool")?;

		let attributes = reader.read_vec(
		   |r| r.read_u16_as_usize(),
		   |r| nom_attribute(r)
			   .with_context(|| "Failed to parse method attribute")
		)?;

		Ok(MethodInfo { access_flags, name, descriptor })
	}
}

#[derive(Debug, Clone)]
pub(crate) struct ClassFile {
	pub(crate) minor_version: u16,
	pub(crate) major_version: u16,
	pub(crate) access_flags: ClassInfoAccess,
	pub(crate) this_class: ClassName,
	pub(crate) super_class: Option<ClassName>,
	pub(crate) interfaces: Vec<ClassName>,
	pub(crate) fields: Vec<FieldInfo>,
	pub(crate) methods: Vec<MethodInfo>,
}

impl ClassFile {
	pub(crate) fn parse(reader: &mut impl Read) -> Result<Self> {
		let magic = reader.read_u32()?;
		if magic != 0xCAFE_BABE {
			bail!("Magic didn't match up: {magic:x}")
		}

		let minor_version = reader.read_u16()?;
		let major_version = reader.read_u16()?;

		if major_version <= 51 {
			bail!("We only accept class files with version >= 52.0, this one has: {major_version}.{minor_version}")
		}

		let pool = Pool::parse(reader)
			.with_context(|| "Failed to parse constant pool")?;

		let access_flags = reader.read_u16()?.into();

		let this_class: ClassName = pool.get_class_name(reader.read_u16_as_usize()?)
			.with_context(|| "Failed to get constant pool item `this_class`")?;
		let super_class: Option<ClassName> = pool.get_super_class(reader.read_u16_as_usize()?)
			.with_context(|| "Failed to get constant pool item `super_class`")?;

		let interfaces: Vec<ClassName> = reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| pool.get_class_name(r.read_u16_as_usize()?)
				.with_context(|| "Failed to get constant pool item representing a direct superinterface")
		)?;

		let fields = reader.read_vec(
			|r| r.read_u16_as_usize(),
			|r| FieldInfo::parse(r, &pool)
				.with_context(|| "Failed to parse a field")
		)?;

		let methods = reader.read_vec(
		    |r| r.read_u16_as_usize(),
		    |r| MethodInfo::parse(r, &pool)
				.with_context(|| "Failed to parse a method")
		)?;

		let attributes = reader.read_vec(
			|r| r.read_u16_as_usize(),
		   |r| nom_attribute(r)
			   .with_context(|| "Failed to parse an attribute for a class file")
		)?;

		let mut end = [0u8];
		if reader.read(&mut end)? != 0 {
			bail!("Expected end of class file")
		}

		Ok(ClassFile { minor_version, major_version, access_flags, this_class, super_class, interfaces, fields, methods })
	}
}
use anyhow::{bail, Context, Result};
use std::fmt::Debug;
use std::io::Read;
use crate::specialized_methods::class_file::cp::Pool;
use crate::tree::access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use crate::tree::descriptor::{FieldDescriptor, MethodDescriptor};


pub(crate) mod name;
pub(crate) mod cp;

trait MyRead: Read {
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

fn nom_attributes(reader: &mut impl Read) -> Result<()> {
	for _ in 0..reader.read_u16_as_usize()? {
		let _ = reader.read_u16_as_usize()?;

		for _ in 0..reader.read_u32_as_usize()? {
			let _ = reader.read_u8()?;
		}
	}

	Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct FieldInfo {
	pub(crate) access_flags: FieldAccessFlags,
	pub(crate) name: String,
	pub(crate) descriptor: FieldDescriptor,
}

impl FieldInfo {
	fn parse(reader: &mut impl Read, pool: &Pool) -> Result<Self> {
		let access_flags = reader.read_u16()?;
		let name_index = reader.read_u16_as_usize()?;
		let descriptor_index = reader.read_u16_as_usize()?;
		nom_attributes(reader)
			.with_context(|| "Failed to parse field attributes")?;

		Ok(FieldInfo {
			access_flags: FieldAccessFlags {
				is_public:    access_flags & 0x0001 != 0,
				is_private:   access_flags & 0x0002 != 0,
				is_protected: access_flags & 0x0004 != 0,
				is_static:    access_flags & 0x0008 != 0,
				is_final:     access_flags & 0x0010 != 0,
				is_volatile:  access_flags & 0x0040 != 0,
				is_transient: access_flags & 0x0080 != 0,
				is_synthetic: access_flags & 0x1000 != 0,
				is_enum:      access_flags & 0x4000 != 0,
			},
			name: pool.get_utf8_info(name_index)
				.with_context(|| "Failed to get field name from constant pool")?,
			descriptor: pool.get_utf8_info(descriptor_index)?.as_str().try_into()
				.with_context(|| "Failed to get field descriptor from constant pool")?,
		})
	}
}

#[derive(Debug, Clone)]
pub(crate) struct MethodInfo {
	pub(crate) access_flags: MethodAccessFlags,
	pub(crate) name: String,
	pub(crate) descriptor: MethodDescriptor,
}

impl MethodInfo {
	fn parse(reader: &mut impl Read, pool: &Pool) -> Result<Self> {
		let access_flags = reader.read_u16()?;
		let name_index = reader.read_u16_as_usize()?;
		let descriptor_index = reader.read_u16_as_usize()?;
		nom_attributes(reader)
			.with_context(|| "Failed to parse method attributes")?;

		Ok(MethodInfo {
			access_flags: MethodAccessFlags {
				is_public:       access_flags & 0x0001 != 0,
				is_private:      access_flags & 0x0002 != 0,
				is_protected:    access_flags & 0x0004 != 0,
				is_static:       access_flags & 0x0008 != 0,
				is_final:        access_flags & 0x0010 != 0,
				is_synchronised: access_flags & 0x0020 != 0,
				is_bridge:       access_flags & 0x0040 != 0,
				is_varargs:      access_flags & 0x0080 != 0,
				is_native:       access_flags & 0x0100 != 0,
				is_abstract:     access_flags & 0x0400 != 0,
				is_strict:       access_flags & 0x0800 != 0,
				is_synthetic:    access_flags & 0x1000 != 0,
			},
			name: pool.get_utf8_info(name_index)
				.with_context(|| "Failed to get method name from constant pool")?,
			descriptor: pool.get_utf8_info(descriptor_index)?.as_str().try_into()
				.with_context(|| "Failed to get method descriptor from constant pool")?,
		})
	}
}

#[derive(Debug, Clone)]
pub(crate) struct ClassFile {
	pub(crate) minor_version: u16,
	pub(crate) major_version: u16,
	pub(crate) access_flags: ClassAccessFlags,
	pub(crate) this_class: String,
	pub(crate) super_class: Option<String>,
	pub(crate) interfaces: Vec<String>,
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

		let pool = Pool::parse(reader)
			.with_context(|| "Failed to parse constant pool")?;

		let access_flags = reader.read_u16()?;
		let this_class_index = reader.read_u16_as_usize()?;
		let super_class_index = reader.read_u16_as_usize()?;

		let interfaces = reader.read_vec(
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

		nom_attributes(reader)
			.with_context(|| "Failed to parse class attributes")?;

		let mut end = [0u8];
		if reader.read(&mut end)? != 0 {
			bail!("Expected end of class file")
		}

		Ok(ClassFile {
			minor_version,
			major_version,
			access_flags: ClassAccessFlags {
				is_public:     access_flags & 0x0001 != 0,
				is_final:      access_flags & 0x0010 != 0,
				is_super:      access_flags & 0x0020 != 0,
				is_interface:  access_flags & 0x0200 != 0,
				is_abstract:   access_flags & 0x0400 != 0,
				is_synthetic:  access_flags & 0x1000 != 0,
				is_annotation: access_flags & 0x2000 != 0,
				is_enum:       access_flags & 0x4000 != 0,
			},
			this_class: pool.get_class_name(this_class_index)
				.with_context(|| "Failed to get constant pool item `this_class`")?,
			super_class: pool.get_super_class(super_class_index)
				.with_context(|| "Failed to get constant pool item `super_class`")?,
			interfaces,
			fields,
			methods
		})
	}
}
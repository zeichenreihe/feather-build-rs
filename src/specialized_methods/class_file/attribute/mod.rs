pub(crate) mod code;

use anyhow::{Context, Result};
use std::io::Read;
use indexmap::IndexMap;
use crate::specialized_methods::class_file::attribute::code::CodeAnalysis;
use crate::specialized_methods::class_file::MyRead;
use crate::specialized_methods::class_file::pool::Pool;

#[derive(Debug, Clone)]
pub(crate) enum Attribute {
	Unknown(Vec<u8>),
	Code(CodeAnalysis),
}

impl Attribute {
	fn parse_code_attribute(reader: &mut impl Read, pool: &Pool) -> Result<Attribute> {
		let _attribute_size = reader.read_u32_as_usize()?;

		let _max_stack = reader.read_u16()?;
		let _max_locals = reader.read_u16()?;
		let code = reader.read_vec(
			|r| r.read_u32_as_usize(),
			|r| r.read_u8()
		)?;

		for _ in 0..reader.read_u16_as_usize()? {
			let _start_pc = reader.read_u16()?;
			let _end_pc = reader.read_u16()?;
			let _handler_pc = reader.read_u16()?;
			let _catch_type = reader.read_u16()?;
		}

		let _attributes = nom_attributes(reader)?;

		let analysis = CodeAnalysis::analyze(code, pool)?;

		Ok(Attribute::Code(analysis))
	}
}

fn nom_attributes(reader: &mut impl Read) -> Result<()> {
	for _ in 0..reader.read_u16_as_usize()? {
		let _name_index = reader.read_u16_as_usize()?;

		for _ in 0..reader.read_u32_as_usize()? {
			let _ = reader.read_u8()?;
		}
	}

	Ok(())
}

pub(crate) fn parse_attributes(reader: &mut impl Read, pool: &Pool) -> Result<IndexMap<String, Attribute>> {
	let mut map = IndexMap::new();

	for _ in 0..reader.read_u16_as_usize()? {
		let name_index = reader.read_u16_as_usize()?;

		let name = pool.get_utf8_info(name_index)?;

		let attribute = match name.as_str() {
			"Code" => {
				Attribute::parse_code_attribute(reader, pool)?
			},
			_ => {
				let data = reader.read_vec(
					|r1| r1.read_u32_as_usize(),
					|r1| r1.read_u8()
				)?;

				Attribute::Unknown(data)
			},
		};

		map.insert(name, attribute);
	}

	Ok(map)
}
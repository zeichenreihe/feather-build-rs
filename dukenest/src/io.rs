use std::io::{BufRead, BufReader, Read};
use anyhow::{anyhow, bail, Context, Result};
use duke::tree::class::InnerClassFlags;
use duke::tree::method::MethodNameAndDesc;
use crate::{Nest, Nests, NestType};

impl Nests {
	pub fn read(vec: &Vec<u8>) -> Result<Nests> {
		let mut slice = vec.as_slice();
		Self::read_from_reader(&mut slice)
	}

	fn read_from_reader(reader: impl Read) -> Result<Nests> {
		let mut nests = Nests::new();

		for (line_number, line) in BufReader::new(reader).lines().enumerate() {
			let line_number = line_number + 1;
			let line = line?;

			let vec: Vec<&str> = line.split('\t').collect();
			if vec.len() != 6 {
				bail!("invalid mapping {line:?} in {line_number}: wrong number of fields: {}, expected 6", vec.len());
			}
			let array: [&str; 6] = vec.try_into().unwrap(); // can't panic, we checked the size

			let [class_name, encl_class_name, encl_method_name,
				encl_method_desc, inner_name, access_string] = array;

			if class_name.is_empty() {
				bail!("invalid mapping {line:?} in line {line_number}: missing class name argument");
			}
			if encl_class_name.is_empty() {
				bail!("invalid mapping {line:?} in line {line_number}: missing enclosing class name argument");
			}
			if inner_name.is_empty() {
				bail!("invalid mapping {line:?} in line {line_number}: missing inner class name argument");
			}

			let encl_method = if encl_method_name.is_empty() || encl_method_desc.is_empty() {
				None
			} else {
				Some(MethodNameAndDesc {
					name: encl_method_name.to_owned().into(),
					desc: encl_method_desc.to_owned().into(),
				})
			};

			let access = || -> Result<_> { Ok( if let Some(hex_access) = access_string.strip_prefix("0x") {
				u16::from_str_radix(hex_access, 16)?
			} else if let Some(binary_access) = access_string.strip_prefix("0b") {
				u16::from_str_radix(binary_access, 2)?
			} else {
				access_string.parse()?
			} ) };
			let access = access().with_context(|| anyhow!("invalid mapping {line:?} in line {line_number}: invalid access flags"))?;

			let nest_type = if inner_name.chars().all(|x| x.is_ascii_digit()) {
				NestType::Anonymous
			} else if inner_name.starts_with(|x: char| x.is_ascii_digit()) {
				NestType::Local
			} else {
				NestType::Inner
			};

			let nest = Nest {
				nest_type,
				class_name: class_name.to_owned().into(),
				encl_class_name: encl_class_name.to_owned().into(),
				encl_method,
				inner_name: inner_name.to_owned(),
				inner_access: InnerClassFlags::from(access),
			};

			nests.add(nest);
		}

		Ok(nests)
	}
}
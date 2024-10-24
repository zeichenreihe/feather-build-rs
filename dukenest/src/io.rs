use std::io::{BufRead, BufReader, Read};
use anyhow::{anyhow, bail, Context, Result};
use java_string::JavaString;
use duke::tree::class::ObjClassName;
use duke::tree::method::{MethodDescriptor, MethodName, MethodNameAndDesc};
use crate::{Nest, Nests, NestType};

impl Nests {
	pub fn read(vec: &Vec<u8>) -> Result<Nests> {
		let mut slice = vec.as_slice();
		Self::read_from_reader(&mut slice)
	}

	fn read_from_reader(reader: impl Read) -> Result<Nests> {
		let mut nests = Nests::default();

		for (line_number, line) in BufReader::new(reader).lines().enumerate() {
			let line_number = line_number + 1;
			let line = line.with_context(|| anyhow!("got an reader error on line {line_number}"))?;

			let nest = Self::read_line(&line)
				.with_context(|| anyhow!("invalid mapping {line:?} in line {line_number}"))?;

			nests.add(nest);
		}

		Ok(nests)
	}

	fn read_line(line: &str) -> Result<Nest> {
		let array: [&str; 6] = line.split('\t')
			.collect::<Vec<&str>>()
			.try_into()
			.map_err(|vec: Vec<&str>| anyhow!("wrong number of fields {}, expected 6: {vec:?}", vec.len()))?;

		let [class_name, encl_class_name, encl_method_name, encl_method_desc, inner_name, access_string] = array;

		if class_name.is_empty() {
			bail!("missing class name argument");
		}
		if encl_class_name.is_empty() {
			bail!("missing enclosing class name argument");
		}
		if inner_name.is_empty() {
			bail!("missing inner class name argument");
		}

		Ok(Nest {
			nest_type: if inner_name.chars().all(|x| x.is_ascii_digit()) {
				NestType::Anonymous
			} else if inner_name.starts_with(|x: char| x.is_ascii_digit()) {
				NestType::Local
			} else {
				NestType::Inner
			},
			class_name: ObjClassName::try_from(JavaString::from(class_name.to_owned()))
				.with_context(|| anyhow!("{class_name:?} is not a valid class name"))?,
			encl_class_name: ObjClassName::try_from(JavaString::from(encl_class_name.to_owned()))
				.with_context(|| anyhow!("{encl_class_name:?} is not a valid class name"))?,
			encl_method: if encl_method_name.is_empty() || encl_method_desc.is_empty() {
				None
			} else {
				Some(MethodNameAndDesc {
					name: MethodName::try_from(JavaString::from(encl_method_name.to_owned()))
						.with_context(|| anyhow!("{encl_method_name:?} is not a valid method name"))?,
					desc: MethodDescriptor::try_from(JavaString::from(encl_method_desc.to_owned()))
						.with_context(|| anyhow!("{encl_method_name:?} is not a valid method descriptor"))?,
				})
			},
			inner_name: inner_name.to_owned().into(),
			inner_access: Self::parse_u16_hex_binary_and_decimal(access_string)
				.with_context(|| anyhow!("invalid access flags: {access_string:?}"))?
				.into(),
		})
	}

	fn parse_u16_hex_binary_and_decimal(s: &str) -> Result<u16> {
		if let Some(hex) = s.strip_prefix("0x") {
			u16::from_str_radix(hex, 16).context("cannot parse as hex number (prefixed with `0x`)")
		} else if let Some(binary) = s.strip_prefix("0b") {
			u16::from_str_radix(binary, 2).context("cannot parse as binary number (prefixed with `0b`)")
		} else {
			s.parse().context("cannot parse as base-10 number (with no prefix)")
		}
	}
}
//#![allow(unused)] // TODO: remove

mod class_file;

use std::io::{Bytes, Read};
use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use crate::tree::mappings::{ClassKey, MethodKey};

#[derive(Debug, Clone)]
struct SpecializedMethods {
	classes: IndexMap<ClassKey, SpecializedMethodsClass>,
}

#[derive(Debug, Clone)]
struct SpecializedMethodsClass {
	methods: IndexMap<MethodKey, MethodKey>,
}

fn get_specialized_methods() -> SpecializedMethods {
	todo!()
}


fn read_class(mut reader: impl Read) -> Result<()> {
	let class = class_file::ClassFile::parse(&mut reader)?;

	println!("{class:#?}");

	Ok(())
}

#[cfg(test)]
mod testing {
	#[test]
	fn class_file() {
		let bytes = include_bytes!("test/MyNode.class");

		crate::specialized_methods::read_class(bytes.as_slice());
	}
}
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use crate::reader::tiny_v2::{Parse, ReadFromColumnIter, try_read, try_read_nonempty, try_read_optional};
use crate::tiny::tree::{ClassMapping, FieldMapping, JavadocMapping, Mappings, MethodMapping, ParameterMapping};

pub(crate) fn read_file(path: impl AsRef<Path> + Debug) -> Result<Mappings> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read mappings file {path:?}"))
}

pub(crate) fn read(reader: impl Read) -> Result<Mappings> {
	Parse::<Mappings, ClassMapping, FieldMapping, MethodMapping, ParameterMapping, JavadocMapping>::parse(reader)
}

impl ReadFromColumnIter for Mappings {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let src = try_read(iter)?;
		let dst = try_read(iter)?;

		Ok(Self::new(src, dst))
	}
}

impl ReadFromColumnIter for ClassMapping {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let src = try_read_nonempty(iter)?;
		let dst = try_read_nonempty(iter)?;

		Ok(Self::new(src, dst))
	}
}

impl ReadFromColumnIter for FieldMapping {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let desc = try_read_nonempty(iter)?;
		let src = try_read_nonempty(iter)?;
		let dst = try_read_nonempty(iter)?;

		Ok(Self::new(desc, src, dst))
	}
}

impl ReadFromColumnIter for MethodMapping {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let desc = try_read_nonempty(iter)?;
		let src = try_read_nonempty(iter)?;
		let dst = try_read_nonempty(iter)?;

		Ok(Self::new(desc, src, dst))
	}
}

impl ReadFromColumnIter for ParameterMapping {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let index = try_read_nonempty(iter)?.parse()
			.with_context(|| anyhow!("illegal parameter index"))?;
		let src = try_read_optional(iter)?.unwrap_or(String::new()); // TODO: ask space what this means, change to `try_read_nonempty(&mut iter)` to see it fail
		let dst = try_read_nonempty(iter)?;

		Ok(Self::new(index, src, dst))
	}
}

impl ReadFromColumnIter for JavadocMapping {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let jav = try_read_nonempty(iter)?;

		Ok(Self::new(jav))
	}
}
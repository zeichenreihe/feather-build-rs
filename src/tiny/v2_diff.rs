use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use crate::reader::tiny_v2::{Parse, ReadFromColumnIter, try_read_nonempty, try_read_optional};
use crate::tiny::diff::{ClassDiff, Diffs, FieldDiff, JavadocDiff, MethodDiff, ParameterDiff};

pub(crate) fn read(path: impl AsRef<Path> + Debug) -> Result<Diffs> {
	Parse::<Diffs, ClassDiff, FieldDiff, MethodDiff, ParameterDiff, JavadocDiff>::parse(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read diff file {path:?}"))
}

impl ReadFromColumnIter for Diffs {
	fn read_from_column_iter<'a>(_: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		// the caller ensure that the iterator is empty
		Ok(Self::new())
	}
}

impl ReadFromColumnIter for ClassDiff {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let src = try_read_nonempty(iter)?;
		let dst_a = try_read_optional(iter)?;
		let dst_b = try_read_optional(iter)?;

		Ok(Self::new(src, dst_a, dst_b))
	}
}

impl ReadFromColumnIter for FieldDiff {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let desc = try_read_nonempty(iter)?;
		let src = try_read_nonempty(iter)?;
		let dst_a = try_read_optional(iter)?;
		let dst_b = try_read_optional(iter)?;

		Ok(Self::new(desc, src, dst_a, dst_b))
	}
}

impl ReadFromColumnIter for MethodDiff {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let desc = try_read_nonempty(iter)?;
		let src = try_read_nonempty(iter)?;
		let dst_a = try_read_optional(iter)?;
		let dst_b = try_read_optional(iter)?;

		Ok(Self::new(desc, src, dst_a, dst_b))
	}
}

impl ReadFromColumnIter for ParameterDiff {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let index = try_read_nonempty(iter)?.parse()
			.with_context(|| anyhow!("illegal parameter index"))?;
		let src = try_read_optional(iter)?.unwrap_or(String::new()); // TODO: ask space what this means, change to `try_read_nonempty(&mut iter)` to see it fail
		let dst_a = try_read_optional(iter)?;
		let dst_b = try_read_optional(iter)?;

		Ok(Self::new(index, src, dst_a, dst_b))
	}
}

impl ReadFromColumnIter for JavadocDiff {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let jav_a = try_read_optional(iter)?;
		let jav_b = try_read_optional(iter)?;

		Ok(Self::new(jav_a, jav_b))
	}
}

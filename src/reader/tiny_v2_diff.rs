use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use crate::reader::{AddMember, parse, ParseEntry, SetDoc, try_read, try_read_optional};

pub fn read(path: impl AsRef<Path> + Debug) -> Result<Diff> {
	parse::<Diff, ClassDiff, FieldDiff, MethodDiff, ParameterDiff, JavadocDiff>(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read file {path:?}"))
}

#[derive(Debug)]
pub struct Diff {
	pub classes: Vec<ClassDiff>,
}

impl ParseEntry for Diff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		if iter.next().is_some() {
			bail!("expected empty namespaces for a diff");
		}

		Ok(Diff {
			classes: Vec::new(),
		})
	}
}
impl AddMember<ClassDiff> for Diff {
	fn add_member(&mut self, member: ClassDiff) {
		self.classes.push(member)
	}
}

#[derive(Debug, Default)]
pub struct ClassDiff {
	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: JavadocDiff,

	pub fields: Vec<FieldDiff>,
	pub methods: Vec<MethodDiff>,
}

impl ParseEntry for ClassDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(Self {
			src: try_read(iter)?,
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			..Self::default()
		})
	}
}
impl SetDoc<JavadocDiff> for ClassDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = doc;
	}
}
impl AddMember<FieldDiff> for ClassDiff {
	fn add_member(&mut self, member: FieldDiff) {
		self.fields.push(member)
	}
}
impl AddMember<MethodDiff> for ClassDiff {
	fn add_member(&mut self, member: MethodDiff) {
		self.methods.push(member)
	}
}

#[derive(Debug, Default)]
pub struct FieldDiff {
	pub desc: String,

	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: JavadocDiff,
}

impl ParseEntry for FieldDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(FieldDiff {
			desc: try_read(iter)?,
			src: try_read(iter)?,
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			..Default::default()
		})
	}
}
impl SetDoc<JavadocDiff> for FieldDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = doc;
	}
}

#[derive(Debug, Default)]
pub struct MethodDiff {
	pub desc: String,

	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: JavadocDiff,

	pub parameters: Vec<ParameterDiff>,
}

impl ParseEntry for MethodDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(MethodDiff {
			desc: try_read(iter)?,
			src: try_read(iter)?,
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			..Default::default()
		})
	}
}
impl SetDoc<JavadocDiff> for MethodDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = doc;
	}
}
impl AddMember<ParameterDiff> for MethodDiff {
	fn add_member(&mut self, member: ParameterDiff) {
		self.parameters.push(member)
	}
}

#[derive(Debug, Default)]
pub struct ParameterDiff {
	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: JavadocDiff,

	pub index: usize,
}

impl ParseEntry for ParameterDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> where Self: Sized {
		Ok(ParameterDiff {
			index: try_read(iter)?.parse()
				.with_context(|| anyhow!("illegal parameter index"))?,
			src: try_read_optional(iter)?.unwrap_or(String::new()), // TODO: ask space what this means, change to `try_read(&mut iter)` to see it fail
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			..Default::default()
		})
	}
}
impl SetDoc<JavadocDiff> for ParameterDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = doc;
	}
}

#[derive(Debug, Default)]
pub struct JavadocDiff {
	pub jav_a: Option<String>,
	pub jav_b: Option<String>,
}

impl ParseEntry for JavadocDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(JavadocDiff {
			jav_a: try_read_optional(iter)?,
			jav_b: try_read_optional(iter)?,
		})
	}
}
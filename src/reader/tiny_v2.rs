use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use crate::reader::{AddMember, parse, ParseEntry, SetDoc, try_read, try_read_optional};

pub fn read(path: impl AsRef<Path> + Debug) -> Result<Mapping> {
	parse::<Mapping, ClassMapping, FieldMapping, MethodMapping, ParameterMapping, JavadocMapping>(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read file {path:?}"))
}

#[derive(Debug)]
pub struct Mapping {
	pub classes: Vec<ClassMapping>,
	pub src: String,
	pub dst: String,
}

impl ParseEntry for Mapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(Mapping {
			src: iter.next()
				.ok_or_else(|| anyhow!("expected two namespaces"))?
				.to_owned(),
			dst: iter.next()
				.ok_or_else(|| anyhow!("expected two namespaces"))?
				.to_owned(),
			classes: Vec::new(),
		})
	}
}
impl AddMember<ClassMapping> for Mapping {
	fn add_member(&mut self, member: ClassMapping) {
		self.classes.push(member)
	}
}

#[derive(Debug, Default)]
pub struct ClassMapping {
	pub src: String,
	pub dst: String,

	pub jav: JavadocMapping,

	pub fields: Vec<FieldMapping>,
	pub methods: Vec<MethodMapping>,
}

impl ParseEntry for ClassMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(ClassMapping {
			src: try_read(iter)?,
			dst: try_read(iter)?,
			..Default::default()
		})
	}
}
impl SetDoc<JavadocMapping> for ClassMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = doc;
	}
}
impl AddMember<FieldMapping> for ClassMapping {
	fn add_member(&mut self, member: FieldMapping) {
		self.fields.push(member);
	}
}
impl AddMember<MethodMapping> for ClassMapping {
	fn add_member(&mut self, member: MethodMapping) {
		self.methods.push(member);
	}
}

#[derive(Debug, Default)]
pub struct FieldMapping {
	pub desc: String,

	pub src: String,
	pub dst: String,

	pub jav: JavadocMapping,
}

impl ParseEntry for FieldMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(FieldMapping {
			desc: try_read(iter)?,
			src: try_read(iter)?,
			dst: try_read(iter)?,
			..Default::default()
		})
	}
}
impl SetDoc<JavadocMapping> for FieldMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = doc;
	}
}

#[derive(Debug, Default)]
pub struct MethodMapping {
	pub desc: String,

	pub src: String,
	pub dst: String,

	pub jav: JavadocMapping,

	pub parameters: Vec<ParameterMapping>,
}

impl ParseEntry for MethodMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(MethodMapping {
			desc: try_read(iter)?,
			src: try_read(iter)?,
			dst: try_read(iter)?,
			..Default::default()
		})
	}
}
impl SetDoc<JavadocMapping> for MethodMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = doc;
	}
}
impl AddMember<ParameterMapping> for MethodMapping {
	fn add_member(&mut self, member: ParameterMapping) {
		self.parameters.push(member);
	}
}

#[derive(Debug, Default)]
pub struct ParameterMapping {
	pub src: String,
	pub dst: String,

	pub jav: JavadocMapping,

	pub index: usize,
}

impl ParseEntry for ParameterMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(ParameterMapping {
			index: try_read(iter)?.parse()
				.with_context(|| anyhow!("illegal parameter index"))?,
			src: try_read_optional(iter)?.unwrap_or(String::new()), // TODO: ask space what this means, change to `try_read(&mut iter)` to see it fail
			dst: try_read(iter)?,
			..Default::default()
		})
	}
}
impl SetDoc<JavadocMapping> for ParameterMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = doc;
	}
}

#[derive(Debug, Default)]
pub struct JavadocMapping {
	pub jav: String,
}

impl ParseEntry for JavadocMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(JavadocMapping {
			jav: try_read(iter)?,
			..Default::default()
		})
	}
}
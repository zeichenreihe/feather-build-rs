use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use crate::reader::{AddMember, parse, ParseEntry, SetDoc, try_read, try_read_optional};

pub fn read(path: impl AsRef<Path> + Debug) -> Result<Mappings> {
	parse::<Mappings, ClassMapping, FieldMapping, MethodMapping, ParameterMapping, JavadocMapping>(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read file {path:?}"))
}

#[derive(Debug, Clone)]
pub struct Mappings {
	pub src: String,
	pub dst: String,
	/// Maps from `src` to the mapping
	pub classes: HashMap<String, ClassMapping>,
}

impl ParseEntry for Mappings {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(Mappings {
			src: iter.next()
				.ok_or_else(|| anyhow!("expected two namespaces"))?
				.to_owned(),
			dst: iter.next()
				.ok_or_else(|| anyhow!("expected two namespaces"))?
				.to_owned(),
			classes: HashMap::new(),
		})
	}
}
impl AddMember<ClassMapping> for Mappings {
	fn add_member(&mut self, member: ClassMapping) {
		self.classes.insert(member.src.clone(), member);
	}
}

#[derive(Debug, Clone)]
pub struct ClassMapping {
	pub src: String,
	pub dst: String,

	pub jav: Option<JavadocMapping>,

	pub fields: HashMap<(String, String), FieldMapping>,
	pub methods: HashMap<(String, String), MethodMapping>,
}

impl ClassMapping {
	pub fn new(src: String, dst: String) -> ClassMapping {
		ClassMapping {
			src, dst,
			jav: None,
			fields: HashMap::new(),
			methods: HashMap::new(),
		}
	}
}
impl ParseEntry for ClassMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(ClassMapping::new(try_read(iter)?, try_read(iter)?))
	}
}
impl SetDoc<JavadocMapping> for ClassMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}
impl AddMember<FieldMapping> for ClassMapping {
	fn add_member(&mut self, member: FieldMapping) {
		self.fields.insert((member.desc.clone(), member.src.clone()), member);
	}
}
impl AddMember<MethodMapping> for ClassMapping {
	fn add_member(&mut self, member: MethodMapping) {
		self.methods.insert((member.desc.clone(), member.src.clone()), member);
	}
}

#[derive(Debug, Clone)]
pub struct FieldMapping {
	pub desc: String,

	pub src: String,
	pub dst: String,

	pub jav: Option<JavadocMapping>,
}

impl FieldMapping {
	pub fn new(desc: String, src: String, dst: String) -> FieldMapping {
		FieldMapping {
			desc, src, dst,
			jav: None,
		}
	}
}
impl ParseEntry for FieldMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(FieldMapping::new(try_read(iter)?, try_read(iter)?, try_read(iter)?))
	}
}
impl SetDoc<JavadocMapping> for FieldMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}

#[derive(Debug, Clone)]
pub struct MethodMapping {
	pub desc: String,

	pub src: String,
	pub dst: String,

	pub jav: Option<JavadocMapping>,

	pub parameters: HashMap<usize, ParameterMapping>,
}

impl MethodMapping {
	pub fn new(desc: String, src: String, dst: String) -> MethodMapping {
		MethodMapping {
			desc, src, dst,
			jav: None,
			parameters: HashMap::new(),
		}
	}
}
impl ParseEntry for MethodMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(MethodMapping::new(try_read(iter)?, try_read(iter)?, try_read(iter)?))
	}
}
impl SetDoc<JavadocMapping> for MethodMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}
impl AddMember<ParameterMapping> for MethodMapping {
	fn add_member(&mut self, member: ParameterMapping) {
		self.parameters.insert(member.index, member);
	}
}

#[derive(Debug, Clone)]
pub struct ParameterMapping {
	pub src: String,
	pub dst: String,

	pub jav: Option<JavadocMapping>,

	pub index: usize,
}

impl ParameterMapping {
	pub fn new(index: usize, src: String, dst: String) -> ParameterMapping {
		ParameterMapping {
			index, src, dst,
			jav: None,
		}
	}
}
impl ParseEntry for ParameterMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(ParameterMapping::new(
			try_read(iter)?.parse()
				.with_context(|| anyhow!("illegal parameter index"))?,
			try_read_optional(iter)?.unwrap_or(String::new()), // TODO: ask space what this means, change to `try_read(&mut iter)` to see it fail
			try_read(iter)?
		))
	}
}
impl SetDoc<JavadocMapping> for ParameterMapping {
	fn set_doc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}

#[derive(Debug, Clone)]
pub struct JavadocMapping {
	pub jav: String,
}

impl ParseEntry for JavadocMapping {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(JavadocMapping {
			jav: try_read(iter)?,
		})
	}
}
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use crate::reader::{parse, ParseEntry, try_read, try_read_optional};
use crate::tiny::{AddMember, RemoveDummy, SetDoc};

pub fn read_file(path: impl AsRef<Path> + Debug) -> Result<Mappings> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read mappings file {path:?}"))
}

pub(crate) fn read(reader: impl Read) -> Result<Mappings> {
	parse::<Mappings, ClassMapping, FieldMapping, MethodMapping, ParameterMapping, JavadocMapping>(reader)
}

#[derive(Debug, Clone)]
pub struct Mappings {
	pub src: String,
	pub dst: String,
	/// Maps from `src` to the mapping
	pub classes: IndexMap<String, ClassMapping>,
}

impl Mappings {
	pub fn write(&self, writer: &mut impl Write) -> Result<()> {
		writeln!(writer, "tiny\t2\t0\t{}\t{}", self.src, self.dst)?;

		for c in self.classes.values() {
			writeln!(writer, "c\t{}\t{}", c.src, c.dst)?;

			if let Some(ref c) = c.jav {
				writeln!(writer, "\tc\t{}", c.jav)?;
			}

			for f in c.fields.values() {
				writeln!(writer, "\tf\t{}\t{}\t{}", f.desc, f.src, f.dst)?;

				if let Some(ref c) = f.jav {
					writeln!(writer, "\t\tc\t{}", c.jav)?;
				}
			}

			for m in c.methods.values() {
				writeln!(writer, "\tm\t{}\t{}\t{}", m.desc, m.src, m.dst)?;

				if let Some(ref c) = m.jav {
					writeln!(writer, "\t\tc\t{}", c.jav)?;
				}

				for p in m.parameters.values() {
					writeln!(writer, "\t\tp\t{}\t{}\t{}", p.index, p.src, p.dst)?;

					if let Some(ref c) = p.jav {
						writeln!(writer, "\t\t\tc\t{}", c.jav)?;
					}
				}
			}
		}

		Ok(())
	}
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
			classes: IndexMap::new(),
		})
	}
}
impl AddMember<ClassMapping> for Mappings {
	fn add_member(&mut self, member: ClassMapping) {
		self.classes.insert(member.src.clone(), member);
	}
}
impl RemoveDummy for Mappings {
	fn remove_dummy(&mut self) -> bool {
		self.classes.remove_dummy()
	}
}

#[derive(Debug, Clone)]
pub struct ClassMapping {
	pub src: String,
	pub dst: String,

	pub jav: Option<JavadocMapping>,

	pub fields: IndexMap<(String, String), FieldMapping>,
	pub methods: IndexMap<(String, String), MethodMapping>,
}

impl ClassMapping {
	pub fn new(src: String, dst: String) -> ClassMapping {
		ClassMapping {
			src, dst,
			jav: None,
			fields: IndexMap::new(),
			methods: IndexMap::new(),
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
impl RemoveDummy for ClassMapping {
	fn remove_dummy(&mut self) -> bool {
		self.jav.remove_dummy()
			& self.fields.remove_dummy()
			& self.methods.remove_dummy()
			&& ( self.dst.starts_with("C_") || self.dst.starts_with("net/minecraft/unmapped/C_") )
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
impl RemoveDummy for FieldMapping {
	fn remove_dummy(&mut self) -> bool {
		self.jav.remove_dummy()
			&& self.dst.starts_with("f_")
	}
}

#[derive(Debug, Clone)]
pub struct MethodMapping {
	pub desc: String,

	pub src: String,
	pub dst: String,

	pub jav: Option<JavadocMapping>,

	pub parameters: IndexMap<usize, ParameterMapping>,
}

impl MethodMapping {
	pub fn new(desc: String, src: String, dst: String) -> MethodMapping {
		MethodMapping {
			desc, src, dst,
			jav: None,
			parameters: IndexMap::new(),
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
impl RemoveDummy for MethodMapping {
	fn remove_dummy(&mut self) -> bool {
		self.jav.remove_dummy()
			& self.parameters.remove_dummy()
			&& ( self.dst.starts_with("m_") || self.dst == "<init>" || self.dst == "<clinit>" )
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
impl RemoveDummy for ParameterMapping {
	fn remove_dummy(&mut self) -> bool {
		self.jav.remove_dummy()
			&& self.dst.starts_with("p_")
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
impl RemoveDummy for JavadocMapping {
	fn remove_dummy(&mut self) -> bool {
		assert!(!self.jav.is_empty(), "{}", self.jav);
		false
	}
}
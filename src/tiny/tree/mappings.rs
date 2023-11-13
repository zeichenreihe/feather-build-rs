use anyhow::Result;
use std::fmt::Write;
use indexmap::IndexMap;
use crate::tiny::{AddMember, ApplyDiff, Member, RemoveDummy};
use crate::tiny::diff::Diffs;
use crate::tiny::tree::ClassMapping;

#[derive(Debug, Clone)]
pub(crate) struct Mappings {
	pub src: String,
	pub dst: String,
	/// Maps from `src` to the mapping
	pub classes: IndexMap<String, ClassMapping>,
}

impl Mappings {
	pub(crate) fn new(src: String, dst: String) -> Mappings {
		Mappings {
			src, dst,
			classes: IndexMap::new(),
		}
	}

	pub(crate) fn write(&self, writer: &mut impl Write) -> Result<()> {
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

impl AddMember<ClassMapping> for Mappings {
	fn add_member(&mut self, member: ClassMapping) {
		self.classes.insert(member.src.clone(), member);
	}
}

impl Member<String, ClassMapping> for Mappings {
	fn get(&self, key: &String) -> Option<&ClassMapping> {
		self.classes.get(key)
	}
}

impl ApplyDiff<Diffs> for Mappings {
	fn apply_diff(&mut self, diff: &Diffs) -> Result<()> {
		self.classes.apply_diff(&diff.classes)
	}
}

impl RemoveDummy for Mappings {
	fn remove_dummy(&mut self) -> bool {
		self.classes.remove_dummy()
	}
}
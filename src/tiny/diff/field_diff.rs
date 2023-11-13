use crate::tiny::diff::JavadocDiff;
use crate::tiny::{Diff, GetJavadoc, GetKey, Op, SetJavadoc};

#[derive(Debug, Clone)]
pub(crate) struct FieldDiff {
	pub(crate) desc: String,

	pub(crate) src: String,
	pub(crate) dst_a: Option<String>,
	pub(crate) dst_b: Option<String>,

	pub(crate) jav: Option<JavadocDiff>,
}

impl FieldDiff {
	pub(crate) fn new(desc: String, src: String, dst_a: Option<String>, dst_b: Option<String>) -> FieldDiff {
		FieldDiff {
			desc, src, dst_a, dst_b,
			jav: None,
		}
	}
}

impl SetJavadoc<JavadocDiff> for FieldDiff {
	fn set_javadoc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocDiff> for FieldDiff {
	fn get_javadoc(&self) -> Option<&JavadocDiff> {
		self.jav.as_ref()
	}
}

impl Diff for FieldDiff {
	fn get_op(&self) -> Op {
		Op::new(&self.dst_a, &self.dst_b)
	}
}

impl GetKey<(String, String)> for FieldDiff {
	fn get_key(&self) -> (String, String) {
		(self.desc.clone(), self.src.clone())
	}
}
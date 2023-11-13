use crate::tiny::diff::JavadocDiff;
use crate::tiny::{Diff, GetJavadoc, GetKey, Op, SetJavadoc};

#[derive(Debug, Clone)]
pub(crate) struct ParameterDiff {
	pub(crate) index: usize,

	pub(crate) src: String,
	pub(crate) dst_a: Option<String>,
	pub(crate) dst_b: Option<String>,

	pub(crate) jav: Option<JavadocDiff>,
}

impl ParameterDiff {
	pub(crate) fn new(index: usize, src: String, dst_a: Option<String>, dst_b: Option<String>) -> ParameterDiff {
		ParameterDiff {
			index, src, dst_a, dst_b,
			jav: None,
		}
	}
}

impl SetJavadoc<JavadocDiff> for ParameterDiff {
	fn set_javadoc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocDiff> for ParameterDiff {
	fn get_javadoc(&self) -> Option<&JavadocDiff> {
		self.jav.as_ref()
	}
}

impl Diff for ParameterDiff {
	fn get_op(&self) -> Op {
		Op::new(&self.dst_a, &self.dst_b)
	}
}

impl GetKey<usize> for ParameterDiff {
	fn get_key(&self) -> usize {
		self.index
	}
}
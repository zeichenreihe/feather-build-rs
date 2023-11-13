use crate::tiny::diff::{JavadocDiff, ParameterDiff};
use crate::tiny::{AddMember, Diff, GetJavadoc, GetKey, Op, SetJavadoc};

#[derive(Debug, Clone)]
pub(crate) struct MethodDiff {
	pub(crate) desc: String,

	pub(crate) src: String,
	pub(crate) dst_a: Option<String>,
	pub(crate) dst_b: Option<String>,

	pub(crate) jav: Option<JavadocDiff>,

	pub(crate) parameters: Vec<ParameterDiff>,
}

impl MethodDiff {
	pub(crate) fn new(desc: String, src: String, dst_a: Option<String>, dst_b: Option<String>) -> MethodDiff {
		MethodDiff {
			desc, src, dst_a, dst_b,
			jav: None,
			parameters: Vec::new(),
		}
	}
}

impl SetJavadoc<JavadocDiff> for MethodDiff {
	fn set_javadoc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocDiff> for MethodDiff {
	fn get_javadoc(&self) -> Option<&JavadocDiff> {
		self.jav.as_ref()
	}
}

impl AddMember<ParameterDiff> for MethodDiff {
	fn add_member(&mut self, member: ParameterDiff) {
		self.parameters.push(member)
	}
}

impl Diff for MethodDiff {
	fn get_op(&self) -> Op {
		Op::new(&self.dst_a, &self.dst_b)
	}
}

impl GetKey<(String, String)> for MethodDiff {
	fn get_key(&self) -> (String, String) {
		(self.desc.clone(), self.src.clone())
	}
}
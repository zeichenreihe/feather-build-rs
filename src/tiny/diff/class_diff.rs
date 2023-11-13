use crate::tiny::diff::{FieldDiff, JavadocDiff, MethodDiff};
use crate::tiny::{AddMember, Diff, GetJavadoc, GetKey, Op, SetJavadoc};

#[derive(Debug, Clone)]
pub(crate) struct ClassDiff {
	pub(crate) src: String,
	pub(crate) dst_a: Option<String>,
	pub(crate) dst_b: Option<String>,

	pub(crate) jav: Option<JavadocDiff>,

	pub(crate) fields: Vec<FieldDiff>,
	pub(crate) methods: Vec<MethodDiff>,
}

impl ClassDiff {
	pub(crate) fn new(src: String, dst_a: Option<String>, dst_b: Option<String>) -> ClassDiff {
		ClassDiff {
			src, dst_a, dst_b,
			jav: None,
			fields: Vec::new(),
			methods: Vec::new(),
		}
	}
}

impl SetJavadoc<JavadocDiff> for ClassDiff {
	fn set_javadoc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocDiff> for ClassDiff {
	fn get_javadoc(&self) -> Option<&JavadocDiff> {
		self.jav.as_ref()
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

impl Diff for ClassDiff {
	fn get_op(&self) -> Op {
		Op::new(&self.dst_a, &self.dst_b)
	}
}

impl GetKey<String> for ClassDiff {
	fn get_key(&self) -> String {
		self.src.clone()
	}
}

use crate::tiny::{Diff, Op};

#[derive(Debug, Clone)]
pub(crate) struct JavadocDiff {
	pub(crate) jav_a: Option<String>,
	pub(crate) jav_b: Option<String>,
}

impl JavadocDiff {
	pub(crate) fn new(jav_a: Option<String>, jav_b: Option<String>) -> JavadocDiff {
		JavadocDiff {
			jav_a, jav_b,
		}
	}
}

impl Diff for JavadocDiff {
	fn get_op(&self) -> Op {
		Op::new(&self.jav_a, &self.jav_b)
	}
}

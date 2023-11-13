use crate::tiny::AddMember;
use crate::tiny::diff::ClassDiff;

#[derive(Debug, Clone)]
pub(crate) struct Diffs {
	pub(crate) classes: Vec<ClassDiff>,
}

impl Diffs {
	pub(crate) fn new() -> Diffs {
		Diffs {
			classes: Vec::new(),
		}
	}
}

impl AddMember<ClassDiff> for Diffs {
	fn add_member(&mut self, member: ClassDiff) {
		self.classes.push(member)
	}
}
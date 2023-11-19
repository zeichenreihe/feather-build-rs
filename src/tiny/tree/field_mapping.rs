#![allow(deprecated)]
use crate::tiny::{ApplyDiff, GetJavadoc, RemoveDummy, SetJavadoc};
use crate::tiny::diff::FieldDiff;
use crate::tiny::tree::JavadocMapping;

#[derive(Debug, Clone)]
pub(crate) struct FieldMapping {
	pub(crate) desc: String,

	pub(crate) src: String,
	pub(crate) dst: String,

	pub(crate) jav: Option<JavadocMapping>,
}

impl FieldMapping {
	pub(crate) fn new(desc: String, src: String, dst: String) -> FieldMapping {
		FieldMapping {
			desc, src, dst,
			jav: None,
		}
	}
}

impl SetJavadoc<JavadocMapping> for FieldMapping {
	fn set_javadoc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocMapping> for FieldMapping {
	fn get_javadoc(&self) -> Option<&JavadocMapping> {
		self.jav.as_ref()
	}
}

impl ApplyDiff<FieldDiff> for FieldMapping {
	fn apply_diff(&mut self, diff: &FieldDiff) -> anyhow::Result<()> {
		// TODO: apply on other fields

		self.jav.apply_diff(&diff.jav)?;

		Ok(())
	}
}

impl RemoveDummy for FieldMapping {
	fn remove_dummy(&mut self) -> bool {
		let jav = self.jav.remove_dummy();

		jav &&
			self.dst.starts_with("f_")
	}
}
#![allow(deprecated)]
use crate::tiny::{ApplyDiff, GetJavadoc, RemoveDummy, SetJavadoc};
use crate::tiny::diff::ParameterDiff;
use crate::tiny::tree::JavadocMapping;

#[derive(Debug, Clone)]
pub(crate) struct ParameterMapping {
	pub(crate) src: String,
	pub(crate) dst: String,

	pub(crate) jav: Option<JavadocMapping>,

	pub(crate) index: usize,
}

impl ParameterMapping {
	pub(crate) fn new(index: usize, src: String, dst: String) -> ParameterMapping {
		ParameterMapping {
			index, src, dst,
			jav: None,
		}
	}
}


impl SetJavadoc<JavadocMapping> for ParameterMapping {
	fn set_javadoc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocMapping> for ParameterMapping {
	fn get_javadoc(&self) -> Option<&JavadocMapping> {
		self.jav.as_ref()
	}
}

impl ApplyDiff<ParameterDiff> for ParameterMapping {
	fn apply_diff(&mut self, diff: &ParameterDiff) -> anyhow::Result<()> {
		// TODO: apply on other fields

		self.jav.apply_diff(&diff.jav)?;

		Ok(())
	}
}

impl RemoveDummy for ParameterMapping {
	fn remove_dummy(&mut self) -> bool {
		let jav = self.jav.remove_dummy();

		jav &&
			self.dst.starts_with("p_")
	}
}
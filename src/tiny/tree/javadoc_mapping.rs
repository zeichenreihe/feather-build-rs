#![allow(deprecated)]
use crate::tiny::{ApplyDiff, RemoveDummy};
use crate::tiny::diff::JavadocDiff;

#[derive(Debug, Clone)]
pub(crate) struct JavadocMapping {
	pub(crate) jav: String,
}

impl JavadocMapping {
	pub(crate) fn new(jav: String) -> JavadocMapping {
		JavadocMapping {
			jav
		}
	}
}

impl ApplyDiff<JavadocDiff> for JavadocMapping {
	fn apply_diff(&mut self, diff: &JavadocDiff) -> anyhow::Result<()> {
		// TODO: apply on other fields

		Ok(())
	}
}

impl RemoveDummy for JavadocMapping {
	fn remove_dummy(&mut self) -> bool {
		assert!(!self.jav.is_empty(), "{}", self.jav);
		false
	}
}
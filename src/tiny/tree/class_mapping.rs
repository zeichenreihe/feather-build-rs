#![allow(deprecated)]
use anyhow::Result;
use indexmap::IndexMap;
use crate::tiny::{AddMember, ApplyDiff, GetJavadoc, Member, RemoveDummy, SetJavadoc};
use crate::tiny::diff::ClassDiff;
use crate::tiny::tree::{FieldMapping, JavadocMapping, MethodMapping};

#[derive(Debug, Clone)]
pub(crate) struct ClassMapping {
	pub(crate) src: String,
	pub(crate) dst: String,

	pub(crate) jav: Option<JavadocMapping>,

	pub(crate) fields: IndexMap<(String, String), FieldMapping>,
	pub(crate) methods: IndexMap<(String, String), MethodMapping>,
}

impl ClassMapping {
	pub(crate) fn new(src: String, dst: String) -> ClassMapping {
		ClassMapping {
			src, dst,
			jav: None,
			fields: IndexMap::new(),
			methods: IndexMap::new(),
		}
	}
}

impl SetJavadoc<JavadocMapping> for ClassMapping {
	fn set_javadoc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocMapping> for ClassMapping {
	fn get_javadoc(&self) -> Option<&JavadocMapping> {
		self.jav.as_ref()
	}
}

impl AddMember<FieldMapping> for ClassMapping {
	fn add_member(&mut self, member: FieldMapping) {
		self.fields.insert((member.desc.clone(), member.src.clone()), member);
	}
}

impl AddMember<MethodMapping> for ClassMapping {
	fn add_member(&mut self, member: MethodMapping) {
		self.methods.insert((member.desc.clone(), member.src.clone()), member);
	}
}

impl Member<(String, String), FieldMapping> for ClassMapping {
	fn get(&self, key: &(String, String)) -> Option<&FieldMapping> {
		self.fields.get(key)
	}
}

impl Member<(String, String), MethodMapping> for ClassMapping {
	fn get(&self, key: &(String, String)) -> Option<&MethodMapping> {
		self.methods.get(key)
	}
}

impl ApplyDiff<ClassDiff> for ClassMapping {
	fn apply_diff(&mut self, diff: &ClassDiff) -> Result<()> {
		// TODO: apply on other fields

		self.fields.apply_diff(&diff.fields)?;
		self.methods.apply_diff(&diff.methods)?;

		self.jav.apply_diff(&diff.jav)?;

		Ok(())
	}
}

impl RemoveDummy for ClassMapping {
	fn remove_dummy(&mut self) -> bool {
		let jav = self.jav.remove_dummy();
		let fields = self.fields.remove_dummy();
		let methods = self.methods.remove_dummy();

		jav && fields && methods && (
			self.dst.starts_with("C_") ||
			self.dst.starts_with("net/minecraft/unmapped/C_")
		)
	}
}
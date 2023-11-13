use indexmap::IndexMap;
use crate::tiny::{AddMember, ApplyDiff, GetJavadoc, Member, RemoveDummy, SetJavadoc};
use crate::tiny::diff::MethodDiff;
use crate::tiny::tree::{JavadocMapping, ParameterMapping};

#[derive(Debug, Clone)]
pub(crate) struct MethodMapping {
	pub(crate) desc: String,

	pub(crate) src: String,
	pub(crate) dst: String,

	pub(crate) jav: Option<JavadocMapping>,

	pub(crate) parameters: IndexMap<usize, ParameterMapping>,
}

impl MethodMapping {
	pub(crate) fn new(desc: String, src: String, dst: String) -> MethodMapping {
		MethodMapping {
			desc, src, dst,
			jav: None,
			parameters: IndexMap::new(),
		}
	}
}

impl SetJavadoc<JavadocMapping> for MethodMapping {
	fn set_javadoc(&mut self, doc: JavadocMapping) {
		self.jav = Some(doc);
	}
}

impl GetJavadoc<JavadocMapping> for MethodMapping {
	fn get_javadoc(&self) -> Option<&JavadocMapping> {
		self.jav.as_ref()
	}
}

impl AddMember<ParameterMapping> for MethodMapping {
	fn add_member(&mut self, member: ParameterMapping) {
		self.parameters.insert(member.index, member);
	}
}

impl Member<usize, ParameterMapping> for MethodMapping {
	fn get(&self, key: &usize) -> Option<&ParameterMapping> {
		self.parameters.get(key)
	}
}

impl ApplyDiff<MethodDiff> for MethodMapping {
	fn apply_diff(&mut self, diff: &MethodDiff) -> anyhow::Result<()> {
		// TODO: apply on other fields

		self.parameters.apply_diff(&diff.parameters)?;

		self.jav.apply_diff(&diff.jav)?;

		Ok(())
	}
}

impl RemoveDummy for MethodMapping {
	fn remove_dummy(&mut self) -> bool {
		let jav = self.jav.remove_dummy();
		let parameters = self.parameters.remove_dummy();

		jav && parameters && (
			self.dst.starts_with("m_") ||
			self.dst == "<init>" ||
			self.dst == "<clinit>"
		)
	}
}
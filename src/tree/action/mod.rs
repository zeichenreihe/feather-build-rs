pub(crate) mod apply_diff;
pub(crate) mod extend_inner_class_names;
pub(crate) mod merge;
pub(crate) mod remapper;
pub(crate) mod remove_dummy;
pub(crate) mod reorder;

use anyhow::Result;
use crate::tree::mappings::Mappings;
use crate::tree::names::Namespace;

impl<const N: usize> Mappings<N> {
	pub(crate) fn rename_namespaces(mut self, from: [&str; N], to: [&str; N]) -> Result<Self> {
		self.info.namespaces.change_names(from, to)?;
		Ok(self)
	}

	pub(crate) fn get_namespace(&self, name: &str) -> Result<Namespace<N>> {
		self.info.namespaces.get_namespace(name)
	}
}

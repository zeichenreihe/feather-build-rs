pub(crate) mod apply_diff;
pub(crate) mod diff_mappings;
pub(crate) mod extend_inner_class_names;
pub(crate) mod insert_dummy;
pub(crate) mod merge;
pub(crate) mod remove_dummy;
pub(crate) mod reorder;

use anyhow::Result;
use crate::tree::mappings::Mappings;
use crate::tree::names::Namespace;

impl<const N: usize, Ns> Mappings<N, Ns> {
	pub fn rename_namespaces(mut self, from: [&str; N], to: [&str; N]) -> Result<Mappings<N, Ns>> {
		self.info.namespaces.change_names(from, to)?;
		Ok(self)
	}

	pub fn get_namespace(&self, name: &str) -> Result<Namespace<N>> {
		self.info.namespaces.get_namespace(name)
	}
}

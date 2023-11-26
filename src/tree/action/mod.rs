mod reorder;
mod merge;
mod remapper;
mod remove_dummy;
mod apply_diff;

use anyhow::{bail, Result};
use crate::tree::mappings::Mappings;
use crate::tree::Namespace;

impl<const N: usize> Mappings<N> {
	pub(crate) fn rename_namespaces(&mut self, from: [&str; N], to: [&str; N]) -> Result<()> {
		if self.info.namespaces != from {
			bail!("Cannot rename namespaces {:?} to {to:?}: expected {from:?}", self.info.namespaces);
		}

		let to = to.map(|x| String::from(x));
		self.info.namespaces = to;
		Ok(())
	}

	pub(crate) fn get_namespace(&self, name: &str) -> Result<Namespace<N>> {
		for (i, namespace) in self.info.namespaces.iter().enumerate() {
			if namespace == name {
				return Ok(Namespace(i));
			}
		}
		bail!("Cannot find namespace with name {name:?}, only got {:?}", self.info.namespaces);
	}
}

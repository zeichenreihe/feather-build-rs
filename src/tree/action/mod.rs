mod reorder;
mod merge;
mod remapper;
mod remove_dummy;

use anyhow::{bail, Result};
use crate::tree::mappings::Mappings;
use crate::tree::Namespace;

impl<const N: usize> Mappings<N> {
	pub(crate) fn get_namespace(&self, name: &str) -> Result<Namespace<N>> {
		for (i, namespace) in self.info.namespaces.iter().enumerate() {
			if namespace == name {
				return Ok(Namespace(i));
			}
		}
		bail!("Cannot find namespace with name {name:?}, only got {:?}", self.info.namespaces);
	}
}

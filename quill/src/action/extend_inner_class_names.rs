use anyhow::{anyhow, Context, Result};
use java_string::{JavaStr, JavaString};
use duke::tree::class::{ClassName, ClassNameSlice};
use crate::tree::mappings::{ClassMapping, ClassNowodeMapping, Mappings};
use crate::tree::names::{Names, Namespace};

fn map<const N: usize>(mappings: &Mappings<N>, namespace: Namespace<N>, name: &ClassNameSlice, mapped: &ClassNameSlice) -> Result<ClassName> {
	Ok(if let Some(parent) = name.get_inner_class_parent() {
		let mapped_parent = mappings.get_class_name(parent, namespace)?;

		let result = map(mappings, namespace, parent, mapped_parent)?;

		ClassName::from_inner_class(result, mapped)
	} else {
		mapped.to_owned()
	})
}

impl<const N: usize> Names<N, ClassName> {
	fn extend_inner_class_name(&self, mappings: &Mappings<N>, namespace: Namespace<N>) -> Result<Names<N, ClassName>> {
		let mut names = self.clone();

		if let (src, Some(b)) = names.get_mut_with_src(namespace)? {
			let src = src.with_context(|| anyhow!("expected to have class name for first namespace, got {self:?}"))?;
			*b = map(mappings, namespace, src, b)?;
		}

		Ok(names)
	}
}

impl<const N: usize> Mappings<N> {
	// TODO: document this
	// this changes the names in the namespace "named" for inner classes:
	// say you have mappings A -> a, A$B -> b, A$B$C -> c (which ofc is
	// "wrong", since this moves inner classes around...), this produces
	// the mappings A -> a, A$B -> a$b, A$B$C -> a$b$c, fixing this
	// inconsistency
	pub fn extend_inner_class_names(&self, namespace: &str) -> Result<Mappings<N>> {
		let namespace = self.get_namespace(namespace)?;
		Ok(Mappings {
			info: self.info.clone(),
			classes: self.classes.iter()
				.map(|(key, c)| Ok((key.clone(), ClassNowodeMapping {
					info: ClassMapping {
						names: c.info.names.extend_inner_class_name(self, namespace)?
					},
					fields: c.fields.clone(),
					methods: c.methods.clone(),
					javadoc: c.javadoc.clone(),
				})))
				.collect::<Result<_>>()?,
			javadoc: self.javadoc.clone(),
		})
	}
}

#[cfg(test)]
mod testing {
	// TODO: test internals
}
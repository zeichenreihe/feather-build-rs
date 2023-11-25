use anyhow::{anyhow, bail, Context, Result};
use crate::tree::mappings::{ClassKey, Mappings};
use crate::tree::Namespace;

impl<const N: usize> Mappings<N> {
	pub(crate) fn remapper(&self, from: Namespace<N>, to: Namespace<N>) -> Result<Remapper<'_, N>> {
		if N < 2 {
			bail!("Cannot create remapper: at least two namespaces are required, got {N}");
		}
		if from == to {
			bail!("Cannot create remapper with source namespace {} being equal to the target namespace {}, consider using the mapping directly", from.0, to.0);
		}

		if from.0 != 0 {
			bail!("Cannot use a combination other than from = 0 for now, got from = {}", from.0);
		}

		Ok(Remapper { from: from.0, to, mappings: &self })
	}
}

#[derive(Debug)]
pub(crate) struct Remapper<'a, const N: usize> {
	from: usize, // in range 0..N
	to: Namespace<N>,
	mappings: &'a Mappings<N>,
}

impl<'a, const N: usize> Remapper<'a, N> {
	pub(crate) fn remap_desc(&self, desc: &str) -> Result<String> {
		let mut s = String::new();

		let mut iter = desc.char_indices();

		while let Some((_, ch)) = iter.next() {
			s.push(ch);

			if ch == 'L' {
				let mut class_name = String::new();
				while let Some((_, ch)) = iter.next() {
					class_name.push(ch);
					if ch == ';' {
						break;
					}
				}
				if class_name.pop() != Some(';') {
					bail!("Descriptor {desc:?} has a missing semicolon somewhere");
				}

				let new_class_name = {
					let key = ClassKey { src: class_name.to_owned() };

					match self.mappings.classes.get(&key) {
						None => &class_name,
						Some(class) => {
							&class.info.names.get(self.to)
								.with_context(|| anyhow!("No name for class {key:?} in namespace {:?}", self.to))?
						}
					}
				};

				s.push_str(&new_class_name);
				s.push(';');
			}
		}

		Ok(s)
	}
}

#[cfg(test)]
mod testing {
	#[test]
	fn remap() {
		// TODO: write test
	}
}
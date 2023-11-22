use anyhow::{bail, Result};
use crate::tree::mappings::{ClassKey, Mappings};

#[derive(Debug)]
pub(crate) struct Remapper<'a, const N: usize> {
	from: usize, // in range 0..N
	to: usize, // in range 0..N
	mappings: &'a Mappings<N>,
}

impl<'a, const N: usize> Remapper<'a, N> {
	pub(crate) fn new(mappings: &'a Mappings<N>, from: usize, to: usize) -> Result<Remapper<'a, N>> {
		if N < 2 {
			bail!("Cannot create remapper: at least two namespaces are required, got {N}");
		}
		if from == to {
			bail!("Cannot create remapper with source namespace {from} being equal to the target namespace {to}, consider using the mapping directly");
		}
		Ok(Remapper { from, to, mappings })
	}

	pub(crate) fn remap_desc(&self, desc: &str) -> Result<String> {
		if self.from != 0 || self.to != 1 {
			bail!("Cannot use a combination other than from = 0 and to = 1 for now, got from = {} and to = {}", self.from, self.to);
		}

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

					self.mappings.classes.get(&key)
						.map_or(&class_name, |class| &class.info.names[1])
				};

				s.push_str(&new_class_name);
				s.push(';');
			}
		}

		Ok(s)
	}
}
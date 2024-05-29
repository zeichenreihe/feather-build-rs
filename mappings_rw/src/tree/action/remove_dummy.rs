
use anyhow::Result;
use crate::tree::mappings::Mappings;

impl<const N: usize> Mappings<N> {
	/// Removed so called "dummy" mappings. Whether or not a mapping is considered a dummy mapping only depends on the mapping
	/// in the namespace given.
	/// # Removal Rules
	/// - a class mappings is removed if in the given namespace its name starts with `C_` or `net/minecraft/unmapped/C_`, and
	///   there are no members, i.e. fields, methods, javadoc, left.
	/// - a field mapping is removed if in the given namespace its name starts with `f_`, and it doesn't have any javadoc.
	/// - a method mapping is removed if in the given namespace its name starts with `m_`, or its name is equal to either
	///   `<init>` or `<clinit>`, and it doesn't have any members, i.e. javadoc or parameter mappings.
	/// - a parameter mapping is removed if its name starts with `p_` and it doesn't have any javadoc.
	pub fn remove_dummy(mut self, namespace: &str) -> Result<Self> {
		let namespace = self.get_namespace(namespace)?;

		self.classes.retain(|_, v| {
			v.fields.retain(|_, v| {
				v.javadoc.is_some() ||
					!v.info.names[namespace].as_ref().is_some_and(|x| x.as_str().starts_with("f_"))
			});

			v.methods.retain(|_, v| {
				v.parameters.retain(|_, v| {
					v.javadoc.is_some() ||
						!v.info.names[namespace].as_ref().is_some_and(|x| x.as_str().starts_with("p_"))
				});

				v.javadoc.is_some() ||
					!v.parameters.is_empty() ||
					!v.info.names[namespace].as_ref().is_some_and(|x|
						x.as_str().starts_with("m_") ||
							x == "<init>" ||
							x == "<clinit>"
					)
			});

			v.javadoc.is_some() ||
				!v.fields.is_empty() ||
				!v.methods.is_empty() ||
				!v.info.names[namespace].as_ref().is_some_and(|x| {
					x.as_str().starts_with("C_") ||
						x.as_str().starts_with("net/minecraft/unmapped/C_")
				})
		});

		Ok(self)
	}
}

#[cfg(test)]
mod testing {
	use anyhow::Result;
	use pretty_assertions::assert_eq;
	use crate::tree::mappings::Mappings;

	#[test]
	fn remove_dummy() -> Result<()> {
		let input = include_str!("test/remove_dummy_input.tiny");
		let expected = include_str!("test/remove_dummy_output.tiny");

		let input: Mappings<2> = crate::tiny_v2::read(input.as_bytes())?;

		let output = input.remove_dummy("namespaceB")?;

		let actual = crate::tiny_v2::write_string(&output)?;

		assert_eq!(actual, expected, "left: actual, right: expected");

		Ok(())
	}
}
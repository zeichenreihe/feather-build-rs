
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
	pub(crate) fn remove_dummy(&mut self, namespace: &str) -> Result<()> {
		let namespace = self.get_namespace(namespace)?.0;

		self.classes.retain(|_, v| !{
			v.fields.retain(|_, v| !{
				v.javadoc.is_none() && v.info.names[namespace].starts_with("f_")
			});

			v.methods.retain(|_, v| !{
				v.parameters.retain(|_, v| !{
					v.javadoc.is_none() && v.info.names[namespace].starts_with("p_")
				});

				v.javadoc.is_none() && v.parameters.is_empty() && (
					v.info.names[namespace].starts_with("m_") ||
						v.info.names[namespace] == "<init>" ||
						v.info.names[namespace] == "<clinit>"
				)
			});

			v.javadoc.is_none() && v.fields.is_empty() && v.methods.is_empty() && (
				v.info.names[namespace].starts_with("C_") ||
					v.info.names[namespace].starts_with("net/minecraft/unmapped/C_")
			)
		});

		Ok(())
	}
}

#[cfg(test)]
mod testing {
	#[test]
	fn remove_dummy() {
		// TODO: write test
	}
}
use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use crate::tree::action::remapper::ARemapper;
use crate::tree::names::Namespace;
use crate::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, MappingInfo, Mappings, MethodMapping, MethodNowodeMapping, ParameterMapping, ParameterNowodeMapping};
use crate::tree::NodeInfo;

impl<const N: usize> Mappings<N> {
	/// Reorders the namespaces to the given order.
	/// # Example
	/// If you call this on a mapping like
	/// ```txt,ignore
	/// c	A	B	C
	/// 	m	(LA;)V	a	b	c
	/// 	f	LA;	a	b	c
	/// ```
	/// with the given namespaces being `["C", "B", "A"]`, you get:
	/// ```txt,ignore
	/// c	C	B	A
	/// 	m	(LC;)V	c	b	a
	/// 	f	LC;	c	b	a
	/// ```
	///
	// TODO: finish this test when we move this stuff to own crate (lib)
	/// ```
	/// let input = """
	/// tiny	2	0
	/// c	C	B	A
	/// 	m	(LC;)V	c	b	a
	/// 	f	LC;	c	b	a""";
	/// println!(input);
	/// panic!();
	/// ```
	pub fn reorder(&self, namespaces: [&str; N]) -> Result<Mappings<N>> {
		// new CommandReorderTinyV2().run([self, return, "intermediary", "official"])

		// at each position we have the namespace (and therefore the old index) to look to find the name
		let mut table = [Namespace::new(0)?; N];
		for i in 0..N {
			table[i] = self.get_namespace(namespaces[i])?;
		}

		let remapper = self.remapper_a(Namespace::new(0)?, table[0])?;

		let mut m = Mappings::new(MappingInfo {
			namespaces: self.info.namespaces.reorder(table),
		});

		for class in self.classes.values() {
			let mapping = ClassMapping {
				names: class.info.names.reorder(table)
					.with_context(|| anyhow!("failed to reorder names for class {:?}", class.info.names))?,
			};

			let mut c = ClassNowodeMapping {
				info: mapping,
				javadoc: class.javadoc.clone(),
				fields: IndexMap::new(),
				methods: IndexMap::new(),
			};

			for field in class.fields.values() {
				let mapping = FieldMapping {
					desc: remapper.map_field_desc(&field.info.desc)?,
					names: field.info.names.reorder(table)
						.with_context(|| anyhow!("failed to reorder names for field {:?} in class {:?}", field.info.names, class.info.names))?,
				};

				let f = FieldNowodeMapping {
					info: mapping,
					javadoc: field.javadoc.clone(),
				};

				c.add_field(f)?;
			}

			for method in class.methods.values() {
				let mapping = MethodMapping {
					desc: remapper.map_method_desc(&method.info.desc)?,
					names: method.info.names.reorder(table)
						.with_context(|| anyhow!("failed to reorder names for method {:?} in class {:?}", method.info.names, class.info.names))?,
				};

				let mut m = MethodNowodeMapping {
					info: mapping,
					javadoc: method.javadoc.clone(),
					parameters: IndexMap::new(),
				};

				for parameter in method.parameters.values() {
					let mapping = ParameterMapping {
						index: parameter.info.index,
						names: parameter.info.names.reorder(table)
							.with_context(|| anyhow!("failed to reorder names for parameter {:?} in method {:?} in class {:?}", parameter.info.names, method.info, class.info.names))?,
					};

					let p = ParameterNowodeMapping {
						info: mapping,
						javadoc: parameter.javadoc.clone(),
					};

					m.add_parameter(p)?;
				}

				c.add_method(m)?;
			}

			m.add_class(c)?;
		}

		Ok(m)
	}
}

#[cfg(test)]
mod testing {
	use anyhow::Result;
	use pretty_assertions::assert_eq;

	#[test]
	fn reorder() -> Result<()> {
		let input = include_str!("test/reorder_input.tiny");
		let expected = include_str!("test/reorder_output.tiny");

		let input = crate::reader::tiny_v2::read(input.as_bytes())?;

		let output = input.reorder(["namespaceB", "namespaceA"])?;

		let actual = crate::writer::tiny_v2::write_string(&output)?;

		assert_eq!(actual, expected, "left: actual, right: expected");

		Ok(())

	}
}
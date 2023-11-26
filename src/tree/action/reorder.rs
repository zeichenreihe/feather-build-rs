use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use crate::tree::{ClassNowode, FieldNowode, MethodNowode, Names, Namespace, ParameterNowode};
use crate::tree::mappings::{ClassMapping, FieldMapping, MappingInfo, Mappings, MethodMapping, ParameterMapping};
fn reorder_array<T: Clone, const N: usize>(arr: &[T; N], table: [Namespace<N>; N]) -> [T; N] {
	table.clone().map(|namespace| arr[namespace.0].clone())
}
fn reorder_names<const N: usize>(names: &Names<N>, table: [Namespace<N>; N]) -> Names<N> {
	let mut arr: &[Option<String>; N] = names.into();
	reorder_array(arr, table).into()
}

impl<const N: usize> Mappings<N> {
	/// Reorders the namespaces to the given order.
	/// # Example
	/// If you call this on a mapping like
	/// ```
	/// c	A	B	C
	/// 	m	(LA;)V	a	b	c
	/// 	f	LA;	a	b	c
	/// ```
	/// with the given namespaces being `["C", "B", "A"]`, you get:
	/// ```
	/// c	C	B	A
	/// 	m	(LC;)V	c	b	a
	/// 	f	LC;	c	b	a
	/// ```
	pub(crate) fn reorder(&self, namespaces: [&str; N]) -> Result<Mappings<N>> {
		// new CommandReorderTinyV2().run([self, return, "intermediary", "official"])

		// at each position we have the namespace (and therefore the old index) to look to find the name
		let mut table = [Namespace(0); N];
		for i in 0..N {
			table[i] = self.get_namespace(namespaces[i])?;
		}

		let remapper = self.remapper(Namespace(0), table[0])?;

		let mut m = Mappings::new(MappingInfo {
			namespaces: reorder_array(&self.info.namespaces, table),
		});

		for class in self.classes.values() {
			let mapping = ClassMapping {
				names: reorder_names(&class.info.names, table),
			};
			let key = mapping.get_key()
				.with_context(|| anyhow!("Failed to invert names for class {:?}", class.info.names))?;

			let mut c = ClassNowode {
				info: mapping,
				javadoc: class.javadoc.clone(),
				fields: IndexMap::new(),
				methods: IndexMap::new(),
			};

			for field in class.fields.values() {
				let mapping = FieldMapping {
					desc: remapper.remap_desc(&field.info.desc)?,
					names: reorder_names(&field.info.names, table),
				};
				let key = mapping.get_key()
					.with_context(|| anyhow!("Failed to invert names for field in class {:?}", class.info.names))?;

				let f = FieldNowode {
					info: mapping,
					javadoc: field.javadoc.clone(),
				};

				c.add_field(key, f)?;
			}

			for method in class.methods.values() {
				let mapping = MethodMapping {
					desc: remapper.remap_desc(&method.info.desc)?,
					names: reorder_names(&method.info.names, table),
				};
				let key = mapping.get_key()
					.with_context(|| anyhow!("Failed to invert names for methods in class {:?}", class.info.names))?;

				let mut m = MethodNowode {
					info: mapping,
					javadoc: method.javadoc.clone(),
					parameters: IndexMap::new(),
				};

				for parameter in method.parameters.values() {
					let mapping = ParameterMapping {
						index: parameter.info.index,
						names: reorder_names(&parameter.info.names, table),
					};
					let key = mapping.get_key()
						.with_context(|| anyhow!("Failed to invert names for parameters in class {:?} method {:?}", class.info.names, method.info))?;

					let p = ParameterNowode {
						info: mapping,
						javadoc: parameter.javadoc.clone(),
					};

					m.add_parameter(key, p)?;
				}

				c.add_method(key, m)?;
			}

			m.add_class(key, c)?;
		}

		Ok(m)
	}
}

#[cfg(test)]
mod testing {
	#[test]
	fn reorder() {
		let input = include_str!("test/reorder_input.tiny");
		let expected = include_str!("test/reorder_output.tiny");

		let input = crate::reader::tiny_v2::read(input.as_bytes()).unwrap();

		let actual = input.reorder(["namespaceB", "namespaceA"]).unwrap();

		let actual = crate::writer::tiny_v2::write_string(&actual).unwrap();

		assert_eq!(actual, expected, "\nactual: {actual}\nexpected: {expected}");

	}
}
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use crate::tree::{ClassNowode, FieldNowode, MethodNowode, Names, Namespace, ParameterNowode};
use crate::tree::mappings::{ClassMapping, FieldMapping, MappingInfo, Mappings, MethodMapping, ParameterMapping};

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

		//TODO: rewrite so that it can actually "reorder" any given namespaces,
		// also ensure that list doesn't have duplicates in it;
		// and also update the test cases

		let namespaces = self.get_namespaces(namespaces)?;

		let namespace = namespaces[0];

		if namespace.0 != 1 {
			bail!("can only invert with namespace 1 currently (remapper can't do anything else)!");
		}

		fn invert<T: Clone, const N: usize>(arr: &[T; N], namespace: usize) -> [T; N] {
			let mut arr = arr.clone();
			arr.swap(0, namespace); // namespace is checked in outer function
			arr
		}
		fn invert_names<const N: usize>(names: &Names<N>, namespace: usize) -> Result<Names<N>> {
			let mut arr: [Option<String>; N] = names.clone().into();
			arr.swap(0, namespace);
			arr.try_into()
				.with_context(|| anyhow!("Cannot invert names {names:?}, as when inverted there's no source namespace"))
		}

		let remapper = self.remapper(Namespace(0), namespace)?;
		let namespace = namespace.0;

		let mut m = Mappings::new(MappingInfo {
			namespaces: invert(&self.info.namespaces, namespace),
		});

		for class in self.classes.values() {
			let mapping = ClassMapping {
				names: invert_names(&class.info.names, namespace)?,
			};
			let key = mapping.get_key();

			let mut c = ClassNowode {
				info: mapping,
				javadoc: class.javadoc.clone(),
				fields: IndexMap::new(),
				methods: IndexMap::new(),
			};

			for field in class.fields.values() {
				let mapping = FieldMapping {
					desc: remapper.remap_desc(&field.info.desc)?,
					names: invert_names(&field.info.names, namespace)?,
				};
				let key = mapping.get_key();

				let f = FieldNowode {
					info: mapping,
					javadoc: field.javadoc.clone(),
				};

				c.add_field(key, f)?;
			}

			for method in class.methods.values() {
				let mapping = MethodMapping {
					desc: remapper.remap_desc(&method.info.desc)?,
					names: invert_names(&method.info.names, namespace)?,
				};
				let key = mapping.get_key();

				let mut m = MethodNowode {
					info: mapping,
					javadoc: method.javadoc.clone(),
					parameters: IndexMap::new(),
				};

				for parameter in method.parameters.values() {
					let mapping = ParameterMapping {
						index: parameter.info.index,
						names: invert_names(&parameter.info.names, namespace)?,
					};
					let key = mapping.get_key();

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
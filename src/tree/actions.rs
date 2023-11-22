use anyhow::{bail, Result};
use indexmap::IndexMap;
use crate::tree::{ClassNowode, FieldNowode, MethodNowode, Namespace, ParameterNowode};
use crate::tree::mappings::{ClassMapping, FieldMapping, MappingInfo, Mappings, MethodMapping, ParameterMapping};
use crate::tree::remapper::Remapper;

impl<const N: usize> Mappings<N> {
	pub(crate) fn get_namespace(&self, name: &str) -> Result<Namespace<N>> {
		for (i, namespace) in self.info.namespaces.iter().enumerate() {
			if namespace == name {
				return Ok(Namespace(i));
			}
		}
		bail!("Cannot find namespace with name {name:?}, only got {:?}", self.info.namespaces);
	}

	pub(crate) fn remapper(&self, from: Namespace<N>, to: Namespace<N>) -> Result<Remapper<'_, N>> {
		Remapper::new(&self, from.0, to.0)
	}

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

	/// # old description: TODO: fix this "oldness"
	/// Inverts a mapping with respect to the given namespace. That means, the given namespace and the "source" or
	/// zero namespace are swapped.
	/// # Example
	/// If you call this on a mapping like
	/// ```
	/// c	A	B	C
	/// 	m	(LA;)V	a	b	c
	/// 	f	LA;	a	b	c
	/// ```
	/// with the given namespace being `2`, or the `C` namespace, then you'll get a result like
	/// ```
	/// c	C	B	A
	/// 	m	(LC;)V	c	b	a
	/// 	f	LC;	c	b	a
	/// ```
	pub(crate) fn reorder(&self, namespaces: [&str; N]) -> Result<Mappings<N>> {
		//TODO: rewrite so that it can actually "reorder" any given namespaces,
		// also ensure that list doesn't have duplicates in it;
		// and also update the test cases; probably even split up this module into submodules, each with just one impl {}

		// new CommandReorderTinyV2().run([self, return, "intermediary", "official"])

		let namespace = namespaces[0];
		let namespace = self.get_namespace(namespace)?;

		if namespace.0 != 1 {
			bail!("can only invert with namespace 1 currently (remapper can't do anything else)!");
		}

		fn invert<T: Clone, const N: usize>(arr: &[T; N], namespace: usize) -> [T; N] {
			let mut arr = arr.clone();
			arr.swap(0, namespace); // namespace is checked in outer function
			arr
		}

		let remapper = self.remapper(Namespace(0), namespace)?;
		let namespace = namespace.0;

		let mut m = Mappings::new(MappingInfo {
			namespaces: invert(&self.info.namespaces, namespace),
		});

		for class in self.classes.values() {
			let mapping = ClassMapping {
				names: invert(&class.info.names, namespace),
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
					names: invert(&field.info.names, namespace),
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
					names: invert(&method.info.names, namespace),
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
						names: invert(&parameter.info.names, namespace),
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
impl Mappings<2> {
	pub(crate) fn merge(a: &Mappings<2>, b: &Mappings<2>) -> Mappings<3> {
		// new CommandMergeTinyV2().run([b, a, return, "intermediary", "official"])

		todo!()
	}
}

#[cfg(test)]
mod testing {
	#[test]
	fn test_reorder() {
		let input = include_str!("test/reorder_input.tiny");
		let expected = include_str!("test/reorder_output.tiny");

		let input = crate::reader::tiny_v2::read(input.as_bytes()).unwrap();

		let actual = input.reorder(["namespaceB", "namespaceA"]).unwrap();

		let actual = crate::writer::tiny_v2::write_string(&actual).unwrap();

		assert_eq!(actual, expected, "\nactual: {actual}\nexpected: {expected}");

	}
}
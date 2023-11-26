use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{bail, Result};
use indexmap::{IndexMap, IndexSet};
use crate::tree::{ClassNowode, FieldNowode, MethodNowode, Names, NodeDataMut, ParameterNowode};
use crate::tree::mappings::{ClassMapping, FieldMapping, MappingInfo, Mappings, MethodMapping, ParameterMapping};

fn merge_option<F, T, V>(f: F, a: &Option<T>, b: &Option<T>) -> Result<Option<V>>
where
	F: Fn(&T) -> &Option<V>,
	V: Clone + Debug,
{
	Ok(match (a.as_ref().map(|x| f(x)), b.as_ref().map(|x| f(x))) {
		(None, None) => unreachable!(),
		(None, Some(b)) => b.clone(),
		(Some(a), None) => a.clone(),
		(Some(a), Some(b)) => {
			match (a, b) {
				(None, None) => None,
				(None, Some(b)) => Some(b.clone()),
				(Some(a), None) => Some(a.clone()),
				(Some(a), Some(b)) => bail!("Cannot merge: both left {a:?} and right {b:?} are given"),
			}
		},
	})
}

fn merge_map<K, V, W, F, I>(a: Option<&IndexMap<K, V>>, b: Option<&IndexMap<K, V>>, merger: F) -> Result<IndexMap<K, W>>
where
	K: Hash + Eq + Clone,
	V: NodeDataMut<I>,
	F: Fn(Option<&V>, Option<&V>) -> Result<W>,
{
	let keys_a = a.iter().map(|x| x.keys());
	let keys_b = b.iter().map(|x| x.keys());

	let keys: IndexSet<&K> = keys_a.chain(keys_b).flatten().collect();

	let mut result = IndexMap::new();

	for key in keys {
		let a = a.map(|x| x.get(key)).flatten();
		let b = b.map(|x| x.get(key)).flatten();

		let merged = merger(a, b)?;
		result.insert(key.clone(), merged);
	}

	Ok(result)
}

fn merge(a: &[String; 2], b: &[String; 2]) -> Result<[String; 3]> {
	if a[0] != b[0] {
		bail!("Cannot merge namespaces {a:?} and {b:?}: First namespaces don't match up");
	}
	Ok([a[0].clone(), a[1].clone(), b[1].clone()])
}

fn merge_names<F, T>(f: F, a: &Option<T>, b: &Option<T>) -> Result<Names<3>>
where
	F: Fn(&T) -> &Names<2>,
{
	Ok(match (a.as_ref().map(|x| f(x)), b.as_ref().map(|x| f(x))) {
		(None, None) => unreachable!(),
		(None, Some(b)) => {
			let b: &[Option<String>; 2] = b.into();
			[b[0].clone(), None, b[1].clone()].into()
		},
		(Some(a), None) => {
			let a: &[Option<String>; 2] = a.into();
			[a[0].clone(), a[1].clone(), None].into()
		},
		(Some(a), Some(b)) => {
			let a: &[Option<String>; 2] = a.into();
			let b: &[Option<String>; 2] = b.into();
			if a[0] != b[0] {
				bail!("Cannot merge {a:?} and {b:?}: The first names must match up");
			}
			[a[0].clone(), a[1].clone(), b[1].clone()].into()
		}
	})
}

// merge two equal ones
fn merge_equal<F, T, V>(f: F, a: &Option<T>, b: &Option<T>) -> Result<V>
where
	F: Fn(&T) -> &V,
	V: Clone + PartialEq + Debug,
{
	let a = a.as_ref().map(|x| f(x));
	let b = b.as_ref().map(|x| f(x));

	match (a, b) {
		(None, None) => unreachable!(),
		(None, Some(b)) => {
			Ok(b.clone())
		},
		(Some(a), None) => {
			Ok(a.clone())
		},
		(Some(a), Some(b)) => {
			if a != b {
				bail!("Cannot merge {a:?} and {b:?}: expected them to be equal, but they are not equal");
			}
			Ok(a.clone())
		}
	}
}

impl Mappings<2> {
	pub(crate) fn merge(a: &Mappings<2>, b: &Mappings<2>) -> Result<Mappings<3>> {
		// new CommandMergeTinyV2().run([b, a, return, "intermediary", "official"])

		if a.info.namespaces[0] != b.info.namespaces[0] {
			bail!("Cant merge two differently named namespaces: {:?} and {:?}", a.info.namespaces, b.info.namespaces);
		}

		Ok(Mappings {
			info: MappingInfo {
				namespaces: merge(&a.info.namespaces, &b.info.namespaces)?,
			},
			javadoc: merge_option(|x| &x.javadoc, &Some(a), &Some(b))?,
			classes: merge_map(
				Some(&a.classes), Some(&b.classes),
				|a, b| Ok(ClassNowode {
					info: ClassMapping {
						names: merge_names(|x| &x.info.names, &a, &b)?,
					},
					javadoc: merge_option(|x| &x.javadoc, &a, &b)?,
					fields: merge_map(
						a.map(|x| &x.fields), b.map(|x| &x.fields),
						|a, b| Ok(FieldNowode {
							info: FieldMapping {
								desc: merge_equal(|x| &x.info.desc, &a, &b)?,
								names: merge_names(|x| &x.info.names, &a, &b)?,
							},
							javadoc: merge_option(|x| &x.javadoc, &a, &b)?,
						})
					)?,
					methods: merge_map(
						a.map(|x| &x.methods), b.map(|x| &x.methods),
						|a, b| Ok(MethodNowode {
							info: MethodMapping {
								desc: merge_equal(|x| &x.info.desc, &a, &b)?,
								names: merge_names(|x| &x.info.names, &a, &b)?,
							},
							javadoc: merge_option(|x| &x.javadoc, &a, &b)?,
							parameters: merge_map(
								a.map(|x| &x.parameters), b.map(|x| &x.parameters),
								|a, b| Ok(ParameterNowode {
									info: ParameterMapping {
										index: merge_equal(|x| &x.info.index, &a, &b)?,
										names: merge_names(|x| &x.info.names, &a, &b)?,
									},
									javadoc: merge_option(|x| &x.javadoc, &a, &b)?,
								})
							)?,
						})
					)?,
				})
			)?,
		})
	}
}

#[cfg(test)]
mod testing {
	use crate::tree::mappings::Mappings;

	#[test]
	fn merge() {
		let input_a = include_str!("test/merge_input_a.tiny");
		let input_b = include_str!("test/merge_input_b.tiny");
		let expected = include_str!("test/merge_output.tiny");

		let input_a = crate::reader::tiny_v2::read(input_a.as_bytes()).unwrap();
		let input_b = crate::reader::tiny_v2::read(input_b.as_bytes()).unwrap();

		let actual = Mappings::merge(&input_a, &input_b).unwrap();

		let actual = crate::writer::tiny_v2::write_string(&actual).unwrap();

		assert_eq!(actual, expected, "\nactual: {actual}\nexpected: {expected}");
	}
}
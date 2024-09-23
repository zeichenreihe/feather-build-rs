use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{bail, Context, Error, Result};
use indexmap::{IndexMap, IndexSet};
use crate::tree::names::Namespaces;
use crate::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, MappingInfo, Mappings, MethodMapping, MethodNowodeMapping, ParameterMapping, ParameterNowodeMapping};
use crate::tree::{NodeInfo, NodeJavadocInfo};

fn merge_javadoc<T, V>(
	a: Option<&T>,
	b: Option<&T>
) -> Result<Option<V>>
where
	T: NodeJavadocInfo<V>,
	V: Clone + Debug + PartialEq,
{
	Ok(match (a.map(NodeJavadocInfo::get_node_javadoc_info), b.map(NodeJavadocInfo::get_node_javadoc_info)) {
		(None, None) => unreachable!(),
		(None, Some(b)) => b.clone(),
		(Some(a), None) => a.clone(),
		(Some(a), Some(b)) => match (a, b) {
			(None, None) => None,
			(None, Some(b)) => Some(b.clone()),
			(Some(a), None) => Some(a.clone()),
			(Some(a), Some(b)) if a == b => Some(a.clone()),
			(Some(a), Some(b)) => bail!("cannot merge: both left {a:?} and right {b:?} are given"),
		},
	})
}

fn merge_map<K, V, W>(
	a: Option<&IndexMap<K, V>>,
	b: Option<&IndexMap<K, V>>,
	merger: impl Fn(Option<&V>, Option<&V>) -> Result<W>
) -> Result<IndexMap<K, W>>
where
	K: Hash + Eq + Clone,
{
	let keys: IndexSet<&K> = a.into_iter().chain(b).flat_map(IndexMap::keys).collect();

	keys.into_iter()
		.map(|key| {
			// at least one of the two is Some(_)!
			// TODO? can we encode this with types?
			let a = a.and_then(|x| x.get(key));
			let b = b.and_then(|x| x.get(key));

			let merged = merger(a, b)?;
			Ok((key.clone(), merged))
		})
		.collect()
}

fn merge_namespaces(a: &Namespaces<2>, b: &Namespaces<2>) -> Result<Namespaces<3>> {
	let a: [String; 2] = a.clone().into();
	let b: [String; 2] = b.clone().into();

	if a[0] != b[0] {
		bail!("cannot merge namespaces {a:?} and {b:?}: first namespaces don't match up");
	}
	let [_, c] = b;
	let [a, b] = a;
	[a, b, c].try_into()
}

fn merge_names<F, T, U, N2, N3, V>(f: F, a: Option<&T>, b: Option<&T>) -> Result<N3>
where
	F: Fn(&V) -> &N2 + Copy,
	T: NodeInfo<V>,
	for<'a> &'a N2: Into<&'a [Option<U>; 2]>,
	N3: TryFrom<[Option<U>; 3], Error=Error>,
	U: Debug + Clone + PartialEq,
{
	match (a.map(NodeInfo::get_node_info).map(f), b.map(NodeInfo::get_node_info).map(f)) {
		(None, None) => unreachable!(),
		(None, Some(b)) => {
			let b: &[Option<U>; 2] = b.into();
			[b[0].clone(), None, b[1].clone()].try_into()
		},
		(Some(a), None) => {
			let a: &[Option<U>; 2] = a.into();
			[a[0].clone(), a[1].clone(), None].try_into()
		},
		(Some(a), Some(b)) => {
			let a: &[Option<U>; 2] = a.into();
			let b: &[Option<U>; 2] = b.into();
			if a[0] != b[0] {
				bail!("cannot merge {a:?} and {b:?}: the first names must match up");
			}
			[a[0].clone(), a[1].clone(), b[1].clone()].try_into()
		}
	}
}

fn merge_equal<F, T, V>(f: F, a: &Option<T>, b: &Option<T>) -> Result<V>
where
	F: Fn(&T) -> &V,
	V: Clone + PartialEq + Debug,
{
	let a = a.as_ref().map(&f);
	let b = b.as_ref().map(f);

	match (a, b) {
		(None, None) => unreachable!(),
		(None, Some(b)) => Ok(b.clone()),
		(Some(a), None) => Ok(a.clone()),
		(Some(a), Some(b)) if a == b => Ok(a.clone()),
		(Some(a), Some(b)) => bail!("cannot merge {a:?} and {b:?}: expected them to be equal, but they are not equal"),
	}
}

impl Mappings<2> {
	// TODO: docs
	pub fn merge(a: &Mappings<2>, b: &Mappings<2>) -> Result<Mappings<3>> {
		Ok(Mappings {
			info: MappingInfo {
				namespaces: merge_namespaces(&a.info.namespaces, &b.info.namespaces).context("failed to merge namespaces")?,
			},
			javadoc: merge_javadoc(Some(a), Some(b))?,
			classes: merge_map(
				Some(&a.classes), Some(&b.classes),
				|a, b| Ok(ClassNowodeMapping {
					info: ClassMapping {
						names: merge_names(|x| &x.names, a, b)?,
					},
					javadoc: merge_javadoc(a, b)?,
					fields: merge_map(
						a.map(|x| &x.fields), b.map(|x| &x.fields),
						|a, b| Ok(FieldNowodeMapping {
							info: FieldMapping {
								desc: merge_equal(|x| &x.info.desc, &a, &b).context("cannot merge field descriptors")?,
								names: merge_names(|x| &x.names, a, b)?,
							},
							javadoc: merge_javadoc(a, b)?,
						})
					)?,
					methods: merge_map(
						a.map(|x| &x.methods), b.map(|x| &x.methods),
						|a, b| Ok(MethodNowodeMapping {
							info: MethodMapping {
								desc: merge_equal(|x| &x.info.desc, &a, &b).context("cannot merge method descriptors")?,
								names: merge_names(|x| &x.names, a, b)?,
							},
							javadoc: merge_javadoc(a, b)?,
							parameters: merge_map(
								a.map(|x| &x.parameters), b.map(|x| &x.parameters),
								|a, b| Ok(ParameterNowodeMapping {
									info: ParameterMapping {
										index: merge_equal(|x| &x.info.index, &a, &b).context("cannot merge parameter indices")?,
										names: merge_names(|x| &x.names, a, b)?,
									},
									javadoc: merge_javadoc(a, b)?,
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
	// TODO: consider testing internals
}
use anyhow::{anyhow, bail, Context, Result};
use crate::tree::mappings::Mappings;
use crate::tree::mappings_diff::{Action, ClassNowodeDiff, FieldNowodeDiff, MappingsDiff, MethodNowodeDiff, ParameterNowodeDiff};
use crate::tree::{GetNames, NodeInfo, NodeJavadocInfo};
use crate::tree::names::Namespace;
use diff_and_merge::*;

pub(crate) mod diff_and_merge {
	use anyhow::Result;
	use std::hash::Hash;
	use indexmap::{IndexMap, IndexSet};

	pub(crate) enum Combination<T> {
		A(T),
		B(T),
		AB(T, T),
	}

	impl<'a, T> Clone for Combination<&'a T> {
		fn clone(&self) -> Self { *self }
	}
	impl<'a, T> Copy for Combination<&'a T> {}

	impl<'a, T> Combination<&'a T> {
		pub(crate) fn map<U>(self, f: impl Fn(&'a T) -> U) -> Combination<U> {
			match self {
				Combination::A(a) => Combination::A(f(a)),
				Combination::B(b) => Combination::B(f(b)),
				Combination::AB(a, b) => Combination::AB(f(a), f(b)),
			}
		}
	}

	fn map_combine_one_side<K, V, W>(
		map: &IndexMap<K, V>,
		combiner: impl Fn(&V) -> Result<W>
	) -> Result<IndexMap<K, W>>
		where
			K: Hash + Eq + Clone
	{
		map.iter()
			.map(|(key, value)| Ok((key.clone(), combiner(value)?)))
			.collect()
	}

	fn zip_map<K, V, W>(
		a: &IndexMap<K, V>,
		b: &IndexMap<K, V>,
		combiner: impl Fn(Combination<&V>) -> Result<W>
	) -> Result<IndexMap<K, W>>
		where
			K: Hash + Eq + Clone
	{
		let keys: IndexSet<&K> = a.keys().chain(b.keys()).collect();
		let combined_map: IndexMap<_, _> = keys.into_iter()
			.map(|key| {
				// Since we have a key, at least one `get` must be `Some(_)`
				let combination = match (a.get(key), b.get(key)) {
					(None, None) => unreachable!(),
					(Some(a), None) => Combination::A(a),
					(None, Some(b)) => Combination::B(b),
					(Some(a), Some(b)) => Combination::AB(a, b),
				};

				(key, combination)
			})
			.collect();

		combined_map.into_iter()
			.map(|(key, value)| Ok((key.clone(), combiner(value)?)))
			.collect()
	}

	pub(crate) fn zip_map_combination<K, V, W>(
		ab: Combination<&IndexMap<K, V>>,
		combiner: impl Fn(Combination<&V>) -> Result<W>
	) -> Result<IndexMap<K, W>>
		where
			K: Hash + Eq + Clone,
	{
		match ab {
			Combination::A(a) => map_combine_one_side(a, |a| combiner(Combination::A(a))),
			Combination::B(b) => map_combine_one_side(b, |b| combiner(Combination::B(b))),
			Combination::AB(a, b) => zip_map(a, b, combiner),
		}
	}
}

fn gen_diff_javadoc<Target, Javadoc>(ab: Combination<&Target>) -> Action<Javadoc>
	where
		Target: NodeJavadocInfo<Option<Javadoc>>,
		Javadoc: Clone,
{
	match ab.map(|target| target.get_node_javadoc_info().clone()) {
		Combination::A(a) => a.map_or(Action::None, Action::Remove),
		Combination::B(b) => b.map_or(Action::None, Action::Add),
		Combination::AB(a, b) => Action::from_tuple(a, b),
	}
}

fn gen_diff_names<Target, Name, Mapping>(ab: Combination<&Target>) -> Result<Action<Name>>
	where
		Target: NodeInfo<Mapping>,
		Name: Clone,
		Mapping: GetNames<2, Name>,
{
	let target_namespace = Namespace::new(1)?;

	match ab.map(|target| target.get_node_info().get_names()[target_namespace].clone()) {
		Combination::A(a) => a.map(Action::Remove).with_context(|| anyhow!("cannot generate diff for removal with empty name")),
		Combination::B(b) => b.map(Action::Add).with_context(|| anyhow!("cannot generate diff for addition with empty name")),
		Combination::AB(a, b) => {
			let a = a.with_context(|| anyhow!("cannot generate diff for mapping with empty name on side a"))?;
			let b = b.with_context(|| anyhow!("cannot generate diff for mapping with empty name on side b"))?;
			Ok(Action::Edit(a, b))
		},
	}
}

impl MappingsDiff {
	pub fn diff(a: &Mappings<2>, b: &Mappings<2>) -> Result<MappingsDiff> {
		if a.info.namespaces != b.info.namespaces {
			bail!("namespaces don't match: {:?} vs {:?}", a.info.namespaces, b.info.namespaces);
		}

		let ab = Combination::AB(a, b);
		Ok(MappingsDiff {
			// TODO: namespace renaming is possible!
			info: Action::None,
			classes: zip_map_combination(
				ab.map(|x| &x.classes),
				|ab| Ok(ClassNowodeDiff {
					info: gen_diff_names(ab)?,
					fields: zip_map_combination(
						ab.map(|x| &x.fields),
						|ab| Ok(FieldNowodeDiff {
							info: gen_diff_names(ab)?,
							javadoc: gen_diff_javadoc(ab),
						})
					)?,
					methods: zip_map_combination(
						ab.map(|x| &x.methods),
						|ab| Ok(MethodNowodeDiff {
							info: gen_diff_names(ab)?,
							parameters: zip_map_combination(
								ab.map(|x| &x.parameters),
								|ab| Ok(ParameterNowodeDiff {
									info: gen_diff_names(ab)?,
									javadoc: gen_diff_javadoc(ab),
								})
							)?,
							javadoc: gen_diff_javadoc(ab),
						})
					)?,
					javadoc: gen_diff_javadoc(ab),
				})
			)?,
			javadoc: gen_diff_javadoc(ab),
		})
	}
}

#[cfg(test)]
mod testing {
	// TODO: test internals?
}
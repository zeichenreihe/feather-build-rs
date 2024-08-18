use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::{IndexMap, IndexSet};
use crate::tree::mappings::Mappings;
use crate::tree::mappings_diff::{Action, ClassNowodeDiff, FieldNowodeDiff, MappingsDiff, MethodNowodeDiff, ParameterNowodeDiff};
use crate::tree::{GetNames, NodeInfo};
use crate::tree::names::Namespace;

fn gen_diff_option<T, V>(
	f: impl Fn(&T) -> &Option<V>,
	a: &Option<T>,
	b: &Option<T>
) -> Option<Action<V>>
	where
		V: Clone,
{
	match (a.as_ref().map(&f), b.as_ref().map(f)) {
		(None, None) => unreachable!(),
		(None, Some(b)) => b.as_ref().map(|b| Action::Add(b.clone())),
		(Some(a), None) => a.as_ref().map(|a| Action::Remove(a.clone())),
		(Some(a), Some(b)) => match (a, b) {
			(None, None) => None,
			(None, Some(b)) => Some(Action::Add(b.clone())),
			(Some(a), None) => Some(Action::Remove(a.clone())),
			(Some(a), Some(b)) => Some(Action::Edit(a.clone(), b.clone())),
		},
	}
}

fn gen_diff_map<K, V, W>(
	a: Option<&IndexMap<K, V>>,
	b: Option<&IndexMap<K, V>>,
	diff_gen: impl Fn(Option<&V>, Option<&V>) -> Result<W>
) -> Result<IndexMap<K, W>>
where
	K: Hash + Eq + Clone,
{
	let keys: IndexSet<&K> = a.into_iter().chain(b).flat_map(IndexMap::keys).collect();

	keys.into_iter()
		.map(|key| {
			let a = a.and_then(|x| x.get(key));
			let b = b.and_then(|x| x.get(key));

			let diff = diff_gen(a, b)?;
			Ok((key.clone(), diff))
		})
		.collect()
}

fn gen_diff_names<Target, Name, Mapping>(
	a: Option<&Target>,
	b: Option<&Target>,
) -> Result<Action<Name>>
	where
		Target: NodeInfo<Mapping>,
		Name: Clone,
		Mapping: GetNames<2, Name>,
{
	let target_namespace = Namespace::new(1)?;

	match (a, b) {
		(None, None) => unreachable!(),
		(None, Some(b)) => {
			b.get_node_info()
				.get_names()[target_namespace]
				.clone()
				.map(|name| Action::Add(name))
				.with_context(|| anyhow!("cannot generate diff for addition with empty name"))
		},
		(Some(a), None) => {
			a.get_node_info()
				.get_names()[target_namespace]
				.clone()
				.map(|name| Action::Remove(name))
				.with_context(|| anyhow!("cannot generate diff for addition with empty name"))
		},
		(Some(a), Some(b)) => {
			let a = a.get_node_info()
				.get_names()[target_namespace]
				.clone()
				.with_context(|| anyhow!("cannot generate diff for mapping with empty name on side a"))?;
			let b = b.get_node_info()
				.get_names()[target_namespace]
				.clone()
				.with_context(|| anyhow!("cannot generate diff for mapping with empty name on side b"))?;
			Ok(Action::Edit(a, b))
		},
	}
}


impl MappingsDiff {
	pub fn diff(a: &Mappings<2>, b: &Mappings<2>) -> Result<MappingsDiff> {
		if a.info.namespaces != b.info.namespaces {
			bail!("namespaces don't match: {:?} vs {:?}", a.info.namespaces, b.info.namespaces);
		}

		Ok(MappingsDiff {
			// TODO: namespace renaming is possible!
			info: Action::None,
			classes: gen_diff_map(
				Some(&a.classes), Some(&b.classes),
				|a, b| Ok(ClassNowodeDiff {
					info: gen_diff_names(a, b)?,
					fields: gen_diff_map(
						a.map(|x| &x.fields), b.map(|x| &x.fields),
						|a, b| Ok(FieldNowodeDiff {
							info: gen_diff_names(a, b)?,
							javadoc: gen_diff_option(|x| &x.javadoc, &a, &b),
						})
					)?,
					methods: gen_diff_map(
						a.map(|x| &x.methods), b.map(|x| &x.methods),
						|a, b| Ok(MethodNowodeDiff {
							info: gen_diff_names(a, b)?,
							parameters: gen_diff_map(
								a.map(|x| &x.parameters), b.map(|x| &x.parameters),
								|a, b| Ok(ParameterNowodeDiff {
									info: gen_diff_names(a, b)?,
									javadoc: gen_diff_option(|x| &x.javadoc, &a, &b),
								})
							)?,
							javadoc: gen_diff_option(|x| &x.javadoc, &a, &b),
						})
					)?,
					javadoc: gen_diff_option(|x| &x.javadoc, &a, &b),
				})
			)?,
			javadoc: gen_diff_option(|x| x, &Some(&a.javadoc), &Some(&b.javadoc)),
		})
	}
}

#[cfg(test)]
mod testing {
	// TODO: test internals?
}
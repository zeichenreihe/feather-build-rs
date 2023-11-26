use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use crate::tree::{NodeData, ClassNowode, FieldNowode, MethodNowode, ParameterNowode, NodeDataMut};
use crate::tree::mappings::Mappings;
use crate::tree::mappings_diff::{Action, MappingsDiff};

fn apply_diff_option<T>(
	diff: &Option<Action<T>>,
	target: Option<T>,
) -> Result<Option<T>>
	where
		T: Debug + Clone + PartialEq,
{
	Ok(match diff {
		None => target,
		Some(Action::Add(b)) => {
			if let Some(target) = target {
				bail!("Cannot add {b:?} as there's already an existing target {target:?}");
			} else {
				Some(b.clone())
			}
		},
		Some(Action::Remove(a)) => {
			if let Some(target) = target {
				if &target != a {
					bail!("Cannot remove existing target {target:?} as {a:?}, because they are not equal");
				}
				None
			} else {
				bail!("Cannot remove non existing target {a:?}");
			}
		},
		Some(Action::Edit(a, b)) => {
			if let Some(target) = target {
				if target != *a {
					bail!("Cannot edit existing target {target:?} as {a:?}, because they are not equal");
				}
				Some(b.clone())
			} else {
				bail!("Cannot edit non existing target from {a:?} to {b:?}");
			}
		},
		Some(Action::None) => {
			if target.is_some() {
				// As there is no child, we do not need to propagate down.
				target
			} else {
				bail!("Cannot not action on a non existing target");
			}
		}
	})
}

fn apply_diff_map<K, D, T, A, F, G>(
	diffs: &IndexMap<K, D>,
	targets: IndexMap<K, T>,
	new_child_node: F,
	apply_child: G,
) -> Result<IndexMap<K, T>>
	where
		K: Debug + Hash + Eq + Clone,
		D: NodeData<Action<A>>,
		T: NodeDataMut<A> + Clone,
		A: Debug + PartialEq + Clone,
		F: Fn(A) -> T,
		G: Fn(&D, T) -> Result<T>,
{
	// There are four different cases:
	// 1. A key is not targets and not in diffs. We never get such a key here.
	// 2. A key is not in targets, but in diffs. We check that the diff action is an addition, and run that.
	// 3. A key is in targets, but not in diffs. As there is no diff action to run, we skip that.
	// 4. A key is in targets and diffs. We check that the diff action is either none, removal or change, and run that.

	// We store references to all the diffs and remove the ones we applied.
	let mut diffs: IndexMap<&K, &D> = diffs.iter().collect();

	let mut results = IndexMap::new();

	for (key, mut target) in targets.into_iter() {
		if let Some(diff) = diffs.remove(&key) {
			// Case 4: Key in both.

			// Our diff can take four forms:
			// Add(b):     This is not possible as there's already an existing target.
			// Remove(a):  We remove it and do not further check any members. // TODO: maybe impl this further check as well?
			// Edit(a, b): We change the current node and further apply diffs of members.
			// None:       We apply diffs of members.
			match diff.node_data() {
				Action::Add(b) => {
					bail!("Cannot add {b:?} as there's already an existing target {:?} with the same key {key:?}", target.node_data());
				},
				Action::Remove(a) => {
					if target.node_data() != a {
						bail!("Cannot remove existing target {:?} with the same key {key:?} as {a:?}, because they are not equal", target.node_data());
					}

					// Not adding it is a removal.
				},
				Action::Edit(a, b) => {
					if target.node_data() != a {
						bail!("Cannot edit existing target {:?} with the same key {key:?} as {a:?}, because they are not equal", target.node_data());
					}
					*target.node_data_mut() = b.clone();

					results.insert(key, apply_child(diff, target)?);
				},
				Action::None => {
					results.insert(key, apply_child(diff, target)?);
				},
			}
		} else {
			// Case 3: Key only in targets.
			// Skip it.
			results.insert(key, target);
		}
	}

	for (key, diff) in diffs.into_iter() {
		// Case 2: Key only in diffs.

		// Our diff can take four forms:
		// Add(b):     We add the new target and further apply the diffs of the children.
		// Remove(a):  This is not possible as there's no such target to remove.
		// Edit(a, b): This is also not possible as there's no target to edit.
		// None:       This is not possible because we can't add (only valid action of members) a target without any stored information.
		match diff.node_data() {
			Action::Add(b) => {
				let node = new_child_node(b.clone());

				let node = apply_child(diff, node)?;

				results.insert(key.clone(), node);
			},
			Action::Remove(a) => bail!("Cannot remove non existing target {a:?}"),
			Action::Edit(a, b) => bail!("Cannot edit non existing target from {a:?} to {b:?}"),
			Action::None => bail!("Cannot not action on a non existing target for key {key:?}"),
		}
	}

	// Case 1: Key in neither.
	// We don't get any key that isn't at least part in one, so we don't need to handle it.

	Ok(results)
}

impl MappingsDiff {
	pub(crate) fn apply_to(&self, target: Mappings<2>) -> Result<Mappings<2>> {
		// TODO: rewrite this to allow "nth namespace" of "mappings<N>"
		Ok(Mappings {
			info: match &self.info {
				Action::Add(a) => bail!("Cannot add {a:?} as there's already an existing target {:?}", target.info),
				Action::Remove(b) => bail!("Cannot remove {b:?} as then there would be no mappings {:?} anymore", target.info),
				Action::Edit(a, b) => {
					if &target.info != a {
						bail!("Cannot edit existing target {:?} as {a:?}, because they are not equal", target.info);
					}
					b.clone()
				},
				Action::None => target.info,
			},
			javadoc: apply_diff_option(&self.javadoc, target.javadoc)?,
			classes: apply_diff_map(
				&self.classes,
				target.classes,
				|class| ClassNowode::new(class),
				|diff, class| Ok(ClassNowode {
					javadoc: apply_diff_option(&diff.javadoc, class.javadoc)?,
					fields: apply_diff_map(
						&diff.fields,
						class.fields,
						|field| FieldNowode::new(field),
						|diff, field| Ok(FieldNowode {
							javadoc: apply_diff_option(&diff.javadoc, field.javadoc)
								.with_context(|| anyhow!("Failed to apply diff for javadoc in field {:?}", field.info))?,
							info: field.info,
						})
					)
						.with_context(|| anyhow!("Failed to apply diff for field in class {:?}", class.info))?,
					methods: apply_diff_map(
						&diff.methods,
						class.methods,
						|method| MethodNowode::new(method),
						|diff, method| Ok(MethodNowode {
							javadoc: apply_diff_option(&diff.javadoc, method.javadoc)
								.with_context(|| anyhow!("Failed to apply diff for javadoc in method {:?}", method.info))?,
							parameters: apply_diff_map(
								&diff.parameters,
								method.parameters,
								|parameter| ParameterNowode::new(parameter),
								|diff, parameter| Ok(ParameterNowode {
									javadoc: apply_diff_option(&diff.javadoc, parameter.javadoc)
										.with_context(|| anyhow!("Failed to apply diff for javadoc in parameter {:?}", parameter.info))?,
									info: parameter.info,
								})
							)
								.with_context(|| anyhow!("Failed to apply diff for parameter in method {:?}", method.info))?,
							info: method.info,
						})
					)
						.with_context(|| anyhow!("Failed to apply diff for method in class {:?}", class.info))?,
					info: class.info,
				})
			)?,
		})
	}
}

use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use crate::tree::mappings::{ClassNowodeMapping, FieldNowodeMapping, Mappings, MethodNowodeMapping, ParameterNowodeMapping};
use crate::tree::mappings_diff::{Action, MappingsDiff};
use crate::tree::names::{Names, Namespace};
use crate::tree::{FromKey, NodeInfo};

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
				bail!("cannot add {b:?} as there's already an existing target {target:?}");
			} else {
				Some(b.clone())
			}
		},
		Some(Action::Remove(a)) => {
			if let Some(target) = target {
				if &target != a {
					bail!("cannot remove existing target {target:?} as {a:?}, because they are not equal");
				}
				None
			} else {
				bail!("cannot remove non existing target {a:?}");
			}
		},
		Some(Action::Edit(a, b)) => {
			if let Some(target) = target {
				if target != *a {
					bail!("cannot edit existing target {target:?} as {a:?}, because they are not equal");
				}
				Some(b.clone())
			} else {
				bail!("cannot edit non existing target from {a:?} to {b:?}");
			}
		},
		Some(Action::None) => {
			if target.is_some() {
				// As there is no child, we do not need to propagate down.
				target
			} else {
				bail!("cannot not action on a non existing target");
			}
		}
	})
}

fn apply_diff_map<const N: usize, Key, Diff, Target, Name, Mapping>(
	target_namespace: Namespace<N>,
	diffs: &IndexMap<Key, Diff>,
	targets: IndexMap<Key, Target>,
	names_get_mut: impl Fn(&mut Mapping) -> &mut Names<N, Name>,
	apply_child: impl Fn(&Diff, Target) -> Result<Target>,
) -> Result<IndexMap<Key, Target>>
	where
		Key: Debug + Hash + Eq + Clone,
		Diff: NodeInfo<Action<Name>>,
		Target: Clone + NodeInfo<Mapping>,
		Name: Debug + PartialEq + Clone,
		Mapping: FromKey<Key>,
{
	// There are four different cases:
	// 1. A key is in targets and diffs.
	// 2. A key is in targets, but not in diffs. As there is no diff action for this key, just copy the entry.
	// 3. A key is not in targets, but in diffs. We check that the diff action is an addition, and run that.
	// 4. A key is not targets and not in diffs. We never get such a key here.
	//  We check that the diff action is either none, removal or change, and run that. (todo: update)

	// We store references to all the diffs and remove the ones we applied.
	let mut diffs: IndexMap<&Key, &Diff> = diffs.iter().collect();

	let mut results = IndexMap::new();

	for (key, mut target) in targets.into_iter() {
		if let Some(diff) = diffs.remove(&key) {
			// Case 1: Key in both.

			// Our diff can take four forms:
			// Add(b):     We put the name in and further apply diffs of members.
			// Remove(a):  We remove it and do not further check any members. // TODO: maybe impl this further check as well?
			// Edit(a, b): We change the current node and further apply diffs of members.
			// None:       We apply diffs of members.
			let action = diff.get_node_info();
			match action {
				Action::Add(b) => {
					// Add the name
					names_get_mut(target.get_node_info_mut())
						.change_name(target_namespace, None, Some(b))
						.with_context(|| anyhow!("cannot apply action {action:?} with same key {key:?}"))?;

					// Run on the children and store
					results.insert(key, apply_child(diff, target)?);
				},
				Action::Remove(a) => {
					// Check the name for removal
					names_get_mut(target.get_node_info_mut())
						.change_name(target_namespace, Some(a), None)
						.with_context(|| anyhow!("cannot apply action {action:?} with same key {key:?}"))?;

					// TODO: consider if we'd instead want to run the children as well
					//  most likely not, since it hits performance

					// Not storing it is a removal
				},
				Action::Edit(a, b) => {
					// Edit the name correctly
					names_get_mut(target.get_node_info_mut())
						.change_name(target_namespace, Some(a), Some(b))
						.with_context(|| anyhow!("cannot apply action {action:?} with same key {key:?}"))?;

					// Run on the children and store
					results.insert(key, apply_child(diff, target)?);
				},
				Action::None => {
					// No name to deal with.

					// Run on the children and store
					results.insert(key, apply_child(diff, target)?);
				},
			}
		} else {
			// Case 2: Key only in targets.
			results.insert(key, target);
		}
	}

	for (key, diff) in diffs.into_iter() {
		// Case 3: Key only in diffs.

		// Our diff can take four forms:
		// Add(b):     We add the new target and further apply the diffs of the children.
		// Remove(a):  This is not possible as there's no such target to remove.
		// Edit(a, b): This is also not possible as there's no target to edit.
		// None:       This is not possible because we can't add (only valid action of members) a target without any stored information.
		match diff.get_node_info() {
			Action::Add(b) => {
				let mut info = Mapping::from_key(key.clone());

				names_get_mut(&mut info)[target_namespace] = Some(b.clone());

				let node = Target::new(info);

				let node = apply_child(diff, node)?;

				results.insert(key.clone(), node);
			},
			action => bail!("cannot apply action {action:?} on non existing target for key {key:?}"),
		}
	}

	// Case 4: Key in neither.
	// We don't get any key that isn't at least part in one, so we don't need to handle it.

	Ok(results)
}

impl MappingsDiff {
	// TODO: docs
	pub fn apply_to<const N: usize>(&self, target: Mappings<N>, namespace: &str) -> Result<Mappings<N>> {
		let namespace = target.get_namespace(namespace)?;
		Ok(Mappings {
			info: match &self.info {
				Action::Add(a) => bail!("cannot add {a:?} as there's already an existing target {:?}", target.info),
				Action::Remove(b) => bail!("cannot remove {b:?} as then there would be no mappings {:?} anymore", target.info),
				action @ Action::Edit(a, b) => {
					let mut t = target.info;
					t.namespaces.change_name(namespace, a, b)
						.with_context(|| anyhow!("cannot apply action {action:?}"))?;
					t
				},
				Action::None => target.info,
			},
			javadoc: apply_diff_option(&self.javadoc, target.javadoc)?,
			classes: apply_diff_map(namespace,
				&self.classes, target.classes, |t| &mut t.names,
				|diff, class| Ok(ClassNowodeMapping {
					javadoc: apply_diff_option(&diff.javadoc, class.javadoc)?,
					fields: apply_diff_map(namespace,
						&diff.fields, class.fields, |t| &mut t.names,
						|diff, field| Ok(FieldNowodeMapping {
							javadoc: apply_diff_option(&diff.javadoc, field.javadoc)
								.with_context(|| anyhow!("failed to apply diff for javadoc in field {:?}", field.info))?,
							info: field.info,
						})
					)
						.with_context(|| anyhow!("failed to apply diff for field in class {:?}", class.info))?,
					methods: apply_diff_map(namespace,
						&diff.methods, class.methods, |t| &mut t.names,
						|diff, method| Ok(MethodNowodeMapping {
							javadoc: apply_diff_option(&diff.javadoc, method.javadoc)
								.with_context(|| anyhow!("failed to apply diff for javadoc in method {:?}", method.info))?,
							parameters: apply_diff_map(namespace,
								&diff.parameters, method.parameters, |t| &mut t.names,
								|diff, parameter| Ok(ParameterNowodeMapping {
									javadoc: apply_diff_option(&diff.javadoc, parameter.javadoc)
										.with_context(|| anyhow!("failed to apply diff for javadoc in parameter {:?}", parameter.info))?,
									info: parameter.info,
								})
							)
								.with_context(|| anyhow!("failed to apply diff for parameter in method {:?}", method.info))?,
							info: method.info,
						})
					)
						.with_context(|| anyhow!("failed to apply diff for method in class {:?}", class.info))?,
					info: class.info,
				})
			)?,
		})
	}
}

// TODO: consider testing internals (see extend_inner_class_names.rs for example)
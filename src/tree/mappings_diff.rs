use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use crate::tree::{NodeData, ClassNowode, FieldNowode, Mapping, MethodNowode, ParameterNowode, NodeDataMut};
use crate::tree::mappings::{ClassKey, ClassMapping, FieldKey, FieldMapping, JavadocMapping, MappingInfo, MethodKey, MethodMapping, ParameterKey, ParameterMapping, Mappings};

#[derive(Debug, Clone, Default)]
pub(crate) enum Action<T> {
	Add(T),
	Remove(T),
	Edit(T, T),
	#[default]
	None,
}

pub(crate) type MappingsDiff = Mapping<
	Action<MappingInfo>,
	ClassKey, Action<ClassMapping>,
	FieldKey, Action<FieldMapping>,
	MethodKey, Action<MethodMapping>,
	ParameterKey, Action<ParameterMapping>,
	Action<JavadocMapping>
>;
pub(crate) type ClassNowodeDiff = ClassNowode<
	Action<ClassMapping>,
	FieldKey, Action<FieldMapping>,
	MethodKey, Action<MethodMapping>,
	ParameterKey, Action<ParameterMapping>,
	Action<JavadocMapping>
>;
pub(crate) type FieldNowodeDiff = FieldNowode<Action<FieldMapping>, Action<JavadocMapping>>;
pub(crate) type MethodNowodeDiff = MethodNowode<
	Action<MethodMapping>,
	ParameterKey, Action<ParameterMapping>,
	Action<JavadocMapping>
>;
pub(crate) type ParameterNowodeDiff = ParameterNowode<Action<ParameterMapping>, Action<JavadocMapping>>;


fn apply_diff_for_option<T>(
	diff: &Option<Action<T>>,
	target: &mut Option<T>,
) -> Result<()>
where
	T: Debug + Clone + PartialEq,
{
	match diff {
		None => {},
		Some(Action::Add(b)) => {
			if let Some(target) = target {
				bail!("Cannot add {b:?} as there's already an existing target {target:?}");
			} else {
				*target = Some(b.clone());
			}
		},
		Some(Action::Remove(a)) => {
			if let Some(target) = target.take() {
				if &target != a {
					bail!("Cannot remove existing target {target:?} as {a:?}, because they are not equal");
				}
			} else {
				bail!("Cannot remove non existing target {a:?}");
			}
		},
		Some(Action::Edit(a, b)) => {
			if let Some(target) = target {
				if target != a {
					bail!("Cannot edit existing target {target:?} as {a:?}, because they are not equal");
				}
				*target = b.clone();
			} else {
				bail!("Cannot edit non existing target from {a:?} to {b:?}");
			}
		},
		Some(Action::None) => {
			if target.is_some() {
				// As there is no child, we do not need to propagate down.
			} else {
				bail!("Cannot not action on a non existing target");
			}
		}
	}

	Ok(())
}

fn apply_diff_for_hash_map<K, D, T, A, F, G>(
	diffs: &HashMap<K, D>,
	targets: &mut HashMap<K, T>,
	new_child_node: F,
	apply_child: G,
) -> Result<()>
where
	K: Debug + Hash + Eq + Clone,
	D: NodeData<Action<A>>,
	T: NodeData<A> + NodeDataMut<A>,
	A: Debug + PartialEq + Clone,
	F: Fn(A) -> T,
	G: Fn(&D, &mut T) -> Result<()>,
{
	// There are four different cases:
	// 1. A key is not targets and not in diffs. We never get such a key here.
	// 2. A key is not in targets, but in diffs. We check that the diff action is an addition, and run that.
	// 3. A key is in targets, but not in diffs. As there is no diff action to run, we skip that.
	// 4. A key is in targets and diffs. We check that the diff action is either none, removal or change, and run that.

	for (key, diff) in diffs.iter() {
		match targets.entry(key.clone()) {
			Entry::Occupied(mut e) => {
				// Case 4: Key in both.

				// Our diff can take four forms:
				// Add(b):     This is not possible as there's already an existing target.
				// Remove(a):  We remove it and do not further check any members. // TODO: maybe impl this further check as well?
				// Edit(a, b): We change the current node and further apply diffs of members.
				// None:       We apply diffs of members.
				match diff.node_data() {
					Action::Add(b) => {
						bail!("Cannot add {b:?} as there's already an existing target {:?} with the same key {key:?}", e.get().node_data());
					},
					Action::Remove(a) => {
						if e.get().node_data() != a {
							bail!("Cannot remove existing target {:?} with the same key {key:?} as {a:?}, because they are not equal", e.get().node_data());
						}

						e.remove();
					},
					Action::Edit(a, b) => {
						if e.get().node_data() != a {
							bail!("Cannot edit existing target {:?} with the same key {key:?} as {a:?}, because they are not equal", e.get().node_data());
						}
						*e.get_mut().node_data_mut() = b.clone();

						apply_child(diff, e.get_mut())?;
					},
					Action::None => {
						apply_child(diff, e.get_mut())?;
					},
				}
			},
			Entry::Vacant(e) => {
				// Case 2: Key only in diffs.

				// Our diff can take four forms:
				// Add(b):     We add the new target and further apply the diffs of the children.
				// Remove(a):  This is not possible as there's no such target to remove.
				// Edit(a, b): This is also not possible as there's no target to edit.
				// None:       This is not possible because we can't add (only valid action of members) a target without any stored information.
				match diff.node_data() {
					Action::Add(b) => {
						let mut node = new_child_node(b.clone());

						apply_child(diff, &mut node)?;

						e.insert(node);
					},
					Action::Remove(a) => {
						bail!("Cannot remove non existing target {a:?}");
					},
					Action::Edit(a, b) => {
						bail!("Cannot edit non existing target from {a:?} to {b:?}");
					},
					Action::None => {
						bail!("Cannot not action on a non existing target for key {key:?}");
					},
				}
			},
		}
	}

	// Case 3: Key only in targets.
	// Skip it. This is done by just not iterating over it.

	// Case 1: Key in neither.
	// We don't get any key that isn't at least part in one, so we don't need to handle it.

	Ok(())
}

impl MappingsDiff {
	pub(crate) fn apply_to(&self, target: &mut Mappings) -> Result<()> {
		match self.node_data() {
			Action::Add(a) => {
				bail!("Cannot add {a:?} as there's already an existing target {:?}", target.node_data());
			},
			Action::Remove(b) => {
				bail!("Cannot remove {b:?} as then there would be no mappings {:?} anymore", target.node_data())
			},
			Action::Edit(a, b) => {
				if target.node_data() != a {
					bail!("Cannot edit existing target {:?} as {a:?}, because they are not equal", target.node_data());
				}
				*target.node_data_mut() = b.clone();
			},
			Action::None => {
				// Nothing to edit, so just skip it.
			},
		}

		apply_diff_for_option(&self.javadoc, &mut target.javadoc)?;

		apply_diff_for_hash_map(
			&self.classes,
			&mut target.classes, |class| ClassNowode::new(class),
			|diff, class| {
				apply_diff_for_option(&diff.javadoc, &mut class.javadoc)?;

				apply_diff_for_hash_map(
					&diff.fields,
					&mut class.fields,
					|field| FieldNowode::new(field),
					|diff, field| {
						apply_diff_for_option(&diff.javadoc, &mut field.javadoc)
							.with_context(|| anyhow!("Failed to apply diff for javadoc in field {:?}", field.node_data()))?;

						Ok(())
					}
				)
					.with_context(|| anyhow!("Failed to apply diff for field in class {:?}", class.node_data()))?;

				apply_diff_for_hash_map(
					&diff.methods,
					&mut class.methods,
					|method| MethodNowode::new(method),
					|diff, method| {
						apply_diff_for_option(&diff.javadoc, &mut method.javadoc)
							.with_context(|| anyhow!("Failed to apply diff for javadoc in method {:?}", method.node_data()))?;

						apply_diff_for_hash_map(
							&diff.parameters,
							&mut method.parameters,
							|parameter| ParameterNowode::new(parameter),
							|diff, parameter| {
								apply_diff_for_option(&diff.javadoc, &mut parameter.javadoc)
									.with_context(|| anyhow!("Failed to apply diff for javadoc in parameter {:?}", parameter.node_data()))?;

								Ok(())
							}
						)
							.with_context(|| anyhow!("Failed to apply diff for parameter in method {:?}", method.node_data()))?;

						Ok(())
					}
				)
					.with_context(|| anyhow!("Failed to apply diff for method in class {:?}", class.node_data()))?;

				Ok(())
			}
		)?;

		Ok(())
	}
}

use std::cmp::Ordering;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::{IndexMap, IndexSet};
use crate::{PropagationOptions, PropagationDirection};
use std::collections::{HashSet, VecDeque};
use std::fmt::Debug;
use indexmap::map::Entry;
use java_string::{JavaStr, JavaString};
use duke::tree::class::{ClassName, ClassNameSlice};
use duke::tree::field::{FieldNameAndDesc, FieldNameSlice};
use duke::tree::method::{MethodNameAndDesc, MethodNameSlice};
use quill::tree::mappings::{ClassMapping, ClassNowodeMapping, Mappings, MethodMapping, MethodNowodeMapping, ParameterKey};
use quill::tree::mappings_diff::{Action, ClassNowodeDiff, FieldNowodeDiff, MappingsDiff, MethodNowodeDiff, ParameterNowodeDiff};
use quill::tree::{FromKey, GetNames, NodeInfo, NodeJavadocInfo};
use quill::tree::names::Namespace;
use crate::version_graph::{VersionEntry, VersionGraph};


#[derive(Clone, Copy)]
enum PropDir {
	Up,
	Down,
}


#[derive(Clone, Copy)]
enum Mode {
	Mappings,
	Javadocs,
}


#[derive(Copy, Clone, PartialEq)]
enum DiffSide {
	A, B
}
impl DiffSide {
	pub(crate) fn opposite(&self) -> DiffSide {
		match self {
			DiffSide::A => DiffSide::B,
			DiffSide::B => DiffSide::A,
		}
	}
}


pub(crate) fn insert_mappings<'version>(
	options: PropagationOptions,
	version_graph: &'version VersionGraph,
	changes: MappingsDiff,
	version: VersionEntry<'version>,
) -> Result<()> {

	let direction_is_up = matches!(&options.direction, PropagationDirection::Up | PropagationDirection::Both);
	let direction_is_down = matches!(&options.direction, PropagationDirection::Down | PropagationDirection::Both);

	let barriers = {
		let mut barriers = IndexSet::new();
		if !direction_is_up {
			barriers.insert(version);
		}
		if !direction_is_down {
			barriers.extend(version_graph.children(version))
		}
		barriers
	};

	let mut queued_changes = IndexMap::new();
	queued_changes.insert(version, changes);

	let mut dirty = HashSet::new();

	while !queued_changes.is_empty() {
		let working_changes = queued_changes;
		queued_changes = IndexMap::new();

		for (version, changes) in working_changes {
			for (class_key, change_class) in &changes.classes {
				propagate_change(
					options.lenient,
					&mut dirty,
					&barriers,
					version_graph,
					version,
					change_class.info.is_diff(),
					change_class.javadoc.as_ref().is_some_and(Action::is_diff),
					|mappings, mode| {
						apply_change_5(class_key, change_class, &mut mappings.classes, mode)
					},
					|diff, insert, side, mode| {
						if let Some(d_class) = diffs_get_class_or_insert_dummy_if_true(diff, class_key, insert) {
							apply_change_7(d_class, change_class, side, insert, mode)
						} else {
							false
						}
					},
					|diff, side, dir, queue_sibling_change_version, mode| {
						let d_class = diff.classes.get(class_key).unwrap(); // the above closure created it already

						// mapping (does not) exists on both sides
						// so do not try to propagate to siblings
						if action_get(&change_class.info, DiffSide::A).is_none() != action_get(&d_class.info, side.opposite()).is_none(){
							let sibling = find_class_sibling(
								diff,
								class_key,
								d_class,
								change_class,
								side,
								mode
							);

							if let Some((sibling_key, sibling)) = sibling {
								let side = side.opposite();

								let mut do_queue_changes = |version: VersionEntry<'version>| {
									let changes = queued_changes.entry(version)
										.or_insert_with(|| MappingsDiff::new(Action::None));

									// originally calls add with sibling.get(side) for both a and b in Edit(a, b)
									let sibling_change = diffs_get_class_or_insert_dummy(changes, sibling_key);

									match mode {
										Mode::Mappings => {
											let ofrom = action_get(&change_class.info, DiffSide::A).unwrap();
											let from = action_get(&sibling.info, side).unwrap();
											let to = action_get(&change_class.info, DiffSide::B).unwrap();

											let (ofrom_stem, ofrom_simple) = make_class_name_stem_and_simple(ofrom);
											let (from_stem, from_simple) = make_class_name_stem_and_simple(from);
											let (to_stem, to_simple) = make_class_name_stem_and_simple(to);

											let ofrom_inner = ofrom.as_inner();
											let from_inner = from.as_inner();
											let to_inner = to.as_inner();

											let to_inner = if ofrom_simple.len() > from_simple.len() {
												format!("{}{}", from_stem, &to_simple[(ofrom_simple.len() - from_simple.len())..])
											} else {
												format!("{}{}", &from_inner[..(from_inner.len() - ofrom_simple.len())], to_simple)
											};
											let to_inner = JavaString::from(to_inner);

											let to_ = unsafe { ClassName::from_inner_unchecked(to_inner) };

											action_set(&mut sibling_change.info, DiffSide::A, Some(from));
											action_set(&mut sibling_change.info, DiffSide::B, Some(&to_));
										},
										Mode::Javadocs => {
											let sibling_javadoc = &sibling.javadoc;
											let javadoc_change = &change_class.javadoc;
											let sibling_javadoc_change = &mut sibling_change.javadoc;
											action_set_optional(sibling_javadoc_change, DiffSide::A, action_get_optional(sibling_javadoc, side));
											action_set_optional(sibling_javadoc_change, DiffSide::B, action_get_optional(javadoc_change, DiffSide::B));
										},
									}
								};

								match dir {
									PropDir::Up => {
										for p in version_graph.parents(queue_sibling_change_version) {
											do_queue_changes(p);
										}
									},
									PropDir::Down => do_queue_changes(queue_sibling_change_version),
								}
							}
						}
					},
				);


				for (field_key, change_field) in &change_class.fields {
					propagate_change(
						options.lenient,
						&mut dirty,
						&barriers,
						version_graph,
						version,
						change_field.info.is_diff(),
						change_field.javadoc.as_ref().is_some_and(Action::is_diff),
						|mappings, mode| {
							let m_class = mappings_get_class_or_insert_dummy(mappings, class_key);
							apply_change_5(field_key, change_field, &mut m_class.fields, mode)
						},
						|diff, insert, side, mode| {
							let d_class = diffs_get_class_or_insert_dummy(diff, class_key);
							if let Some(d_field) = diffs_get_field_or_insert_dummy_if_true(d_class, field_key, insert) {
								apply_change_7(d_field, change_field, side, insert, mode)
							} else {
								false
							}
						},
						|diff, side, dir, queue_sibling_change_version, mode| {
							let d_class = diff.classes.get(class_key).unwrap(); // the above closure created it already
							let d_field = d_class.fields.get(field_key).unwrap(); // the above closure created it already

							// mapping (does not) exists on both sides
							// so do not try to propagate to siblings
							if action_get(&change_field.info, DiffSide::A).is_none() != action_get(&d_field.info, side.opposite()).is_none(){
								let sibling = find_field_sibling(
									class_key,
									field_key,
									diff,
									d_class,
									d_field,
									change_class,
									change_field,
									side,
									mode
								);

								if let Some((parent_sibling_key, sibling_key, sibling)) = sibling {
									let side = side.opposite();


									let mut do_queue_changes = |version: VersionEntry<'version>| {
										let changes = queued_changes.entry(version)
											.or_insert_with(|| MappingsDiff::new(Action::None));

										// originally calls add with parent_sibling.get(side) for both a and b in Edit(a, b)
										let sibling_parent_change = diffs_get_class_or_insert_dummy(changes, parent_sibling_key);
										let sibling_change = diffs_get_field_or_insert_dummy(sibling_parent_change, sibling_key);

										match mode {
											Mode::Mappings => {
												let from = action_get(&sibling.info, side);
												let to = action_get(&change_field.info, DiffSide::B);

												action_set(&mut sibling_change.info, DiffSide::A, from);
												action_set(&mut sibling_change.info, DiffSide::B, to);
											},
											Mode::Javadocs => {
												let sibling_javadoc = &sibling.javadoc;
												let javadoc_change = &change_field.javadoc;
												let sibling_javadoc_change = &mut sibling_change.javadoc;
												action_set_optional(sibling_javadoc_change, DiffSide::A, action_get_optional(sibling_javadoc, side));
												action_set_optional(sibling_javadoc_change, DiffSide::B, action_get_optional(javadoc_change, DiffSide::B));
											},
										}
									};

									match dir {
										PropDir::Up => {
											for p in version_graph.parents(queue_sibling_change_version) {
												do_queue_changes(p);
											}
										},
										PropDir::Down => do_queue_changes(queue_sibling_change_version),
									}
								}
							}
						},
					);

					// has no children
				}
				for (method_key, change_method) in &change_class.methods {
					propagate_change(
						options.lenient,
						&mut dirty,
						&barriers,
						version_graph,
						version,
						change_method.info.is_diff(),
						change_method.javadoc.as_ref().is_some_and(Action::is_diff),
						|mappings, mode| {
							let m_class = mappings_get_class_or_insert_dummy(mappings, class_key);
							apply_change_5(method_key, change_method, &mut m_class.methods, mode)
						},
						|diff, insert, side, mode| {
							let d_class = diffs_get_class_or_insert_dummy(diff, class_key);
							if let Some(d_method) = diffs_get_method_or_insert_dummy_if_true(d_class, method_key, insert) {
								apply_change_7(d_method, change_method, side, insert, mode)
							} else {
								false
							}
						},
						|diff, side, dir, queue_sibling_change_version, mode| {
							let d_class = diff.classes.get(class_key).unwrap(); // the above closure created it already
							let d_method = d_class.methods.get(method_key).unwrap(); // the above closure created it already

							// mapping (does not) exists on both sides
							// so do not try to propagate to siblings
							if action_get(&change_method.info, DiffSide::A).is_none() != action_get(&d_method.info, side.opposite()).is_none(){
								let sibling = find_method_sibling(
									class_key,
									method_key,
									diff,
									d_class,
									d_method,
									change_class,
									change_method,
									side,
									mode
								);

								if let Some((parent_sibling_key, sibling_key, sibling)) = sibling {
									let side = side.opposite();

									let mut do_queue_changes = |version: VersionEntry<'version>| {
										let changes = queued_changes.entry(version)
											.or_insert_with(|| MappingsDiff::new(Action::None));

										// originally calls add with parent_sibling.get(side) for both a and b in Edit(a, b)
										let sibling_parent_change = diffs_get_class_or_insert_dummy(changes, parent_sibling_key);
										let sibling_change = diffs_get_method_or_insert_dummy(sibling_parent_change, sibling_key);

										match mode {
											Mode::Mappings => {
												let from = action_get(&sibling.info, side);
												let to = action_get(&change_method.info, DiffSide::B);

												action_set(&mut sibling_change.info, DiffSide::A, from);
												action_set(&mut sibling_change.info, DiffSide::B, to);
											},
											Mode::Javadocs => {
												let sibling_javadoc = &sibling.javadoc;
												let javadoc_change = &change_method.javadoc;
												let sibling_javadoc_change = &mut sibling_change.javadoc;
												action_set_optional(sibling_javadoc_change, DiffSide::A, action_get_optional(sibling_javadoc, side));
												action_set_optional(sibling_javadoc_change, DiffSide::B, action_get_optional(javadoc_change, DiffSide::B));
											},
										}
									};

									match dir {
										PropDir::Up => {
											for p in version_graph.parents(queue_sibling_change_version) {
												do_queue_changes(p);
											}
										},
										PropDir::Down => do_queue_changes(queue_sibling_change_version),
									}
								}
							}
						}
					);

					for (parameter_key, change_parameter) in &change_method.parameters {
						propagate_change(
							options.lenient,
							&mut dirty,
							&barriers,
							version_graph,
							version,
							change_parameter.info.is_diff(),
							change_parameter.javadoc.as_ref().is_some_and(Action::is_diff),
							|mappings, mode| {
								let m_class = mappings_get_class_or_insert_dummy(mappings, class_key);
								let m_method = mappings_get_method_or_insert_dummy(m_class, method_key);
								apply_change_5(parameter_key, change_parameter, &mut m_method.parameters, mode)
							},
							|diff, insert, side, mode| {
								let d_class = diffs_get_class_or_insert_dummy(diff, class_key);
								let d_method = diffs_get_method_or_insert_dummy(d_class, method_key);
								if let Some(d_parameter) = diffs_get_parameter_or_insert_dummy_if_true(d_method, parameter_key, insert) {
									apply_change_7(d_parameter, change_parameter, side, insert, mode)
								} else {
									false
								}
							},
							|_diff, _side, _dir, _queue_sibling_change_version, _mode| {
								// parameters don't queue siblings
							},
						);

						// has no children
					}
				}
			}
		}
	}

	version_graph.write();

	Ok(())
}

#[allow(clippy::too_many_arguments)]
fn propagate_change<'version>(
	options_lenient: bool,
	dirty: &mut HashSet<VersionEntry<'version>>,
	barriers: &IndexSet<VersionEntry<'version>>,
	version_graph: &'version VersionGraph,
	version: VersionEntry<'version>,
	op_is_not_none_mappings: bool,
	op_is_not_none_javadocs: bool,
	apply_to_mappings: impl Fn(&mut Mappings<2>, Mode) -> bool,
	apply_to_diffs: impl Fn(&mut MappingsDiff, bool, DiffSide, Mode) -> bool,
	mut queue_sibling_changes: impl FnMut(&MappingsDiff, DiffSide, PropDir, VersionEntry<'version>, Mode),
) {
	let mut propagate = |mode: Mode| {
		let mut propagation = PropagationQueue::new();
		propagation.offer(PropDir::Up, version);

		while let Some((dir, n)) = propagation.poll() {
			match dir {
				PropDir::Up => {
					if let Some(mappings) = version_graph.is_root_then_get_mappings(n) {
						let mut mappings = mappings.clone();
						// TODO: store mappings across multiple times

						let success = apply_to_mappings(&mut mappings, mode);

						if success {
							// success, now propagate in opposite direction
							propagation.offer(PropDir::Down, n);

							dirty.insert(n);
						}
					} else {
						let side = DiffSide::B;
						let insert = barriers.contains(&n);
						let dir = PropDir::Up;
						let queue_sibling_change_version = n;

						for p in version_graph.parents(n) {
							let mut diff = version_graph.get_diff(p, n)
								.unwrap() // this unwrap needs to be replaced with ?
								.unwrap(); // this seems to be fine, bc we checked stuff?!
							// TODO: store diffs across multiple times

							let success = apply_to_diffs(&mut diff, insert, side, mode);

							if success {
								if options_lenient && !insert {
									queue_sibling_changes(&diff, side, dir, queue_sibling_change_version, mode);
								}

								// change applied, now propagate in the opposite direction
								propagation.offer(PropDir::Down, n);

								dirty.insert(n);
							} else {
								// change not applied to this version, propagate further
								propagation.offer(PropDir::Up, p);
							}
						}
					}
				},
				PropDir::Down => {
					for c in version_graph.children(n) {
						if let Some(mappings) = version_graph.is_root_then_get_mappings(c) {
							let mut mappings = mappings.clone();

							let success = apply_to_mappings(&mut mappings, mode);

							// TODO: store mappings

							if success {
								dirty.insert(c);
							}
						} else {
							// loop could be removed since p is always equal to s, bc of parent/child symmetry

							let mut diff = version_graph.get_diff(n, c)
								.unwrap() // this unwrap needs to be replaced with ?
								.unwrap(); // this seems to be fine, bc we checked stuff?!

							let insert = barriers.contains(&c);
							let side = DiffSide::A;
							let dir = PropDir::Down;
							let queue_sibling_change_version = c;

							let success = apply_to_diffs(&mut diff, insert, side, mode);

							// TODO: store diff

							if success {
								if options_lenient && !insert {
									queue_sibling_changes(&diff, side, dir, queue_sibling_change_version, mode);
								}

								dirty.insert(c);
							} else {
								// change not applied to this version, propagate further

								// change came down from some version, but
								// could be propagated up to other parents
								propagation.offer(PropDir::Up, c);
								propagation.offer(PropDir::Down, c);
							}
						}
					}
				},
			}
		}
	};

	if op_is_not_none_mappings {
		propagate(Mode::Mappings);
	}
	if op_is_not_none_javadocs {
		propagate(Mode::Javadocs)
	}
}

fn mappings_get_class_or_insert_dummy<'a>(
	mappings: &'a mut Mappings<2>,
	class_key: &'a ClassName
) -> &'a mut ClassNowodeMapping<2> {
	mappings.classes.entry(class_key.clone())
		.or_insert_with_key(|key| {
			// insert dummy mapping
			ClassNowodeMapping::new(ClassMapping::from_key(key.clone()))
		})
}


fn mappings_get_method_or_insert_dummy<'a>(
	class: &'a mut ClassNowodeMapping<2>,
	method_key: &'a MethodNameAndDesc,
) -> &'a mut MethodNowodeMapping<2> {
	class.methods.entry(method_key.clone())
		.or_insert_with_key(|key| {
			// insert dummy mapping
			MethodNowodeMapping::new(MethodMapping::from_key(key.clone()))
		})
}

fn apply_change_5<Key, Diff, Target, Name, Mapping, T> (
	key: &Key,
	change: &Diff,
	parent_children: &mut IndexMap<Key, Target>,
	mode: Mode,
) -> bool
	where
		Key: Debug + Hash + Eq + Clone,
		Diff: NodeInfo<Action<Name>> + NodeJavadocInfo<Action<T>>,
		Target: NodeInfo<Mapping> + NodeJavadocInfo<T>,
		Name: Debug + Clone + PartialEq,
		Mapping: FromKey<Key> + GetNames<2, Name>,
		T: Debug + Clone + PartialEq,
{

	match mode {
		Mode::Mappings => apply_change_5_mappings_impl(key, change, parent_children),
		Mode::Javadocs => apply_change_5_javadoc_impl(key, change, parent_children),
	}
}

fn apply_change_5_mappings_impl<Key, Diff, Target, Name, Mapping>(
	key: &Key,
	change: &Diff,
	parent_children: &mut IndexMap<Key, Target>,
) -> bool
	where
		Key: Debug + Hash + Eq + Clone,
		Diff: NodeInfo<Action<Name>>,
		Target: NodeInfo<Mapping>,
		Name: Debug + Clone + PartialEq,
		Mapping: FromKey<Key> + GetNames<2, Name>,
{
	let second_namespace: Namespace<2> = Namespace::new(1).unwrap();

	match change.get_node_info() {
		Action::Add(b) => {

			let mut info = Mapping::from_key(key.clone());

			info.get_names_mut()[second_namespace] = Some(b.clone());

			let child = Target::new(info);

			match parent_children.entry(key.clone()) {
				Entry::Occupied(e) => {
					eprintln!("ignoring invalid change {:?} to ... - mapping already exists!", change.get_node_info());
					false
				},
				Entry::Vacant(e) => {
					e.insert(child);
					true
				},
			}
		},
		Action::Remove(a) => {
			if parent_children.remove(key).is_some() {
				// TODO: check the removed one?
				true
			} else {
				eprintln!("ignoring invalid change {:?} to ... - mapping already exists!", change.get_node_info());
				false
			}
		},
		Action::Edit(a, b) => {
			if let Some(to_edit) = parent_children.get_mut(key) {

				let result = to_edit.get_node_info_mut()
					.get_names_mut()
					.change_name(second_namespace, Some(a), Some(b));

				match result {
					Ok(_) => true,
					Err(e) => {
						eprintln!("ignoring invalid change {:?} to ... - mapping does not match!", change.get_node_info());
						false
					},
				}
			} else {
				eprintln!("ignoring invalid change {:?} to ... - mapping does not exist!", change.get_node_info());
				false
			}
		},
		Action::None => true,
	}
}

fn apply_change_5_javadoc_impl<Key, Change, Target, T>(
	key: &Key,
	change: &Change,
	parent_children: &mut IndexMap<Key, Target>,
) -> bool
	where
		Key: Eq + Hash,
		Change: NodeJavadocInfo<Action<T>>,
		Target: NodeJavadocInfo<T>,
		T: Debug + Clone + PartialEq,
{
	// copy from apply_diff.rs, modified for &mut as target
	#[allow(clippy::unit_arg)]
	fn apply_diff_option<T>(
		diff: &Option<Action<T>>,
		target: &mut Option<T>,
	) -> Result<()>
		where
			T: Debug + Clone + PartialEq,
	{
		Ok(match diff {
			None => {
				// do nothing
			},
			Some(Action::Add(b)) => {
				if let Some(target) = target {
					bail!("cannot add {b:?} as there's already an existing target {target:?}");
				} else {
					*target = Some(b.clone());
				}
			},
			Some(Action::Remove(a)) => {
				if let Some(target_) = target {
					if target_ != a {
						bail!("cannot remove existing target {target:?} as {a:?}, because they are not equal");
					}
					*target = None;
				} else {
					bail!("cannot remove non existing target {a:?}");
				}
			},
			Some(Action::Edit(a, b)) => {
				if let Some(target_) = target {
					if target_ != a {
						bail!("cannot edit existing target {target:?} as {a:?}, because they are not equal");
					}
					*target = Some(b.clone());
				} else {
					bail!("cannot edit non existing target from {a:?} to {b:?}");
				}
			},
			Some(Action::None) => {
				if target.is_some() {
					// As there is no child, we do not need to propagate down.

					// do nothing
				} else {
					bail!("cannot not action on a non existing target");
				}
			}
		})
	}


	if let Some(target) = parent_children.get_mut(key) {
		let change = change.get_node_javadoc_info();
		let target = target.get_node_javadoc_info_mut();

		match apply_diff_option(change, target) {
			Ok(()) => true,
			Err(e) => {
				eprintln!("ignoring invalid change {:?} to ... - javadoc does not match!", change);
				false
			},
		}
	} else {
		false
	}
}

fn diffs_get_class_or_insert_dummy<'a>(
	diffs: &'a mut MappingsDiff,
	class_key: &'a ClassName
) -> &'a mut ClassNowodeDiff {
	diffs.classes.entry(class_key.clone())
		// insert dummy mapping
		.or_insert_with(|| ClassNowodeDiff::new(Action::None))
}

fn diffs_get_class_or_insert_dummy_if_true<'a>(
	diffs: &'a mut MappingsDiff,
	class_key: &'a ClassName,
	insert: bool,
) -> Option<&'a mut ClassNowodeDiff> {
	match diffs.classes.entry(class_key.clone()) {
		Entry::Occupied(e) => Some(e.into_mut()),
		Entry::Vacant(e) => if insert {
			// insert dummy mapping
			Some(e.insert(ClassNowodeDiff::new(Action::None)))
		} else {
			None
		},
	}
}

fn diffs_get_field_or_insert_dummy<'a>(
	class: &'a mut ClassNowodeDiff,
	field_key: &'a FieldNameAndDesc
) -> &'a mut FieldNowodeDiff {
	class.fields.entry(field_key.clone())
		// insert dummy mapping
		.or_insert_with(|| FieldNowodeDiff::new(Action::None))
}



fn diffs_get_field_or_insert_dummy_if_true<'a>(
	class: &'a mut ClassNowodeDiff,
	field_key: &'a FieldNameAndDesc,
	insert: bool,
) -> Option<&'a mut FieldNowodeDiff> {
	match class.fields.entry(field_key.clone()) {
		Entry::Occupied(e) => Some(e.into_mut()),
		Entry::Vacant(e) => if insert {
			// insert dummy mapping
			Some(e.insert(FieldNowodeDiff::new(Action::None)))
		} else {
			None
		},
	}
}


fn diffs_get_method_or_insert_dummy<'a>(
	class: &'a mut ClassNowodeDiff,
	method_key: &'a MethodNameAndDesc,
) -> &'a mut MethodNowodeDiff {
	class.methods.entry(method_key.clone())
		// insert dummy mapping
		.or_insert_with(|| MethodNowodeDiff::new(Action::None))
}


fn diffs_get_method_or_insert_dummy_if_true<'a>(
	class: &'a mut ClassNowodeDiff,
	method_key: &'a MethodNameAndDesc,
	insert: bool,
) -> Option<&'a mut MethodNowodeDiff> {
	match class.methods.entry(method_key.clone()) {
		Entry::Occupied(e) => Some(e.into_mut()),
		Entry::Vacant(e) => if insert {
			// insert dummy mapping
			Some(e.insert(MethodNowodeDiff::new(Action::None)))
		} else {
			None
		},
	}
}

fn diffs_get_parameter_or_insert_dummy<'a>(
	method: &'a mut MethodNowodeDiff,
	parameter_key: &'a ParameterKey
) -> &'a mut ParameterNowodeDiff {
	method.parameters.entry(parameter_key.clone())
		// insert dummy mapping
		.or_insert_with(|| ParameterNowodeDiff::new(Action::None))
}



fn diffs_get_parameter_or_insert_dummy_if_true<'a>(
	method: &'a mut MethodNowodeDiff,
	parameter_key: &'a ParameterKey,
	insert: bool,
) -> Option<&'a mut ParameterNowodeDiff> {
	match method.parameters.entry(parameter_key.clone()) {
		Entry::Occupied(e) => Some(e.into_mut()),
		Entry::Vacant(e) => if insert {
			// insert dummy mapping
			Some(e.insert(ParameterNowodeDiff::new(Action::None)))
		} else {
			None
		},
	}
}


fn apply_change_to_diff<T: Debug + PartialEq + Clone /* TODO: Clone was necessary bc of action_set impl*/>(
	target: &mut Action<T>,
	change: &Action<T>,
	side: DiffSide,
) -> Result<()> {
	if action_get(target, side) == action_get(change, DiffSide::A) {
		action_set(target, side, action_get(change, DiffSide::B));
		Ok(())
	} else {
		bail!("ignoring invalid change {:?} on {:?} - diff does not mach", change, target)
	}
}


fn apply_change_7<Target, Change, T, U>(
	d: &mut Target,
	change: &Change,
	side: DiffSide,
	insert: bool,
	mode: Mode,
) -> bool
	where
		Target: NodeInfo<Action<T>> + NodeJavadocInfo<Action<U>>,
		Change: NodeInfo<Action<T>> + NodeJavadocInfo<Action<U>>,
		T: Debug + PartialEq + Clone /* TODO: Clone was necessary bc of impl of action_set... */,
		U: PartialEq + Debug + Clone, // TODO: debug and clone bc of impl of called fn
{
	match mode {
		Mode::Mappings => apply_change_7_mappings_impl(d, change, side, insert),
		Mode::Javadocs => apply_change_7_javadoc_impl(d, change, side, insert),
	}
}

fn apply_change_7_mappings_impl<Target, Change, T>(
	d: &mut Target,
	change: &Change,
	side: DiffSide,
	insert: bool,
) -> bool
	where
		Target: NodeInfo<Action<T>>,
		Change: NodeInfo<Action<T>>,
		T: Debug + PartialEq + Clone /* TODO: Clone was necessary bc of impl of action_set... */,
{
	let d = d.get_node_info_mut();
	let change = change.get_node_info();

	if d.is_diff() {
		// it's not a dummy

		match apply_change_to_diff(d, change, side) {
			Ok(()) => true,
			Err(e) => {
				// TODO: shouldn't this return false?
				eprintln!("ignoring invalid change {:?} to ... - diff does not match!", change);
				true
			}
		}
	} else if insert {
		// might be dummy
		action_set(d, side, action_get(change, DiffSide::B));
		action_set(d, side.opposite(), action_get(change, DiffSide::A));
		true
	} else {
		false
	}
}

fn apply_change_7_javadoc_impl<Target, Change, T, U>(
	d: &mut Target,
	change: &Change,
	side: DiffSide,
	insert: bool,
) -> bool
	where
		Target: NodeInfo<Action<T>> + NodeJavadocInfo<Action<U>>,
		Change: NodeJavadocInfo<Action<U>>,
		U: PartialEq + Debug + Clone, // TODO: debug and clone bc of impl of called fn
{
	if is_add_or_remove_on_side(d.get_node_info(), side) ||
		d.get_node_javadoc_info().as_ref().is_some_and(Action::is_diff) ||
		insert
	{
		apply_change_to_diff_optional(side, change.get_node_javadoc_info(), d.get_node_javadoc_info_mut())
	} else {
		false
	}
}

fn apply_change_to_diff_optional<T: Debug + Clone + PartialEq>(
	side: DiffSide,
	change: &Option<Action<T>>,
	target: &mut Option<Action<T>>,
) -> bool {
	if target.as_ref().is_some_and(|x| x.is_diff()) {
		// not a dummy
		if action_get_optional(target, side) == action_get_optional(change, DiffSide::A) {
			action_set_optional(target, side, action_get_optional(change, DiffSide::B));
			true
		} else {
			eprintln!("ignoring invalid change {:?} to ... - diff does not match!", change);
			false
		}
	} else {
		// might be dummy
		action_set_optional(target, side, action_get_optional(change, DiffSide::B));
		action_set_optional(target, side.opposite(), action_get_optional(change, DiffSide::A));
		true
	}
}

struct PropagationQueue<'version> {
	queue_up: VecDeque<VersionEntry<'version>>,
	queue_down: VecDeque<VersionEntry<'version>>,

	version_up: IndexSet<VersionEntry<'version>>,
	version_down: IndexSet<VersionEntry<'version>>,
}

impl<'version> PropagationQueue<'version> {
	fn new<'a>() -> PropagationQueue<'a> {
		PropagationQueue {
			queue_up: Default::default(),
			queue_down: Default::default(),
			version_up: Default::default(),
			version_down: Default::default(),
		}
	}

	fn offer(&mut self, dir: PropDir, version: VersionEntry<'version>) -> bool {
		match dir {
			PropDir::Up => {
				if self.version_up.insert(version) {
					self.queue_up.push_back(version);
					self.queue_up.make_contiguous()
						.sort_by_key(|v| v.depth());
					true
				} else {
					false
				}
			},
			PropDir::Down => {
				if self.version_down.insert(version) {
					self.queue_down.push_back(version);
					self.queue_down.make_contiguous()
						.sort_by_key(|v| v.depth());
					true
				} else {
					false
				}
			},
		}
	}

	fn poll(&mut self) -> Option<(PropDir, VersionEntry<'version>)> {
		if let Some(v) = self.queue_up.pop_front() {
			return Some((PropDir::Up, v))
		}

		if let Some(v) = self.queue_down.pop_front() {
			return Some((PropDir::Down, v))
		}

		None
	}
}

fn is_add_or_remove_on_side<T>(diff: &Action<T>, side: DiffSide) -> bool {
	match side {
		DiffSide::A => matches!(diff, Action::Remove(_)),
		DiffSide::B => matches!(diff, Action::Add(_)),
	}
}

fn action_get<T>(diff: &Action<T>, side: DiffSide) -> Option<&T> {
	match side {
		DiffSide::A => match diff {
			Action::Add(_) => None,
			Action::Remove(a) => Some(a),
			Action::Edit(a, _) => Some(a),
			Action::None => None,
		},
		DiffSide::B => match diff {
			Action::Add(b) => Some(b),
			Action::Remove(_) => None,
			Action::Edit(_, b) => Some(b),
			Action::None => None,
		},
	}
}
fn action_get_optional<T>(diff: &Option<Action<T>>, side: DiffSide) -> Option<&T> {
	match diff {
		Some(diff) => action_get(diff, side),
		None => None,
	}
}

fn action_to_array<T>(action: Action<T>) -> [Option<T>; 2] {
	match action {
		Action::Add(b) => [None, Some(b)],
		Action::Remove(a) => [Some(a), None],
		Action::Edit(a, b) => [Some(a), Some(b)],
		Action::None => [None, None],
	}
}
fn array_to_action<T>(array: [Option<T>; 2]) -> Action<T> {
	match array {
		[None, None] => Action::None,
		[None, Some(b)] => Action::Add(b),
		[Some(a), None] => Action::Remove(a),
		[Some(a), Some(b)] => Action::Edit(a, b),
	}
}

fn action_set<T: Clone>(action: &mut Action<T>, side: DiffSide, value: Option<&T>) {
	let mut array = action_to_array(action.clone());
	match side {
		DiffSide::A => array[0] = value.cloned(),
		DiffSide::B => array[1] = value.cloned(),
	}
	let new_action = array_to_action(array);
	*action = new_action;
}
fn action_set_optional<T: Clone>(action: &mut Option<Action<T>>, side: DiffSide, value: Option<&T>) {
	match action {
		Some(action_) => {
			action_set(action_, side, value);

			if matches!(action_, Action::None) {
				*action = None;
			}
		},
		None => {
			let mut action_ = Action::None;

			action_set(&mut action_, side, value);

			if !matches!(action_, Action::None) {
				*action = Some(action_);
			}
		}
	}
}


fn make_class_name_stem_and_simple(class_name: &ClassName) -> (&JavaStr, &JavaStr) {
	let s = class_name.as_inner();
	s.rfind('/').map_or((JavaStr::from_str(""), s), |i| s.split_at(i))
}


fn get_id_internal(s: &JavaStr) -> &JavaStr {
	let mut chars = s.char_indices().rev().peekable();

	while let Some((i, ch)) = chars.next() {
		if ch == '$' || ch == '/' {
			return &s[i..];
		}

		if ch == '_' {
			if let Some(&(_, prev_ch)) = chars.peek() {
				if prev_ch == 'C' || prev_ch == 'f' || prev_ch == 'm' || prev_ch == 'p' {
					return &s[i..];
				}
			}
		}
	}

	s
}

fn get_id_class(class_key: &ClassName) -> &ClassNameSlice {
	unsafe { ClassNameSlice::from_inner_unchecked(get_id_internal(class_key.as_inner())) }
}
fn get_id_field(field_key: &FieldNameAndDesc) -> &FieldNameSlice {
	unsafe { FieldNameSlice::from_inner_unchecked(get_id_internal(field_key.name.as_inner())) }
}
fn get_id_method(method_key: &MethodNameAndDesc) -> &MethodNameSlice {
	unsafe { MethodNameSlice::from_inner_unchecked(get_id_internal(method_key.name.as_inner())) }
}

fn find_class_sibling<'a>(
	diff: &'a MappingsDiff,
	class_key: &'a ClassName,
	d_class: &'a ClassNowodeDiff,
	change_class: &'a ClassNowodeDiff,
	side: DiffSide,
	mode: Mode,
) -> Option<(&'a ClassName, &'a ClassNowodeDiff)> {
	let id = get_id_class(class_key);
	let siblings: Vec<_> = diff.classes.iter()
		.filter(|(key, diff)| get_id_class(key) == id &&
			key != &class_key &&
			diff.info.is_diff()
		)
		.filter(|(_, sibling)| match mode {
			Mode::Mappings => {
				// for the side that the change was applied to,
				// we need to check against the value before the change
				action_get(&change_class.info, DiffSide::A).is_none() != action_get(&sibling.info, side).is_none() &&
					action_get(&d_class.info, side.opposite()).is_none() != action_get(&sibling.info, side.opposite()).is_none() &&
					{
						let simple = action_get(&change_class.info, DiffSide::A).unwrap()
							.get_simple_name().as_inner();
						let sibling_simple = action_get(&sibling.info, side.opposite()).unwrap()
							.get_simple_name().as_inner();

						let ends_with = if simple.len() > sibling_simple.len() {
							simple.ends_with(sibling_simple)
						} else {
							sibling_simple.ends_with(simple)
						};

						ends_with && action_get(&change_class.info, DiffSide::B).unwrap().as_inner() != sibling_simple
					}
			},
			Mode::Javadocs => {
				// for the side that the change was applied to,
				// we need to check against the value before the change
				action_get_optional(&change_class.javadoc, DiffSide::A).is_none() != action_get_optional(&sibling.javadoc, side).is_none() &&
					action_get_optional(&d_class.javadoc, side.opposite()).is_none() != action_get_optional(&sibling.javadoc, side.opposite()).is_none() &&
					action_get_optional(&change_class.javadoc, DiffSide::A) == action_get_optional(&sibling.javadoc, side.opposite())
			},
		})
		.collect();

	match siblings.len().cmp(&1) {
		Ordering::Less => None,
		Ordering::Equal => {
			let mut siblings = siblings;
			Some(siblings.remove(0))
		},
		Ordering::Greater => panic!("multiple siblings for change: {:?}: {:?}", change_class, siblings),
	}
}

// TODO: search for panic/unwrap

#[allow(clippy::too_many_arguments)]
fn find_field_sibling<'a>(
	class_key: &'a ClassName,
	field_key: &'a FieldNameAndDesc,
	diff: &'a MappingsDiff,
	d_class: &'a ClassNowodeDiff,
	d_field: &'a FieldNowodeDiff,
	change_class: &'a ClassNowodeDiff,
	change_field: &'a FieldNowodeDiff,
	side: DiffSide,
	mode: Mode,
) -> Option<(&'a ClassName, &'a FieldNameAndDesc, &'a FieldNowodeDiff)> {
	let id = get_id_field(field_key);

	let sibling_parent = find_class_sibling(
		diff,
		class_key,
		d_class,
		change_class,
		side,
		mode,
	);

	let siblings: Vec<_> = std::iter::once((class_key, d_class))
		.chain(sibling_parent)
		.flat_map(|(class_key, class)| {
			class.fields.iter()
				.filter(|(key, diff)| get_id_field(key) == id &&
					key != &field_key &&
					diff.info.is_diff()
				)
				.map(move |(field_key, field_diff)| (class_key, field_key, field_diff))
		})
		.filter(|(_, _, sibling)| {
			// for the side that the change was applied to,
			// we need to check against the value before the change
			match mode {
				Mode::Mappings => {
					action_get(&change_field.info, DiffSide::A).is_none() != action_get(&sibling.info, side).is_none() &&
						action_get(&d_field.info, side.opposite()).is_none() != action_get(&sibling.info, side.opposite()).is_none() &&
						action_get(&change_field.info, DiffSide::A) == action_get(&sibling.info, side.opposite())
				},
				Mode::Javadocs => {
					action_get_optional(&change_field.javadoc, DiffSide::A).is_none() != action_get_optional(&sibling.javadoc, side).is_none() &&
						action_get_optional(&d_field.javadoc, side.opposite()).is_none() != action_get_optional(&sibling.javadoc, side.opposite()).is_none() &&
						action_get_optional(&change_field.javadoc, DiffSide::A) == action_get_optional(&sibling.javadoc, side.opposite())
				},
			}
		})
		.collect();

	match siblings.len().cmp(&1) {
		Ordering::Less => None,
		Ordering::Equal => {
			let mut siblings = siblings;
			Some(siblings.remove(0))
		},
		Ordering::Greater => manually_select_item(d_field, siblings, |(_, x, _)| x),
	}
}


#[allow(clippy::too_many_arguments)]
fn find_method_sibling<'a>(
	class_key: &'a ClassName,
	method_key: &'a MethodNameAndDesc,
	diff: &'a MappingsDiff,
	d_class: &'a ClassNowodeDiff,
	d_method: &'a MethodNowodeDiff,
	change_class: &'a ClassNowodeDiff,
	change_method: &'a MethodNowodeDiff,
	side: DiffSide,
	mode: Mode,
) -> Option<(&'a ClassName, &'a MethodNameAndDesc, &'a MethodNowodeDiff)> {
	let id = get_id_method(method_key);

	let sibling_parent = find_class_sibling(
		diff,
		class_key,
		d_class,
		change_class,
		side,
		mode,
	);

	let siblings: Vec<_> = std::iter::once((class_key, d_class))
		.chain(sibling_parent)
		.flat_map(|(class_key, class)| {
			class.methods.iter()
				.filter(|(key, diff)| get_id_method(key) == id &&
					key != &method_key &&
					diff.info.is_diff()
				)
				.map(move |(method_key, method_diff)| (class_key, method_key, method_diff))
		})
		.filter(|(_, _, sibling)| {
			// for the side that the change was applied to,
			// we need to check against the value before the change
			match mode {
				Mode::Mappings => {
					action_get(&change_method.info, DiffSide::A).is_none() != action_get(&sibling.info, side).is_none() &&
						action_get(&d_method.info, side.opposite()).is_none() != action_get(&sibling.info, side.opposite()).is_none() &&
						action_get(&change_method.info, DiffSide::A) == action_get(&sibling.info, side.opposite())
				},
				Mode::Javadocs => {
					action_get_optional(&change_method.javadoc, DiffSide::A).is_none() != action_get_optional(&sibling.javadoc, side).is_none() &&
						action_get_optional(&d_method.javadoc, side.opposite()).is_none() != action_get_optional(&sibling.javadoc, side.opposite()).is_none() &&
						action_get_optional(&change_method.javadoc, DiffSide::A) == action_get_optional(&sibling.javadoc, side.opposite())
				},
			}
		})
		.collect();

	match siblings.len().cmp(&1) {
		Ordering::Less => None,
		Ordering::Equal => {
			let mut siblings = siblings;
			Some(siblings.remove(0))
		},
		Ordering::Greater => manually_select_item(d_method, siblings, |(_, x, _)| x),
	}
}

fn manually_select_item<T, F: Debug>(
	d: impl Debug,
	mut vec: Vec<T>,
	f: impl Fn(&T) -> &F,
) -> Option<T> {
	println!("multiple propagation candidates for {:?}", d);
	for (i, item) in vec.iter().enumerate() {
		println!("{}: {:?}", i, f(item))
	}
	println!("{}: none", vec.len());
	loop {
		let mut cmd = String::new();
		match std::io::stdin().read_line(&mut cmd) {
			Ok(_) => {},
			Err(e) => {
				println!("error reading line: {e:?}");
				continue;
			}
		}
		let i: usize = match cmd.trim_end().parse() {
			Ok(i) => i,
			Err(e) => {
				println!("error parsing input: {e:?}");
				println!("please enter a number from the list above");
				continue;
			},
		};

		if (0..vec.len()).contains(&i) {
			let sibling = vec.swap_remove(i);
			println!("chose {:?}", f(&sibling));
			return Some(sibling);
		}
		if vec.len() == i {
			println!("chose none");
			return None;
		}

		println!("number out of range! - please try again");
	}
}
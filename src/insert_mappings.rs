use std::cmp::Ordering;
use std::hash::Hash;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::{IndexMap, IndexSet};
use crate::PropagationDirection;
use std::collections::{HashSet, VecDeque};
use std::fmt::Debug;
use indexmap::map::Entry;
use java_string::{JavaStr, JavaString};
use duke::tree::class::ObjClassName;
use duke::tree::field::FieldNameAndDesc;
use duke::tree::method::MethodNameAndDesc;
use quill::tree::mappings::Mappings;
use quill::tree::mappings_diff::{Action, ClassNowodeDiff, FieldNowodeDiff, MappingsDiff, MethodNowodeDiff};
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

enum Changed {
	Same,
	Edited,
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
	propagation_direction: PropagationDirection,
	propagate_lenient: bool,
	version_graph: &'version VersionGraph,
	changes: MappingsDiff,
	version: VersionEntry<'version>,
) -> Result<()> {

	let (direction_is_up, direction_is_down) = match propagation_direction {
		PropagationDirection::None => (false, false),
		PropagationDirection::Both => (true, true),
		PropagationDirection::Up => (true, false),
		PropagationDirection::Down => (false, true),
	};

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
					propagate_lenient,
					&mut dirty,
					&barriers,
					version_graph,
					version,
					change_class.info.is_diff(),
					change_class.javadoc.is_diff(),
					|mappings, mode| {
						apply_change_mappings(class_key, change_class, &mut mappings.classes, mode)
					},
					|diff, insert, side, mode| {
						diff.classes
							.get_mut_or_default_if(class_key, insert)
							.map_or(Ok(Changed::Same), |diff_class| {
								apply_change_diffs(diff_class, change_class, side, insert, mode)
							})
					},
					|diff, side, dir, queue_sibling_change_version, mode| {
						let diff_class = diff.classes.get(class_key).unwrap(); // the above closure created it already

						// mapping (does not) exists on both sides
						// so do not try to propagate to siblings
						if get(&change_class.info, DiffSide::A).is_none() != get(&diff_class.info, side.opposite()).is_none(){
							let sibling = find_class_sibling(
								diff,
								class_key,
								diff_class,
								change_class,
								side,
								mode
							)?;
							// post condition here: depending on lengths, one.ends_with(another) for simple name of the opposite side of the returned sibling,
							// and the simple name of `a` of change_class

							if let Some((sibling_class_key, sibling)) = sibling {
								let side = side.opposite();

								let mut do_queue_changes = |version: VersionEntry<'version>| {
									let sibling_class = queued_changes
										.entry(version).or_default()
										.classes
										.entry(sibling_class_key.clone()).or_default();

									match mode {
										Mode::Mappings => {
											let Action::Edit(ofrom, to) = &change_class.info else {
												panic!("originally this was an unwrap");
											};

											// post condition from above: this is the one from sibling, and `ofrom` is the one from change_class
											let from = get(&sibling.info, side).unwrap();

											// ofrom = "OS" // "net/minecraft/client/Minecraft$OS"
											// to = "OperatingSystem" // "net/minecraft/client/Minecraft$OperatingSystem"
											// from = "net/minecraft/isom/IsomPreviewCanvas__OS"?

											fn make_class_name_stem_and_simple(class_name: &ObjClassName) -> (&JavaStr, &JavaStr) {
												let s = class_name.as_inner();
												s.rfind('/').map_or((JavaStr::from_str(""), s), |i| s.split_at(i))
											}

											let (ofrom_stem, ofrom_simple) = make_class_name_stem_and_simple(ofrom);
											let (from_stem, from_simple) = make_class_name_stem_and_simple(from);
											let (to_stem, to_simple) = make_class_name_stem_and_simple(to);

											// ofrom_stem = "", ofrom_simple = "OS"
											// from_stem = "net/minecraft/isom/", from_simple = "IsomPreviewCanvas__OS"
											// to_stem = "", to_simple = "OperatingSystem"

											let to_inner = match ofrom_simple.len().cmp(&from_simple.len()) {
												Ordering::Less => {
													// the post condition from the find_class_sibling fn
													assert!(from_simple.ends_with(ofrom_simple));

													// Case where `from` uses `__`  for inner classes, `ofrom`/`to` is only the inner class name (on inner classes)

													let a = from_simple.strip_suffix(ofrom_simple)
														.unwrap(); // see assert above
													// ("IsomPreviewCanvas__", "OperatingSystem")
													let (a, b) = (a, to_simple);

													let mut to_inner = JavaString::from(a);
													to_inner.push_java_str(b);
													to_inner
												},
												Ordering::Equal => {
													// the post condition from the find_class_sibling fn
													assert_eq!(from_simple, ofrom_simple);

													// This is the case where the way inner classes are represented doesn't change.

													// ("", "OperatingSystem") -> "OperatingSystem"
													to_simple.to_owned()
												},
												Ordering::Greater => {
													// the post condition from the find_class_sibling fn
													assert!(ofrom_simple.ends_with(from_simple));

													// Case where `ofrom`/`to` use `__` for inner classes, `from` is only the inner class name (on inner classes)

													// here consider
													// ofrom_stem = "net/minecraft/isom/", ofrom_simple = "IsomPreviewCanvas__OS"
													// from_stem = "", from_simple = "OS"
													// to_stem = "net/minecraft/isom/", to_simple = "IsomPreviewCanvas__OperatingSystem"
													// gives us
													// ("", "OperatingSystem")
													let (a, b) = (from_stem, &to_simple[(ofrom_simple.len() - from_simple.len())..]);


													let mut to_inner = JavaString::from(a);
													to_inner.push_java_str(b);
													to_inner
												},
											};

											let to_new = unsafe { ObjClassName::from_inner_unchecked(to_inner) };

											sibling_class.info = Action::from_tuple(Some(from.clone()), Some(to_new));
										},
										Mode::Javadocs => {
											let from = get(&sibling.javadoc, side);
											let (_, to) = &change_class.javadoc.as_ref().to_tuple();
											sibling_class.javadoc = Action::from_tuple(from.cloned(), to.cloned());
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

						Ok(())
					},
				)?;


				for (field_key, change_field) in &change_class.fields {
					propagate_change(
						propagate_lenient,
						&mut dirty,
						&barriers,
						version_graph,
						version,
						change_field.info.is_diff(),
						change_field.javadoc.is_diff(),
						|mappings, mode| {
							let mappings_class = mappings.classes
								.entry(class_key.clone()).or_insert_with_key(create_dummy_mapping);
							apply_change_mappings(field_key, change_field, &mut mappings_class.fields, mode)
						},
						|diff, insert, side, mode| {
							diff.classes
								.entry(class_key.clone()).or_default()
								.fields
								.get_mut_or_default_if(field_key, insert)
								.map_or(Ok(Changed::Same), |diff_field| {
									apply_change_diffs(diff_field, change_field, side, insert, mode)
								})
						},
						|diff, side, dir, queue_sibling_change_version, mode| {
							let diff_class = diff.classes.get(class_key).unwrap(); // the above closure created it already
							let diff_field = diff_class.fields.get(field_key).unwrap(); // the above closure created it already

							// mapping (does not) exists on both sides
							// so do not try to propagate to siblings
							if get(&change_field.info, DiffSide::A).is_none() != get(&diff_field.info, side.opposite()).is_none(){
								let sibling = find_field_sibling(
									class_key,
									field_key,
									diff,
									diff_class,
									diff_field,
									change_class,
									change_field,
									side,
									mode
								)?;

								if let Some((sibling_class_key, sibling_field_key, sibling)) = sibling {
									let side = side.opposite();


									let mut do_queue_changes = |version: VersionEntry<'version>| {
										let sibling_field = queued_changes
											.entry(version).or_default()
											.classes
											.entry(sibling_class_key.clone()).or_default()
											.fields
											.entry(sibling_field_key.clone()).or_default();

										match mode {
											Mode::Mappings => {
												let from = get(&sibling.info, side);
												let (_, to) = change_field.info.as_ref().to_tuple();
												sibling_field.info = Action::from_tuple(from.cloned(), to.cloned());
											},
											Mode::Javadocs => {
												let from = get(&sibling.javadoc, side);
												let (_, to) = change_field.javadoc.as_ref().to_tuple();
												sibling_field.javadoc = Action::from_tuple(from.cloned(), to.cloned());
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

							Ok(())
						},
					)?;

					// has no children
				}
				for (method_key, change_method) in &change_class.methods {
					propagate_change(
						propagate_lenient,
						&mut dirty,
						&barriers,
						version_graph,
						version,
						change_method.info.is_diff(),
						change_method.javadoc.is_diff(),
						|mappings, mode| {
							let mappings_class = mappings.classes
								.entry(class_key.clone()).or_insert_with_key(create_dummy_mapping);
							apply_change_mappings(method_key, change_method, &mut mappings_class.methods, mode)
						},
						|diff, insert, side, mode| {
							diff.classes
								.entry(class_key.clone()).or_default()
								.methods
								.get_mut_or_default_if(method_key, insert)
								.map_or(Ok(Changed::Same), |diff_method| {
									apply_change_diffs(diff_method, change_method, side, insert, mode)
								})
						},
						|diff, side, dir, queue_sibling_change_version, mode| {
							let diff_class = diff.classes.get(class_key).unwrap(); // the above closure created it already
							let diff_method = diff_class.methods.get(method_key).unwrap(); // the above closure created it already

							// mapping (does not) exists on both sides
							// so do not try to propagate to siblings
							if get(&change_method.info, DiffSide::A).is_none() != get(&diff_method.info, side.opposite()).is_none(){
								let sibling = find_method_sibling(
									class_key,
									method_key,
									diff,
									diff_class,
									diff_method,
									change_class,
									change_method,
									side,
									mode
								)?;

								if let Some((parent_sibling_key, sibling_key, sibling)) = sibling {
									let side = side.opposite();

									let mut do_queue_changes = |version: VersionEntry<'version>| {
										let sibling_method = queued_changes
											.entry(version).or_default()
											.classes
											.entry(parent_sibling_key.clone()).or_default()
											.methods
											.entry(sibling_key.clone()).or_default();

										match mode {
											Mode::Mappings => {
												let from = get(&sibling.info, side);
												let (_, to) = change_method.info.as_ref().to_tuple();
												sibling_method.info = Action::from_tuple(from.cloned(), to.cloned());
											},
											Mode::Javadocs => {
												let from = get(&sibling.javadoc, side);
												let (_, to) = change_method.javadoc.as_ref().to_tuple();
												sibling_method.javadoc = Action::from_tuple(from.cloned(), to.cloned());
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

							Ok(())
						}
					)?;

					for (parameter_key, change_parameter) in &change_method.parameters {
						propagate_change(
							propagate_lenient,
							&mut dirty,
							&barriers,
							version_graph,
							version,
							change_parameter.info.is_diff(),
							change_parameter.javadoc.is_diff(),
							|mappings, mode| {
								let mappings_method = mappings.classes
									.entry(class_key.clone()).or_insert_with_key(create_dummy_mapping)
									.methods
									.entry(method_key.clone()).or_insert_with_key(create_dummy_mapping);
								apply_change_mappings(parameter_key, change_parameter, &mut mappings_method.parameters, mode)
							},
							|diff, insert, side, mode| {
								diff.classes
									.entry(class_key.clone()).or_default()
									.methods
									.entry(method_key.clone()).or_default()
									.parameters
									.get_mut_or_default_if(parameter_key, insert)
									.map_or(Ok(Changed::Same), |diff_parameter| {
										apply_change_diffs(diff_parameter, change_parameter, side, insert, mode)
									})
							},
							|_diff, _side, _dir, _queue_sibling_change_version, _mode| {
								// parameters don't queue siblings
								Ok(())
							},
						)?;

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
	apply_to_mappings: impl Fn(&mut Mappings<2>, Mode) -> Result<Changed>,
	apply_to_diffs: impl Fn(&mut MappingsDiff, bool, DiffSide, Mode) -> Result<Changed>,
	mut queue_sibling_changes: impl FnMut(&MappingsDiff, DiffSide, PropDir, VersionEntry<'version>, Mode) -> Result<()>,
) -> Result<()> {
	let mut propagate = |mode: Mode| -> Result<()> {
		let mut propagation = PropagationQueue::default();
		propagation.offer_up(version);

		fn handle_error_to_success_bool(x: Result<Changed>) -> bool {
			match x {
				Ok(x) => match x {
					Changed::Same => false,
					Changed::Edited => true,
				},
				Err(e) => {
					eprintln!("{e:?}");
					eprintln!("ignoring invalid change");
					false
				},
			}
		}

		while let Some((dir, n)) = propagation.poll() {
			match dir {
				PropDir::Up => {
					if let Some(mappings) = version_graph.is_root_then_get_mappings(n) {
						let mut mappings = mappings.clone();
						// TODO: store mappings across multiple times

						let success = handle_error_to_success_bool(apply_to_mappings(&mut mappings, mode));

						if success {
							// success, now propagate in opposite direction
							propagation.offer_down(n);

							dirty.insert(n);
						}
					} else {
						let side = DiffSide::B;
						let insert = barriers.contains(&n);
						let dir = PropDir::Up;
						let queue_sibling_change_version = n;

						for p in version_graph.parents(n) {
							let mut diff = version_graph.get_diff(p, n)?
								.unwrap(); // this is fine bc between a parent and child there's always a diff
							// TODO: store diffs across multiple times

							let success = handle_error_to_success_bool(apply_to_diffs(&mut diff, insert, side, mode));

							if success {
								if options_lenient && !insert {
									queue_sibling_changes(&diff, side, dir, queue_sibling_change_version, mode)?;
								}

								// change applied, now propagate in the opposite direction
								propagation.offer_down(n);

								dirty.insert(n);
							} else {
								// change not applied to this version, propagate further
								propagation.offer_up(p);
							}
						}
					}
				},
				PropDir::Down => {
					for c in version_graph.children(n) {
						if let Some(mappings) = version_graph.is_root_then_get_mappings(c) {
							let mut mappings = mappings.clone();

							let success = handle_error_to_success_bool(apply_to_mappings(&mut mappings, mode));

							// TODO: store mappings

							if success {
								dirty.insert(c);
							}
						} else {
							// loop could be removed since p is always equal to s, bc of parent/child symmetry

							let mut diff = version_graph.get_diff(n, c)?
								.unwrap(); // this is fine bc between parent and child there's always a diff

							let insert = barriers.contains(&c);
							let side = DiffSide::A;
							let dir = PropDir::Down;
							let queue_sibling_change_version = c;

							let success = handle_error_to_success_bool(apply_to_diffs(&mut diff, insert, side, mode));

							// TODO: store diff

							if success {
								if options_lenient && !insert {
									queue_sibling_changes(&diff, side, dir, queue_sibling_change_version, mode)?;
								}

								dirty.insert(c);
							} else {
								// change not applied to this version, propagate further

								// change came down from some version, but
								// could be propagated up to other parents
								propagation.offer_up(c);
								propagation.offer_down(c);
							}
						}
					}
				},
			}
		}

		Ok(())
	};

	if op_is_not_none_mappings {
		propagate(Mode::Mappings)?;
	}
	if op_is_not_none_javadocs {
		propagate(Mode::Javadocs)?;
	}

	Ok(())
}

/// Clearly marks adding a dummy mapping.
fn create_dummy_mapping<Key, Node, Info>(key: &Key) -> Node
	where
		Key: Clone,
		Node: NodeInfo<Info>,
		Info: FromKey<Key>,
{
	NodeInfo::new(FromKey::from_key(key.clone()))
}

fn apply_change_mappings<Key, Diff, Target, Name, Mapping, Javadoc> (
	key: &Key,
	change: &Diff,
	parent_children: &mut IndexMap<Key, Target>,
	mode: Mode,
) -> Result<Changed>
	where
		Key: Debug + Hash + Eq + Clone,
		Diff: NodeInfo<Action<Name>> + NodeJavadocInfo<Action<Javadoc>>,
		Target: NodeInfo<Mapping> + NodeJavadocInfo<Option<Javadoc>>,
		Name: Debug + Clone + PartialEq,
		Mapping: FromKey<Key> + GetNames<2, Name>,
		Javadoc: Debug + Clone + PartialEq,
{
	match mode {
		Mode::Mappings => apply_change_mappings_mappings_impl(key, change, parent_children)
			.with_context(|| anyhow!("on key {key:?} for change {:?}", change.get_node_info())),
		Mode::Javadocs => apply_change_mappings_javadoc_impl(key, change, parent_children)
			.with_context(|| anyhow!("on key {key:?} for javadoc change {:?}", change.get_node_javadoc_info())),
	}
}

fn apply_change_mappings_mappings_impl<Key, Diff, Target, Name, Mapping>(
	key: &Key,
	change: &Diff,
	parent_children: &mut IndexMap<Key, Target>,
) -> Result<Changed>
	where
		Key: Debug + Hash + Eq + Clone,
		Diff: NodeInfo<Action<Name>>,
		Target: NodeInfo<Mapping>,
		Name: Debug + Clone + PartialEq,
		Mapping: FromKey<Key> + GetNames<2, Name>,
{
	let second_namespace: Namespace<2> = Namespace::new(1).unwrap();

	match change.get_node_info() {
		Action::None => Ok(Changed::Same),
		Action::Add(b) => {
			let mut info = Mapping::from_key(key.clone());

			info.get_names_mut()[second_namespace] = Some(b.clone());

			let child = Target::new(info);

			match parent_children.entry(key.clone()) {
				Entry::Occupied(e) => bail!("mapping for key {:?} already exists", e.key()),
				Entry::Vacant(e) => {
					e.insert(child);
					Ok(Changed::Edited)
				},
			}
		},
		Action::Remove(a) => {
			if parent_children.remove(key).is_some() {
				// TODO: check the removed one?
				Ok(Changed::Edited)
			} else {
				bail!("mapping already exists");
			}
		},
		Action::Edit(a, b) => {
			parent_children.get_mut(key)
				.with_context(|| anyhow!("mapping for key {key:?} does not exist"))
				.and_then(|to_edit| to_edit
					.get_node_info_mut()
					.get_names_mut()
					.change_name(second_namespace, Some(a), Some(b)).map(|_| Changed::Edited)
					.with_context(|| anyhow!("mapping for key {key:?} does not match"))
				)
		},
	}
}

fn apply_change_mappings_javadoc_impl<Key, Change, Target, T>(
	key: &Key,
	change: &Change,
	parent_children: &mut IndexMap<Key, Target>,
) -> Result<Changed>
	where
		Key: Eq + Hash,
		Change: NodeJavadocInfo<Action<T>>,
		Target: NodeJavadocInfo<Option<T>>,
		T: Debug + Clone + PartialEq,
{
	if let Some(target) = parent_children.get_mut(key) {
		let change = change.get_node_javadoc_info();
		let target = target.get_node_javadoc_info_mut();

		quill::apply_diff_option(change, target.take())
			.context("javadoc does not match")
			.map(|new| {
				*target = new;
				Changed::Edited
			})
	} else {
		Ok(Changed::Same)
	}
}

trait MapGetOrDefaultIf<K, V> {
	fn get_mut_or_default_if<'a>(&'a mut self, key: &'a K, insert: bool) -> Option<&'a mut V>;
}

impl<K, V> MapGetOrDefaultIf<K, V> for IndexMap<K, V>
	where
		K: Eq + Hash + Clone,
		V: Default,
{
	fn get_mut_or_default_if<'a>(&'a mut self, key: &'a K, insert: bool) -> Option<&'a mut V> {
		if insert {
			// insert dummy mapping
			Some(self.entry(key.clone()).or_default())
		} else {
			self.get_mut(key)
		}
	}
}

fn apply_change_to_diff<T: Debug + PartialEq + Clone /* TODO: Clone was necessary bc of cloned call */>(
	target: &mut Action<T>,
	change: &Action<T>,
	side: DiffSide,
) -> Result<()> {
	let (change_a, change_b) = change.as_ref().to_tuple();

	match side {
		DiffSide::A => {
			if target.as_ref().to_tuple().0 == change_a {
				let value = change_b.cloned();
				let b = std::mem::take(target).to_tuple().1;
				*target = Action::from_tuple(value, b);
				Ok(())
			} else {
				bail!("ignoring invalid change {:?} on {:?} - diff does not mach", change, target)
			}
		},
		DiffSide::B => {
			if target.as_ref().to_tuple().1 == change_a {
				let value = change_b.cloned();
				let a = std::mem::take(target).to_tuple().0;
				*target = Action::from_tuple(a, value);
				Ok(())
			} else {
				bail!("ignoring invalid change {:?} on {:?} - diff does not mach", change, target)
			}
		},
	}
}

fn apply_change_diffs<Target, Change, T, U>(
	d: &mut Target,
	change: &Change,
	side: DiffSide,
	insert: bool,
	mode: Mode,
) -> Result<Changed>
	where
		Target: NodeInfo<Action<T>> + NodeJavadocInfo<Action<U>>,
		Change: NodeInfo<Action<T>> + NodeJavadocInfo<Action<U>>,
		T: Debug + PartialEq + Clone /* TODO: Clone was necessary bc of impl of action_set... */,
		U: PartialEq + Debug + Clone, // TODO: debug and clone bc of impl of called fn
{
	match mode {
		Mode::Mappings => {
			let target = d.get_node_info_mut();
			let change = change.get_node_info();

			if target.is_diff() {
				// it's not a dummy
				apply_change_to_diff(target, change, side).map(|()| Changed::Edited)
			} else if insert {
				// might be dummy
				match side {
					DiffSide::A => *target = change.clone().flip(),
					DiffSide::B => *target = change.clone(),
				}
				Ok(Changed::Edited)
			} else {
				Ok(Changed::Same)
			}
		},
		Mode::Javadocs => {
			if is_add_or_remove_on_side(d.get_node_info(), side) || // if the thing gets added/removed, we must also store the javadoc
				d.get_node_javadoc_info().is_diff() ||
				insert
			{
				let target = d.get_node_javadoc_info_mut();
				let change = change.get_node_javadoc_info();

				if target.is_diff() {
					// not a dummy
					apply_change_to_diff(target, change, side).map(|()| Changed::Edited)
				} else {
					match side {
						DiffSide::A => *target = change.clone().flip(),
						DiffSide::B => *target = change.clone(),
					}
					Ok(Changed::Edited)
				}
			} else {
				Ok(Changed::Same)
			}
		},
	}
}

#[derive(Default)]
struct PropagationQueue<'version> {
	queue_up: VecDeque<VersionEntry<'version>>,
	queue_down: VecDeque<VersionEntry<'version>>,

	version_up: IndexSet<VersionEntry<'version>>,
	version_down: IndexSet<VersionEntry<'version>>,
}

impl<'version> PropagationQueue<'version> {
	fn offer_up(&mut self, version: VersionEntry<'version>) {
		if self.version_up.insert(version) {
			self.queue_up.push_back(version);
			self.queue_up.make_contiguous()
				.sort_by_key(|v| v.depth());
		}
	}


	fn offer_down(&mut self, version: VersionEntry<'version>) {
		if self.version_down.insert(version) {
			self.queue_down.push_back(version);
			self.queue_down.make_contiguous()
				.sort_by_key(|v| v.depth());
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

fn get<T>(diff: &Action<T>, side: DiffSide) -> Option<&T> {
	let (a, b) = diff.as_ref().to_tuple();
	match side {
		DiffSide::A => a,
		DiffSide::B => b,
	}
}

fn swap_if_side_b<T>(side: DiffSide, ab: (T, T)) -> (T, T) {
	match side {
		DiffSide::A => ab,
		DiffSide::B => {
			let (a, b) = ab;
			(b, a)
		},
	}
}

fn get_id_class(class_key: &ObjClassName) -> &JavaStr {
	let s = class_key.as_inner();

	let last_section = s.rsplit_once('/')
		.map_or(s, |(_, last_section)| last_section);

	last_section.rsplit_once("C_")
		.or_else(|| s.rsplit_once('$'))
		.map_or(last_section, |(_, id)| id)
}
fn get_id_field(field_key: &FieldNameAndDesc) -> &JavaStr {
	let s = field_key.name.as_inner();
	s.rsplit_once("f_").map_or(s, |(_, id)| id)
}
fn get_id_method(method_key: &MethodNameAndDesc) -> &JavaStr {
	let s = method_key.name.as_inner();
	s.rsplit_once("m_").map_or(s, |(_, id)| id)
}

/// postconditions:
///
/// `change_class` the side `a` get the simple name, call that `simple`
///
/// the returned sibling the opposite of `side` get the simple name, call that `sibling_simple`
///
/// guarantees: either `simple.ends_with(sibling_simple)` or `sibling_simple.ends_with(simple)` (choosing the longer one for the `self` arg)
fn find_class_sibling<'a>(
	diff: &'a MappingsDiff,
	class_key: &'a ObjClassName,
	diff_class: &'a ClassNowodeDiff,
	change_class: &'a ClassNowodeDiff,
	side: DiffSide,
	mode: Mode,
) -> Result<Option<(&'a ObjClassName, &'a ClassNowodeDiff)>> {
	let id = get_id_class(class_key);
	let siblings: Vec<_> = diff.classes.iter()
		.filter(|(key, diff)| get_id_class(key) == id && key != &class_key && diff.info.is_diff())
		.filter(|(_, sibling)| match mode {
			Mode::Mappings => {
				let (class_change_a, class_change_b) = change_class.info.as_ref().to_tuple();
				let (sibling_side, sibling_side_op) = swap_if_side_b(side, sibling.info.as_ref().to_tuple());
				let diff_class_side_op = get(&diff_class.info, side.opposite());

				// for the side that the change was applied to,
				// we need to check against the value before the change
				class_change_a.is_none() != sibling_side.is_none() &&
					diff_class_side_op.is_none() != sibling_side_op.is_none() &&
					{
						// The simple names must end with each other. This catches cases where Inner was renamed to Outer__Inner, and the other way around.
						let simple = class_change_a.unwrap().get_simple_name().as_inner();
						let sibling_simple = sibling_side_op.unwrap().get_simple_name().as_inner();

						let ends_with = match simple.len().cmp(&sibling_simple.len()) {
							Ordering::Less => sibling_simple.ends_with(simple),
							Ordering::Equal => sibling_simple == simple,
							Ordering::Greater => simple.ends_with(sibling_simple),
						};

						ends_with && class_change_b.unwrap().as_inner() != sibling_simple
					}
			},
			Mode::Javadocs => {
				let (class_change_a, _) = change_class.javadoc.as_ref().to_tuple();
				let (sibling_side, sibling_side_op) = swap_if_side_b(side, sibling.javadoc.as_ref().to_tuple());
				let diff_class_side_op = get(&diff_class.javadoc, side.opposite());
				// for the side that the change was applied to,
				// we need to check against the value before the change
				class_change_a.is_none() != sibling_side.is_none() &&
					diff_class_side_op.is_none() != sibling_side_op.is_none() &&
					class_change_a == sibling_side_op
			},
		})
		.collect();

	match siblings.len().cmp(&1) {
		Ordering::Less => Ok(None),
		Ordering::Equal => {
			let mut siblings = siblings;
			Ok(Some(siblings.remove(0)))
		},
		Ordering::Greater => bail!("multiple siblings for change: {:?}: {:?}", change_class, siblings),
	}
}

// TODO: search for panic/unwrap

#[allow(clippy::too_many_arguments)]
fn find_field_sibling<'a>(
	class_key: &'a ObjClassName,
	field_key: &'a FieldNameAndDesc,
	diff: &'a MappingsDiff,
	diff_class: &'a ClassNowodeDiff,
	diff_field: &'a FieldNowodeDiff,
	change_class: &'a ClassNowodeDiff,
	change_field: &'a FieldNowodeDiff,
	side: DiffSide,
	mode: Mode,
) -> Result<Option<(&'a ObjClassName, &'a FieldNameAndDesc, &'a FieldNowodeDiff)>> {
	let id = get_id_field(field_key);

	let sibling_parent = find_class_sibling(
		diff,
		class_key,
		diff_class,
		change_class,
		side,
		mode,
	)?;

	let siblings: Vec<_> = std::iter::once((class_key, diff_class))
		.chain(sibling_parent)
		.flat_map(|(class_key, class)| {
			class.fields.iter()
				.filter(|(key, diff)| get_id_field(key) == id && key != &field_key && diff.info.is_diff())
				.map(move |(field_key, field_diff)| (class_key, field_key, field_diff))
		})
		.filter(|(_, _, sibling)| {
			// for the side that the change was applied to,
			// we need to check against the value before the change
			match mode {
				Mode::Mappings => {
					let (field_change_a, _) = change_field.info.as_ref().to_tuple();
					let (sibling_side, sibling_side_op) = swap_if_side_b(side, sibling.info.as_ref().to_tuple());
					let diff_field_side_op = get(&diff_field.info, side.opposite());
					
					field_change_a.is_none() != sibling_side.is_none() &&
						diff_field_side_op.is_none() != sibling_side_op.is_none() &&
						field_change_a == sibling_side_op
				},
				Mode::Javadocs => {
					let (field_change_a, _) = change_field.javadoc.as_ref().to_tuple();
					let (sibling_side, sibling_side_op) = swap_if_side_b(side, sibling.javadoc.as_ref().to_tuple());
					let diff_field_side_op = get(&diff_field.javadoc, side.opposite());
					field_change_a.is_none() != sibling_side.is_none() &&
						diff_field_side_op.is_none() != sibling_side_op.is_none() &&
						field_change_a == sibling_side_op
				},
			}
		})
		.collect();

	Ok(match siblings.len().cmp(&1) {
		Ordering::Less => None,
		Ordering::Equal => {
			let mut siblings = siblings;
			Some(siblings.remove(0))
		},
		Ordering::Greater => manually_select_item(diff_field, siblings, |(_, x, _)| x),
	})
}


#[allow(clippy::too_many_arguments)]
fn find_method_sibling<'a>(
	class_key: &'a ObjClassName,
	method_key: &'a MethodNameAndDesc,
	diff: &'a MappingsDiff,
	diff_class: &'a ClassNowodeDiff,
	diff_method: &'a MethodNowodeDiff,
	change_class: &'a ClassNowodeDiff,
	change_method: &'a MethodNowodeDiff,
	side: DiffSide,
	mode: Mode,
) -> Result<Option<(&'a ObjClassName, &'a MethodNameAndDesc, &'a MethodNowodeDiff)>> {
	let id = get_id_method(method_key);

	let sibling_parent = find_class_sibling(
		diff,
		class_key,
		diff_class,
		change_class,
		side,
		mode,
	)?;

	let siblings: Vec<_> = std::iter::once((class_key, diff_class))
		.chain(sibling_parent)
		.flat_map(|(class_key, class)| {
			class.methods.iter()
				.filter(|(key, diff)| get_id_method(key) == id && key != &method_key && diff.info.is_diff())
				.map(move |(method_key, method_diff)| (class_key, method_key, method_diff))
		})
		.filter(|(_, _, sibling)| {
			// for the side that the change was applied to,
			// we need to check against the value before the change
			match mode {
				Mode::Mappings => {
					let (method_change_a, _) = change_method.info.as_ref().to_tuple();
					let (sibling_side, sibling_side_op) = swap_if_side_b(side, sibling.info.as_ref().to_tuple());
					let diff_method_side_op = get(&diff_method.info, side.opposite());

					method_change_a.is_none() != sibling_side.is_none() &&
						diff_method_side_op.is_none() != sibling_side_op.is_none() &&
						method_change_a == sibling_side_op
				},
				Mode::Javadocs => {
					let (method_change_a, _) = change_method.javadoc.as_ref().to_tuple();
					let (sibling_side, sibling_side_op) = swap_if_side_b(side, sibling.javadoc.as_ref().to_tuple());
					let diff_method_side_op = get(&diff_method.javadoc, side.opposite());

					method_change_a.is_none() != sibling_side.is_none() &&
						diff_method_side_op.is_none() != sibling_side_op.is_none() &&
						method_change_a == sibling_side_op
				},
			}
		})
		.collect();

	Ok(match siblings.len().cmp(&1) {
		Ordering::Less => None,
		Ordering::Equal => {
			let mut siblings = siblings;
			Some(siblings.remove(0))
		},
		Ordering::Greater => manually_select_item(diff_method, siblings, |(_, x, _)| x),
	})
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
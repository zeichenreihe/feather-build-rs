use anyhow::Result;

#[deprecated]
pub(crate) mod v2;
#[deprecated]
pub(crate) mod v2_diff;
#[deprecated]
pub(crate) mod tree;
#[deprecated]
pub(crate) mod diff;

pub(crate) trait SetJavadoc<J> {
	fn set_javadoc(&mut self, javadoc: J);
}
pub(crate) trait GetJavadoc<J> {
	fn get_javadoc(&self) -> Option<&J>;
}

pub(crate) trait AddMember<M> {
	fn add_member(&mut self, member: M);
}

pub(crate) trait Member<K, V> {
	fn get(&self, key: &K) -> Option<&V>;
}

/// A trait that allows items to remove "dummy" mappings or diffs
pub(crate) trait RemoveDummy {
	/// Return `true` if the attempt of removing dummies was successful for all.
	fn remove_dummy(&mut self) -> bool;
}

mod remove_dummy_impl {
	use std::hash::Hash;
	use indexmap::IndexMap;
	use crate::tiny::RemoveDummy;

	impl<T> RemoveDummy for Option<T>
		where
			T: RemoveDummy,
	{
		fn remove_dummy(&mut self) -> bool {
			if let Some(x) = self {
				let success = x.remove_dummy();
				if success {
					*self = None;
				}
				success
			} else {
				true
			}
		}
	}

	impl<K, V> RemoveDummy for IndexMap<K, V>
		where
			K: Hash + Eq,
			V: RemoveDummy,
	{
		fn remove_dummy(&mut self) -> bool {
			self.retain(|_, v| !v.remove_dummy());

			self.is_empty()
		}
	}
}

pub(crate) trait GetKey<K: Sized> {
	fn get_key(&self) -> K;
}

pub(crate) trait ApplyDiff<D> {
	fn apply_diff(&mut self, diff: &D) -> Result<()>;
}

mod apply_diff_impl {
	use std::hash::Hash;
	use anyhow::Result;
	use indexmap::IndexMap;
	use indexmap::map::Entry;
	use crate::tiny::{ApplyDiff, Diff, GetKey, Op};

	impl<D, M> ApplyDiff<Option<D>> for Option<M>
	where
		M: ApplyDiff<D>,
	{
		fn apply_diff(&mut self, diff: &Option<D>) -> Result<()> {
			match diff {
				None => Ok(()),
				Some(diff) => {
					todo!()
				},
			}
		}
	}

	impl<D, K, M> ApplyDiff<Vec<D>> for IndexMap<K, M>
	where
		D: GetKey<K> + Diff,
		K: Eq + Hash,
		M: ApplyDiff<D>,
	{
		fn apply_diff(&mut self, diffs: &Vec<D>) -> Result<()> {
			for diff in diffs {
				let entry = self.entry(diff.get_key());

				match diff.get_op() {
					Op::None => {
						if let Entry::Occupied(mut entry) = entry {
							todo!()
						}
					},
					Op::Change => {
						if let Entry::Occupied(mut entry) = entry {
							todo!()
						}
					},
					Op::Add => {
						if let Entry::Occupied(mut entry) = entry {
							todo!()
						}
					},
					Op::Remove => {
						if let Entry::Occupied(mut entry) = entry {
							todo!()
						}
					},
				}
			}

			Ok(())
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) enum Op {
	None,
	Change,
	Add,
	Remove,
}

impl Op {
	pub(crate) fn new(a: &Option<String>, b: &Option<String>) -> Op {
		match (a, b) {
			(None, Some(_)) => Op::Add,
			(Some(_), None) => Op::Remove,
			(Some(a), Some(b)) if a != b => Op::Change,
			_ => Op::None,
		}
	}
}

pub(crate) trait Diff {
	fn get_op(&self) -> Op;
}
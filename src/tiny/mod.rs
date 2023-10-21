use std::hash::Hash;
use indexmap::IndexMap;

pub mod v2;
pub mod v2_diff;

pub trait SetDoc<J> {
	fn set_doc(&mut self, doc: J);
}

pub trait AddMember<M> {
	fn add_member(&mut self, member: M);
}

pub trait RemoveDummy {
	/// Try to empty the current element, and if it's empty, return true.
	///
	/// Note that `a.remove_dummy() & b.remove_dummy()` is used here (and not `&&`), since it cleans up `b` even
	/// tho `a` failed to clean up.
	fn remove_dummy(&mut self) -> bool;
}
impl<T> RemoveDummy for Option<T>
where
	T: RemoveDummy,
{
	fn remove_dummy(&mut self) -> bool {
		if let Some(x) = self {
			if x.remove_dummy() {
				*self = None;
				true
			} else {
				false
			}
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
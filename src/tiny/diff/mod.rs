#![allow(deprecated)]

mod diffs;
mod class_diff;
mod field_diff;
mod method_diff;
mod parameter_diff;
mod javadoc_diff;

pub(crate) use diffs::*;
pub(crate) use class_diff::*;
pub(crate) use field_diff::*;
pub(crate) use method_diff::*;
pub(crate) use parameter_diff::*;
pub(crate) use javadoc_diff::*;




pub(crate) mod old_diffs_impl {
	use std::fmt::Debug;
	use std::hash::Hash;
	use anyhow::{anyhow, bail, Context, Result};
	use indexmap::IndexMap;
	use indexmap::map::Entry;
	use crate::tiny::diff::{ClassDiff, Diffs, FieldDiff, JavadocDiff, MethodDiff, ParameterDiff};
	use crate::tiny::GetKey;
	use crate::tiny::tree::{ClassMapping, FieldMapping, JavadocMapping, Mappings, MethodMapping, ParameterMapping};

	pub(crate) trait ApplyDiffOld<T> {
		fn apply_to_old(&self, target: T) -> Result<()>;
	}

	trait OperationExecution<T>
		where
			T: Sized,
	{
		fn get_operation(&self) -> Operation<String>;
		fn apply_inner(&self, inner: &mut T) -> Result<()>;
		fn apply_change(inner: &mut T, dst_a: &String, dst_b: &String) -> Result<()>;
		fn apply_add(&self, dst: String) -> Result<T>;
		fn apply_remove(inner: T, dst_a: &String) -> Result<()>;
	}

	impl<'a, D, M> ApplyDiffOld<&'a mut Option<M>> for Option<D>
		where
			D: OperationExecution<M>,
			M: Debug,
	{
		fn apply_to_old(&self, inner: &'a mut Option<M>) -> Result<()> {
			match self {
				Some(x) => {
					match x.get_operation() {
						Operation::None => {
							if let Some(y) = inner {
								x.apply_inner(y)?;
							}
						},
						Operation::Change(dst_a, dst_b) => {
							if let Some(y) = inner {
								D::apply_change(y, dst_a, dst_b)?;
								x.apply_inner(y)?;
							} else {
								bail!("Cannot change item {dst_a:?} to {dst_b:?}: no item given")
							}
						},
						Operation::Add(dst_b) => {
							if let Some(y) = inner {
								bail!("Cannot add item {dst_b:?}: already existing: {y:?}")
							} else {
								let mut v = x.apply_add(dst_b.clone())?;

								x.apply_inner(&mut v)?;

								*inner = Some(v);
							}
						},
						Operation::Remove(dst_a) => {
							if let Some(y) = inner.take() {
								D::apply_remove(y, dst_a)?;
							} else {
								bail!("Cannot remove item {dst_a:?}: no item given")
							}
						},
					}
				},
				None => *inner = None,
			}
			Ok(())
		}
	}

	impl<D, K, M> ApplyDiffOld<&mut IndexMap<K, M>> for Vec<D>
		where
			D: OperationExecution<M> + GetKey<K>,
			K: Debug + Hash + Eq,
			M: Debug,
	{
		fn apply_to_old(&self, map: &mut IndexMap<K, M>) -> Result<()> {
			for diff in self {
				let entry = map.entry(diff.get_key());

				match diff.get_operation() {
					Operation::None => {
						if let Entry::Occupied(mut entry) = entry {
							diff.apply_inner(entry.get_mut())?;
						}
					},
					Operation::Change(dst_a, dst_b) => {
						if let Entry::Occupied(mut entry) = entry {
							D::apply_change(entry.get_mut(), dst_a, dst_b)?;
							diff.apply_inner(entry.get_mut())?;
						} else {
							bail!("Cannot change item {dst_a:?} to {dst_b:?}: no item given")
						}
					},
					Operation::Add(dst_b) => {
						match entry {
							Entry::Occupied(entry) => bail!("Cannot add item {dst_b}: already existing: {entry:?}"),
							Entry::Vacant(entry) => {
								let mut v = diff.apply_add(dst_b.clone())?;

								diff.apply_inner(&mut v)?;

								entry.insert(v);
							},
						}
					},
					Operation::Remove(dst_a) => {
						if let Entry::Occupied(entry) = entry {
							D::apply_remove(entry.remove(), dst_a)?;
						} else {
							bail!("Cannot remove item {dst_a:?}: no item given")
						}
					},
				}
			}

			Ok(())
		}
	}

	#[derive(Debug, Clone)]
	pub(crate) enum Operation<'a, T> {
		None,
		Change(&'a T, &'a T),
		Add(&'a T),
		Remove(&'a T),
	}

	impl<T> Operation<'_, T> {
		#[deprecated]
		pub(crate) fn of<'a>(a: &'a Option<T>, b: &'a Option<T>) -> Operation<'a, T>
			where
				T: PartialEq
		{
			match (a, b) {
				(None, Some(b)) => Operation::Add(b),
				(Some(a), None) => Operation::Remove(a),
				(Some(a), Some(b)) if a != b => Operation::Change(a, b),
				_ => Operation::None,
			}
		}
	}



	impl OperationExecution<MethodMapping> for MethodDiff {
		fn get_operation(&self) -> Operation<String> {
			Operation::of(&self.dst_a, &self.dst_b)
		}

		fn apply_inner(&self, inner: &mut MethodMapping) -> Result<()> {
			self.jav.apply_to_old(&mut inner.jav)
				.with_context(|| anyhow!("Failed to apply diff on javadoc of method {:?} {:?}", inner.src, inner.dst))?;
			self.parameters.apply_to_old(&mut inner.parameters)
				.with_context(|| anyhow!("Failed to apply diff on parameters of method {:?} {:?}", inner.src, inner.dst))
		}

		fn apply_change(inner: &mut MethodMapping, dst_a: &String, dst_b: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot change: got {:?} but expected {dst_a:?}, to map to {dst_b:?}", inner.dst)
			}
			inner.dst = dst_b.to_owned();
			Ok(())
		}

		fn apply_add(&self, dst: String) -> Result<MethodMapping> {
			Ok(MethodMapping::new(self.desc.clone(), self.src.clone(), dst))
		}

		fn apply_remove(inner: MethodMapping, dst_a: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot remove: got {:?} but expected {dst_a:?}", inner.dst);
			}
			Ok(())
		}
	}
	impl OperationExecution<ParameterMapping> for ParameterDiff {
		fn get_operation(&self) -> Operation<String> {
			Operation::of(&self.dst_a, &self.dst_b)
		}

		fn apply_inner(&self, inner: &mut ParameterMapping) -> Result<()> {
			self.jav.apply_to_old(&mut inner.jav)
				.with_context(|| anyhow!("Failed to apply diff on javadoc of parameter {:?} {:?} {:?}", inner.index, inner.src, inner.dst))
		}

		fn apply_change(inner: &mut ParameterMapping, dst_a: &String, dst_b: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot change: got {:?} but expected {dst_a:?}, to map to {dst_b:?}", inner.dst)
			}
			inner.dst = dst_b.to_owned();
			Ok(())
		}

		fn apply_add(&self, dst: String) -> Result<ParameterMapping> {
			Ok(ParameterMapping::new(self.index, self.src.clone(), dst))
		}

		fn apply_remove(inner: ParameterMapping, dst_a: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot remove: got {:?} but expected {dst_a:?}", inner.dst);
			}
			Ok(())
		}
	}

	impl ApplyDiffOld<&mut Mappings> for Diffs {
		fn apply_to_old(&self, target: &mut Mappings) -> Result<()> {
			self.classes.apply_to_old(&mut target.classes)
				.with_context(|| anyhow!("Failed to apply diff on mapping {:?} {:?}", target.src, target.dst))?;
			Ok(())
		}
	}
	impl OperationExecution<ClassMapping> for ClassDiff {
		fn get_operation(&self) -> Operation<String> {
			Operation::of(&self.dst_a, &self.dst_b)
		}

		fn apply_inner(&self, inner: &mut ClassMapping) -> Result<()> {
			self.jav.apply_to_old(&mut inner.jav)
				.with_context(|| anyhow!("Failed to apply diff on javadoc of class {:?} {:?}", inner.src, inner.dst))?;
			self.fields.apply_to_old(&mut inner.fields)
				.with_context(|| anyhow!("Failed to apply diff on field of class {:?} {:?}", inner.src, inner.dst))?;
			self.methods.apply_to_old(&mut inner.methods)
				.with_context(|| anyhow!("Failed to apply diff on method of class {:?} {:?}", inner.src, inner.dst))
		}

		fn apply_change(inner: &mut ClassMapping, dst_a: &String, dst_b: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot change: got {:?} but expected {dst_a:?}, to map to {dst_b:?}", inner.dst)
			}
			inner.dst = dst_b.to_owned();
			Ok(())
		}

		fn apply_add(&self, dst: String) -> Result<ClassMapping> {
			Ok(ClassMapping::new(self.src.clone(), dst))
		}

		fn apply_remove(inner: ClassMapping, dst_a: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot remove: got {:?} but expected {dst_a:?}", inner.dst);
			}
			Ok(())
		}
	}
	impl OperationExecution<FieldMapping> for FieldDiff {
		fn get_operation(&self) -> Operation<String> {
			Operation::of(&self.dst_a, &self.dst_b)
		}

		fn apply_inner(&self, inner: &mut FieldMapping) -> Result<()> {
			self.jav.apply_to_old(&mut inner.jav)
				.with_context(|| anyhow!("Failed to apply diff on javadoc of field {:?} {:?}", inner.src, inner.dst))
		}

		fn apply_change(inner: &mut FieldMapping, dst_a: &String, dst_b: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot change: got {:?} but expected {dst_a:?}, to map to {dst_b:?}", inner.dst)
			}
			inner.dst = dst_b.to_owned();
			Ok(())
		}

		fn apply_add(&self, dst: String) -> Result<FieldMapping> {
			Ok(FieldMapping::new(self.desc.clone(), self.src.clone(), dst))
		}

		fn apply_remove(inner: FieldMapping, dst_a: &String) -> Result<()> {
			if &inner.dst != dst_a {
				bail!("Cannot remove: got {:?} but expected {dst_a:?}", inner.dst);
			}
			Ok(())
		}
	}

	impl OperationExecution<JavadocMapping> for JavadocDiff {
		fn get_operation(&self) -> Operation<String> {
			Operation::of(&self.jav_a, &self.jav_b)
		}

		fn apply_inner(&self, inner: &mut JavadocMapping) -> Result<()> {
			Ok(())
		}

		fn apply_change(inner: &mut JavadocMapping, dst_a: &String, dst_b: &String) -> Result<()> {
			if &inner.jav != dst_a {
				bail!("Cannot change: got {:?} but expected {dst_a:?}, to map to {dst_b:?}", inner.jav)
			}
			inner.jav = dst_b.to_owned();
			Ok(())
		}

		fn apply_add(&self, dst: String) -> Result<JavadocMapping> {
			Ok(JavadocMapping { jav: dst })
		}

		fn apply_remove(inner: JavadocMapping, dst_a: &String) -> Result<()> {
			if &inner.jav != dst_a {
				bail!("Cannot remove: got {:?} but expected {dst_a:?}", inner.jav);
			}
			Ok(())
		}
	}
}



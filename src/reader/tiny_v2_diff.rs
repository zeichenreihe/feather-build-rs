use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use crate::reader::{AddMember, parse, ParseEntry, SetDoc, try_read, try_read_optional};
use crate::reader::tiny_v2::{ClassMapping, FieldMapping, JavadocMapping, Mappings, MethodMapping, ParameterMapping};

pub fn read(path: impl AsRef<Path> + Debug) -> Result<Diffs> {
	parse::<Diffs, ClassDiff, FieldDiff, MethodDiff, ParameterDiff, JavadocDiff>(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read file {path:?}"))
}

pub trait ApplyDiff<T> {
	fn apply_to(&self, target: T) -> Result<()>;
}
trait GetKey<V> {
	fn get_key(&self) -> V;
}

impl<'a, T, U> ApplyDiff<&'a mut Option<U>> for Option<T>
	where
		T: ApplyDiff<&'a mut Option<U>>,
{
	fn apply_to(&self, target: &'a mut Option<U>) -> Result<()> {
		match self {
			Some(x) => x.apply_to(target)?,
			None => *target = None,
		}
		Ok(())
	}
}

impl<'a, T, U, V> ApplyDiff<Entry<'a, U, V>> for Option<T>
where
	T: ApplyDiff<Entry<'a, U, V>>,
{
	fn apply_to(&self, entry: Entry<'a, U, V>) -> Result<()> {
		match self {
			Some(x) => x.apply_to(entry)?,
			None => {
				if let Entry::Occupied(entry) = entry {
					entry.remove();
				}
			},
		}
		Ok(())
	}
}

impl<T, U, V> ApplyDiff<&mut HashMap<V, U>> for Vec<T>
where
	T: for<'a> ApplyDiff<Entry<'a, V, U>> + GetKey<V>,
	U: Clone,
	V: Eq + Hash,
{
	fn apply_to(&self, target: &mut HashMap<V, U>) -> Result<()> {
		for diff in self {
			diff.apply_to(target.entry(diff.get_key()))?;
		}

		Ok(())
	}
}

#[derive(Debug, Clone)]
pub enum Operation<'a> {
	None,
	Change(&'a String, &'a String),
	Add(&'a String),
	Remove(&'a String),
}

impl Operation<'_> {
	pub fn of<'a>(a: &'a Option<String>, b: &'a Option<String>) -> Operation<'a> {
		match (a, b) {
			(None, Some(b)) => Operation::Add(b),
			(Some(a), None) => Operation::Remove(a),
			(Some(a), Some(b)) if a != b => Operation::Change(a, b),
			_ => Operation::None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct Diffs {
	pub classes: Vec<ClassDiff>,
}

impl ParseEntry for Diffs {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		if iter.next().is_some() {
			bail!("expected empty namespaces for a diff");
		}

		Ok(Diffs {
			classes: Vec::new(),
		})
	}
}
impl AddMember<ClassDiff> for Diffs {
	fn add_member(&mut self, member: ClassDiff) {
		self.classes.push(member)
	}
}
impl ApplyDiff<&mut Mappings> for Diffs {
	fn apply_to(&self, target: &mut Mappings) -> Result<()> {
		self.classes.apply_to(&mut target.classes)
			.with_context(|| anyhow!("Failed on applying diff to mapping `{}` `{}`", target.src, target.dst))?;
		Ok(())
	}
}

#[derive(Debug, Clone)]
pub struct ClassDiff {
	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: Option<JavadocDiff>,

	pub fields: Vec<FieldDiff>,
	pub methods: Vec<MethodDiff>,
}

impl ParseEntry for ClassDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(Self {
			src: try_read(iter)?,
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			jav: None,
			fields: Vec::new(),
			methods: Vec::new(),
		})
	}
}
impl SetDoc<JavadocDiff> for ClassDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}
impl AddMember<FieldDiff> for ClassDiff {
	fn add_member(&mut self, member: FieldDiff) {
		self.fields.push(member)
	}
}
impl AddMember<MethodDiff> for ClassDiff {
	fn add_member(&mut self, member: MethodDiff) {
		self.methods.push(member)
	}
}
impl<T: Debug> ApplyDiff<Entry<'_, T, ClassMapping>> for ClassDiff {
	fn apply_to(&self, entry: Entry<'_, T, ClassMapping>) -> Result<()> {
		match entry {
			Entry::Occupied(mut entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => {
						self.jav.apply_to(&mut entry.get_mut().jav)?;
						self.fields.apply_to(&mut entry.get_mut().fields)?;
						self.methods.apply_to(&mut entry.get_mut().methods)?;
					},
					Operation::Change(dst_a, dst_b) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot change: got {:?} but expected {dst_a}, to map to {dst_b}", entry.get())
						}
						entry.get_mut().dst = dst_b.to_owned();

						self.jav.apply_to(&mut entry.get_mut().jav)?;
						self.fields.apply_to(&mut entry.get_mut().fields)?;
						self.methods.apply_to(&mut entry.get_mut().methods)?;
					},
					Operation::Add(dst_b) => {
						bail!("Cannot add item {dst_b}: already existing: {entry:?}")
					},
					Operation::Remove(dst_a) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot remove: got {:?} but expected {dst_a}", entry.get());
						}
						entry.remove();
					},
				}
			},
			Entry::Vacant(entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => {},
					Operation::Change(dst_a, dst_b) => {
						bail!("Cannot change item {dst_a} to {dst_b}: no item given")
					},
					Operation::Add(dst_b) => {
						let mut v = ClassMapping::new(self.src.clone(), dst_b.clone());

						self.jav.apply_to(&mut v.jav)?;
						self.fields.apply_to(&mut v.fields)?;
						self.methods.apply_to(&mut v.methods)?;

						entry.insert(v);
					},
					Operation::Remove(dst_a) => {
						bail!("Cannot remove item {dst_a}: no item given")
					},
				}
			},
		}

		Ok(())
	}
}
impl GetKey<String> for ClassDiff {
	fn get_key(&self) -> String {
		self.src.clone()
	}
}

#[derive(Debug, Clone)]
pub struct FieldDiff {
	pub desc: String,

	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: Option<JavadocDiff>,
}

impl ParseEntry for FieldDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(FieldDiff {
			desc: try_read(iter)?,
			src: try_read(iter)?,
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			jav: None,
		})
	}
}
impl SetDoc<JavadocDiff> for FieldDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}
impl<T: Debug> ApplyDiff<Entry<'_, T, FieldMapping>> for FieldDiff {
	fn apply_to(&self, entry: Entry<'_, T, FieldMapping>) -> Result<()> {
		match entry {
			Entry::Occupied(mut entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => {
						self.jav.apply_to(&mut entry.get_mut().jav)?;
					},
					Operation::Change(dst_a, dst_b) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot change: got {:?} but expected {dst_a}, to map to {dst_b}", entry.get())
						}
						entry.get_mut().dst = dst_b.to_owned();
						self.jav.apply_to(&mut entry.get_mut().jav)?;
					},
					Operation::Add(dst_b) => {
						bail!("Cannot add item {dst_b}: already existing: {entry:?}")
					},
					Operation::Remove(dst_a) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot remove: got {:?} but expected {dst_a}", entry.get());
						}
						entry.remove();
					},
				}
			},
			Entry::Vacant(entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => {},
					Operation::Change(dst_a, dst_b) => {
						bail!("Cannot change item {dst_a} to {dst_b}: no item given")
					},
					Operation::Add(dst_b) => {
						let mut f = FieldMapping::new(self.desc.clone(), self.src.clone(), dst_b.clone());
						self.jav.apply_to(&mut f.jav)?;
						entry.insert(f);
					},
					Operation::Remove(dst_a) => {
						bail!("Cannot remove item {dst_a}: no item given")
					},
				}
			},
		}

		Ok(())
	}
}
impl GetKey<(String, String)> for FieldDiff {
	fn get_key(&self) -> (String, String) {
		(self.desc.clone(), self.src.clone())
	}
}

#[derive(Debug, Clone)]
pub struct MethodDiff {
	pub desc: String,

	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: Option<JavadocDiff>,

	pub parameters: Vec<ParameterDiff>,
}

impl ParseEntry for MethodDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(MethodDiff {
			desc: try_read(iter)?,
			src: try_read(iter)?,
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			jav: None,
			parameters: Vec::new(),
		})
	}
}
impl SetDoc<JavadocDiff> for MethodDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}
impl AddMember<ParameterDiff> for MethodDiff {
	fn add_member(&mut self, member: ParameterDiff) {
		self.parameters.push(member)
	}
}
impl<T: Debug> ApplyDiff<Entry<'_, T, MethodMapping>> for MethodDiff {
	fn apply_to(&self, entry: Entry<'_, T, MethodMapping>) -> Result<()> {
		match entry {
			Entry::Occupied(mut entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => {
						self.jav.apply_to(&mut entry.get_mut().jav)?;
						self.parameters.apply_to(&mut entry.get_mut().parameters)?;
					},
					Operation::Change(dst_a, dst_b) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot change: got {:?} but expected {dst_a}, to map to {dst_b}", entry.get())
						}
						entry.get_mut().dst = dst_b.to_owned();
						self.jav.apply_to(&mut entry.get_mut().jav)?;
						self.parameters.apply_to(&mut entry.get_mut().parameters)?;
					},
					Operation::Add(dst_b) => {
						bail!("Cannot add item {dst_b}: already existing: {entry:?}")
					},
					Operation::Remove(dst_a) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot remove: got {:?} but expected {dst_a}", entry.get());
						}
						entry.remove();
					},
				}
			},
			Entry::Vacant(entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => {},
					Operation::Change(dst_a, dst_b) => {
						bail!("Cannot change item {dst_a} to {dst_b}: no item given")
					},
					Operation::Add(dst_b) => {
						let mut v = MethodMapping::new(self.desc.clone(), self.src.clone(), dst_b.clone());

						self.jav.apply_to(&mut v.jav)?;
						self.parameters.apply_to(&mut v.parameters)?;

						entry.insert(v);
					},
					Operation::Remove(dst_a) => {
						bail!("Cannot remove item {dst_a}: no item given")
					},
				}
			},
		}

		Ok(())
	}
}
impl GetKey<(String, String)> for MethodDiff {
	fn get_key(&self) -> (String, String) {
		(self.desc.clone(), self.src.clone())
	}
}

#[derive(Debug, Clone)]
pub struct ParameterDiff {
	pub src: String,
	pub dst_a: Option<String>,
	pub dst_b: Option<String>,

	pub jav: Option<JavadocDiff>,

	pub index: usize,
}

impl ParseEntry for ParameterDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		let index = try_read(iter)?.parse()
			.with_context(|| anyhow!("illegal parameter index"))?;
		Ok(ParameterDiff {
			index,
			src: try_read_optional(iter)?.unwrap_or(String::new()), // TODO: ask space what this means, change to `try_read(&mut iter)` to see it fail
			dst_a: try_read_optional(iter)?,
			dst_b: try_read_optional(iter)?,
			jav: None,
		})
	}
}
impl SetDoc<JavadocDiff> for ParameterDiff {
	fn set_doc(&mut self, doc: JavadocDiff) {
		self.jav = Some(doc);
	}
}
impl<T: Debug> ApplyDiff<Entry<'_, T, ParameterMapping>> for ParameterDiff {
	fn apply_to(&self, entry: Entry<'_, T, ParameterMapping>) -> Result<()> {
		match entry {
			Entry::Occupied(mut entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => self.jav.apply_to(&mut entry.get_mut().jav)?,
					Operation::Change(dst_a, dst_b) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot change: got {:?} but expected {dst_a}, to map to {dst_b}", entry.get())
						}
						entry.get_mut().dst = dst_b.to_owned();
						self.jav.apply_to(&mut entry.get_mut().jav)?;
					},
					Operation::Add(dst_b) => {
						bail!("Cannot add item {dst_b}: already existing: {entry:?}")
					},
					Operation::Remove(dst_a) => {
						if &entry.get().dst != dst_a {
							bail!("Cannot remove: got {:?} but expected {dst_a}", entry.get());
						}
						entry.remove();
					},
				}
			},
			Entry::Vacant(entry) => {
				match Operation::of(&self.dst_a, &self.dst_b) {
					Operation::None => {},
					Operation::Change(dst_a, dst_b) => {
						bail!("Cannot change item {dst_a} to {dst_b}: no item given")
					},
					Operation::Add(dst_b) => {
						let mut v = ParameterMapping::new(self.index, self.src.clone(), dst_b.clone());
						self.jav.apply_to(&mut v.jav)?;
						entry.insert(v);
					},
					Operation::Remove(dst_a) => {
						bail!("Cannot remove item {dst_a}: no item given")
					},
				}
			},
		}

		Ok(())
	}
}
impl GetKey<usize> for ParameterDiff {
	fn get_key(&self) -> usize {
		self.index
	}
}

#[derive(Debug, Clone)]
pub struct JavadocDiff {
	pub jav_a: Option<String>,
	pub jav_b: Option<String>,
}

impl ParseEntry for JavadocDiff {
	fn from_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self> {
		Ok(JavadocDiff {
			jav_a: try_read_optional(iter)?,
			jav_b: try_read_optional(iter)?,
		})
	}
}
impl ApplyDiff<&mut Option<JavadocMapping>> for JavadocDiff {
	fn apply_to(&self, target: &mut Option<JavadocMapping>) -> Result<()> {
		match Operation::of(&self.jav_a, &self.jav_b) {
			Operation::None => {},
			Operation::Change(dst_a, dst_b) => {
				if let Some(target) = target {
					if &target.jav != dst_a {
						bail!("Cannot change: got {:?} but expected {dst_a}, to map to {dst_b}", target)
					}
					target.jav = dst_b.to_owned()
				} else {
					bail!("Cannot change item: no item given")
				}
			},
			Operation::Add(dst_b) => {
				if let Some(target) = target {
					bail!("Cannot add item {dst_b}: already existing: {target:?}")
				} else {
					*target = Some(JavadocMapping {
						jav: dst_b.to_owned(),
					})
				}
			},
			Operation::Remove(dst_a) => {
				if target.is_some() {
					// TODO:
					//if &target.get().dst != dst_a {
					//	bail!("Cannot remove: got {:?} but expected {dst_a}", entry.get());
					//}
					*target = None;
				} else {
					bail!("Cannot remove item {dst_a}: no item given")
				}
			},
		}

		Ok(())
	}
}
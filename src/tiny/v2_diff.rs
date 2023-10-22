use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use crate::reader::{parse, ParseEntry, try_read, try_read_optional};
use crate::tiny::{AddMember, SetDoc};
use crate::tiny::v2::{ClassMapping, FieldMapping, JavadocMapping, Mappings, MethodMapping, ParameterMapping};

pub fn read(path: impl AsRef<Path> + Debug) -> Result<Diffs> {
	parse::<Diffs, ClassDiff, FieldDiff, MethodDiff, ParameterDiff, JavadocDiff>(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read diff file {path:?}"))
}

pub trait ApplyDiff<T> {
	fn apply_to(&self, target: T) -> Result<()>;
}

trait OperationExecution<T>
where
	T: Sized,
{
	fn get_operation(&self) -> Operation;
	fn apply_inner(&self, inner: &mut T) -> Result<()>;
	fn apply_change(inner: &mut T, dst_a: &String, dst_b: &String) -> Result<()>;
	fn apply_add(&self, dst: String) -> Result<T>;
	fn apply_remove(inner: T, dst_a: &String) -> Result<()>;
}

trait GetKey<K, V>
where
	K: Sized,
{
	fn get_key(&self) -> K;
}

impl<'a, T, U> ApplyDiff<&'a mut Option<U>> for Option<T>
where
	T: OperationExecution<U>,
	U: Debug,
{
	fn apply_to(&self, inner: &'a mut Option<U>) -> Result<()> {
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
							T::apply_change(y, dst_a, dst_b)?;
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
							T::apply_remove(y, dst_a)?;
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

impl<T, K, V> ApplyDiff<&mut IndexMap<K, V>> for Vec<T>
where
	T: OperationExecution<V> + GetKey<K, V>,
	K: Debug + Hash + Eq,
	V: Debug,
{
	fn apply_to(&self, map: &mut IndexMap<K, V>) -> Result<()> {
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
						T::apply_change(entry.get_mut(), dst_a, dst_b)?;
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
						T::apply_remove(entry.remove(), dst_a)?;
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
			.with_context(|| anyhow!("Failed to apply diff on mapping {:?} {:?}", target.src, target.dst))?;
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
impl OperationExecution<ClassMapping> for ClassDiff {
	fn get_operation(&self) -> Operation {
		Operation::of(&self.dst_a, &self.dst_b)
	}

	fn apply_inner(&self, inner: &mut ClassMapping) -> Result<()> {
		self.jav.apply_to(&mut inner.jav)
			.with_context(|| anyhow!("Failed to apply diff on javadoc of class {:?} {:?}", inner.src, inner.dst))?;
		self.fields.apply_to(&mut inner.fields)
			.with_context(|| anyhow!("Failed to apply diff on field of class {:?} {:?}", inner.src, inner.dst))?;
		self.methods.apply_to(&mut inner.methods)
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
impl GetKey<String, ClassMapping> for ClassDiff {
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
impl OperationExecution<FieldMapping> for FieldDiff {
	fn get_operation(&self) -> Operation {
		Operation::of(&self.dst_a, &self.dst_b)
	}

	fn apply_inner(&self, inner: &mut FieldMapping) -> Result<()> {
		self.jav.apply_to(&mut inner.jav)
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
impl GetKey<(String, String), FieldMapping> for FieldDiff {
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
impl OperationExecution<MethodMapping> for MethodDiff {
	fn get_operation(&self) -> Operation {
		Operation::of(&self.dst_a, &self.dst_b)
	}

	fn apply_inner(&self, inner: &mut MethodMapping) -> Result<()> {
		self.jav.apply_to(&mut inner.jav)
			.with_context(|| anyhow!("Failed to apply diff on javadoc of method {:?} {:?}", inner.src, inner.dst))?;
		self.parameters.apply_to(&mut inner.parameters)
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
impl GetKey<(String, String), MethodMapping> for MethodDiff {
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
impl OperationExecution<ParameterMapping> for ParameterDiff {
	fn get_operation(&self) -> Operation {
		Operation::of(&self.dst_a, &self.dst_b)
	}

	fn apply_inner(&self, inner: &mut ParameterMapping) -> Result<()> {
		self.jav.apply_to(&mut inner.jav)
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
impl GetKey<usize, ParameterMapping> for ParameterDiff {
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
impl OperationExecution<JavadocMapping> for JavadocDiff {
	fn get_operation(&self) -> Operation {
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
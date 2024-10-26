//! Remappers for remapping class names, descriptors, fields and methods.
//!
//! For remapping just classes and descriptors, you're interested in [`ARemapper`].
//! If you also want to remap field names and method names, use the [`BRemapper`].
//!
//! Note that implementors of these traits can be created by the methods
//! [`Mappings::remapper_a`] and [`Mappings::remapper_b`] for remapping
//! between given namespaces.
//!
//! In case you want to implement a remapper yourself, you only need to define the trait methods that don't have
//! a default implementation.
//!
//! # What is a "remapper"?
//! A remapper answers the question for you "what is the name of X in namespace Y?"
//!

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use anyhow::{bail, Result};
use indexmap::{IndexMap, IndexSet};
use java_string::{JavaCodePoint, JavaStr, JavaString};
use duke::tree::class::{ArrClassName, ClassName, ClassNameSlice, ObjClassName, ObjClassNameSlice};
use duke::tree::descriptor::{ReturnDescriptor, ReturnDescriptorSlice};
use duke::tree::field::{FieldDescriptor, FieldDescriptorSlice, FieldNameAndDesc, FieldNameSlice, FieldRef};
use duke::tree::method::{MethodDescriptor, MethodDescriptorSlice, MethodNameAndDesc, MethodNameSlice, MethodRef, MethodRefObj};
use crate::tree::mappings::Mappings;
use crate::tree::names::Namespace;

/// A remapper supporting remapping of class names and descriptors.
pub trait ARemapper {
	/// Maps a class name to a new one, if the mapping exists.
	///
	/// If the mapping doesn't exist, returns `Ok(None)`.
	fn map_class_fail(&self, class: &ObjClassNameSlice) -> Result<Option<ObjClassName>>;

	/// Maps a class name to a new one, if the mapping doesn't exist, return the old one.
	///
	/// Do not implement this yourself.
	fn map_class(&self, class: &ObjClassNameSlice) -> Result<ObjClassName> {
		Ok(self.map_class_fail(class)?.unwrap_or_else(|| class.to_owned()))
	}

	/// Maps [any class name][ClassName] (not just [`ObjClassName`]) to a new one.
	///
	/// See [`ARemapper::map_class`] for more details.
	fn map_class_any(&self, class: &ClassNameSlice) -> Result<ClassName> {
		match class.as_arr_and_obj() {
			Ok(arr) => {
				// SAFETY: `arr` is an array class name. Array class names are field descriptors.
				unsafe { map_desc(self, arr.as_inner()) }
					// SAFETY: The returned descriptor only has names in L...; changed. Therefore it is a valid array class name.
					.map(|string| unsafe { ArrClassName::from_inner_unchecked(string) })
					.map(From::from)
			},
			Err(obj) => {
				self.map_class(obj)
					.map(From::from)
			},
		}
	}

	/// Maps a field descriptor to a new one.
	///
	/// Note that this relies on the fact that for non-existing class mappings class names are just copied over.
	///
	/// Do not implement this yourself.
	fn map_field_desc(&self, desc: &FieldDescriptorSlice) -> Result<FieldDescriptor> {
		// SAFETY: `desc` is a field descriptor.
		unsafe { map_desc(self, desc.as_inner()) }
			// SAFETY: The returned field descriptor string is always valid.
			.map(|string| unsafe { FieldDescriptor::from_inner_unchecked(string) })
	}

	/// Maps a method descriptor to a new one.
	///
	/// Note that this relies on the fact that for non-existing class mappings class names are just copied over.
	///
	/// Do not implement this yourself.
	fn map_method_desc(&self, desc: &MethodDescriptorSlice) -> Result<MethodDescriptor> {
		// SAFETY: `desc` is a method descriptor.
		unsafe { map_desc(self, desc.as_inner()) }
			// SAFETY: The returned method descriptor string is always valid.
			.map(|string| unsafe { MethodDescriptor::from_inner_unchecked(string) })
	}

	/// Maps a return descriptor to a new one.
	///
	/// Note that this relies on the fact that for non-existing class mappings class names are just copied over.
	///
	/// Do not implement this yourself.
	fn map_return_desc(&self, desc: &ReturnDescriptorSlice) -> Result<ReturnDescriptor> {
		// SAFETY: `desc` is a return descriptor.
		unsafe { map_desc(self, desc.as_inner()) }
			// SAFETY: The returned return descriptor string is always valid.
			.map(|string| unsafe { ReturnDescriptor::from_inner_unchecked(string) })
	}
}


/// Maps a descriptor to a new one.
///
/// # Safety
/// `desc` must be a valid field, method or return descriptor.
// TODO: what about returning Cow<'a, str> ('a on the &'a str)? would return `desc` if no remapping
//  was done, and a String if it was
unsafe fn map_desc(remapper: &(impl ARemapper + ?Sized), desc: &JavaStr) -> Result<JavaString> {
	let mut s = JavaString::new();

	let mut iter = desc.chars();

	while let Some(ch) = iter.next() {
		s.push_java(ch);

		if ch == 'L' {
			let mut class_name = JavaString::new();
			for ch in iter.by_ref() {
				class_name.push_java(ch);
				if ch == ';' {
					break;
				}
			}
			if class_name.pop() != Some(JavaCodePoint::from_char(';')) {
				bail!("descriptor {desc:?} has a missing semicolon somewhere");
			}

			// String to ClassName doesn't allocate new memory, so it's fine
			// SAFETY: `class_name` is a valid class name since it comes from a valid descriptor.
			let old_class_name = unsafe { ObjClassName::from_inner_unchecked(class_name) };
			let new_class_name = remapper.map_class(&old_class_name)?;

			s.push_java_str(new_class_name.as_inner());
			s.push(';');
		}
	}

	Ok(s)
}

#[derive(Debug)]
pub struct ARemapperImpl<'a, const N: usize> {
	classes: IndexMap<&'a ObjClassNameSlice, &'a ObjClassNameSlice>,
}

impl<'a, const N: usize> ARemapper for ARemapperImpl<'a, N> {
	fn map_class_fail(&self, class: &ObjClassNameSlice) -> Result<Option<ObjClassName>> {
		match self.classes.get(class) {
			None => Ok(None),
			Some(&class) => Ok(Some(class.to_owned())),
		}
	}
}

impl<const N: usize> Mappings<N> {
	pub fn remapper_a(&self, from: Namespace<N>, to: Namespace<N>) -> Result<ARemapperImpl<'_, N>> {
		let mut classes = IndexMap::new();
		for class in self.classes.values() {
			if let (Some(from), Some(to)) = (&class.info.names[from], &class.info.names[to]) {
				classes.insert(from.as_slice(), to.as_slice());
			}
		}
		Ok(ARemapperImpl { classes })
	}
}

impl Mappings<2> {
	// TODO: this should probably not exist...
	pub fn remapper_a_first_to_second(&self) -> Result<ARemapperImpl<'_, 2>> {
		self.remapper_a(Namespace::new(0)?, Namespace::new(1)?)
	}
}

/// A remapper supporting remapping fields and methods, as well as class names and descriptors.
///
/// If you only want to remap class names and descriptors, consider using [ARemapper] instead.
pub trait BRemapper: ARemapper {
	/// Maps a field name and field descriptor to new ones, if the mapping exists.
	///
	/// If the mapping doesn't exist, returns `Ok(None)`.
	///
	/// Note that in the `None` case you must map the field descriptor manually. See [`map_field`] for a method that
	/// just takes the old name if no mapping exist (but yet maps the field descriptor).
	fn map_field_fail(&self, owner_name: &ObjClassNameSlice, field_name: &FieldNameSlice, field_desc: &FieldDescriptorSlice) -> Result<Option<FieldNameAndDesc>>;

	/// Maps a field name and field descriptor to new ones, if the mapping doesn't exist returns the old name with a
	/// mapped descriptor.
	///
	/// Do not implement this yourself.
	fn map_field(&self, class: &ObjClassNameSlice, field_name: &FieldNameSlice, field_desc: &FieldDescriptorSlice) -> Result<FieldNameAndDesc> {
		self.map_field_fail(class, field_name, field_desc)?
			.map(Ok)
			.unwrap_or_else(|| Ok(FieldNameAndDesc {
				desc: self.map_field_desc(field_desc)?,
				name: field_name.to_owned(),
			}))
	}

	/// Maps a [`FieldRef`], taking care of the class name as well.
	///
	/// Do not implement this yourself.
	fn map_field_ref(&self, field_ref: &FieldRef) -> Result<FieldRef> {
		let field_key = self.map_field(&field_ref.class, &field_ref.name, &field_ref.desc)?;
		let class_name = self.map_class(&field_ref.class)?;

		Ok(field_key.with_class(class_name))
	}

	/// Maps a method name and method descriptor to new ones, if the mapping exists.
	///
	/// If the mapping doesn't exist, returns `Ok(None)`.
	///
	/// Note that in the `None` case you must map the method descriptor manually. See [`map_method`] for a method that
	/// just takes the old name if no mapping exist (but yet maps the method descriptor).
	fn map_method_fail(&self, owner_name: &ObjClassNameSlice, method_name: &MethodNameSlice, method_desc: &MethodDescriptorSlice)
		-> Result<Option<MethodNameAndDesc>>;

	/// Maps a method name and method descriptor to new ones, if the mapping doesn't exist returns the old name with a
	/// mapped descriptor.
	///
	/// Do not implement this yourself.
	fn map_method(&self, class: &ObjClassNameSlice, method_name: &MethodNameSlice, method_desc: &MethodDescriptorSlice) -> Result<MethodNameAndDesc> {
		self.map_method_fail(class, method_name, method_desc)?
			.map(Ok)
			.unwrap_or_else(|| Ok(MethodNameAndDesc {
				desc: self.map_method_desc(method_desc)?,
				name: method_name.to_owned(),
			}))
	}

	/// Maps a [`MethodNameAndDesc`].
	///
	/// This is essentially just a call to [`BRemapper::map_method`].
	///
	/// Do not implement this yourself.
	fn map_method_name_and_desc(&self, class: &ObjClassNameSlice, method_name_and_desc: &MethodNameAndDesc) -> Result<MethodNameAndDesc> {
		self.map_method(class, &method_name_and_desc.name, &method_name_and_desc.desc)
	}

	/// Maps a [`MethodRef`], taking care of the class name as well.
	///
	/// If the [`MethodRef`] references an array class, no remapping of the name or descriptor is performed.
	///
	/// Do not implement this yourself.
	fn map_method_ref(&self, method_ref: &MethodRef) -> Result<MethodRef> {
		let method_key = if let Some(obj_class) = method_ref.class.as_obj() {
			self.map_method(obj_class, &method_ref.name, &method_ref.desc)?
		} else {
			MethodNameAndDesc {
				name: method_ref.name.clone(),
				desc: method_ref.desc.clone(), // an array's class method can only contain descriptors with names from the JDK
			}
		};
		let class_name = self.map_class_any(&method_ref.class)?;

		Ok(method_key.with_class(class_name))
	}

	fn map_method_ref_obj(&self, method_ref: &MethodRefObj) -> Result<MethodRefObj> {
		let method_key = self.map_method(&method_ref.class, &method_ref.name, &method_ref.desc)?;
		let class_name = self.map_class(&method_ref.class)?;

		Ok(method_key.with_class_obj(class_name))
	}
}

#[derive(Debug, PartialEq, Eq)]
struct TupleKey<A, B>(A, B);
#[derive(Debug, PartialEq, Eq)]
struct TupleReq<A, B>(A, B);

impl<Name, Desc, DescDeref: ?Sized> Hash for TupleKey<&'_ Name, Desc>
where
	Name: Hash + ?Sized,
	Desc: std::ops::Deref<Target = DescDeref>,
	DescDeref: Hash,
{
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.hash(state);
		std::ops::Deref::deref(&self.1).hash(state);
	}
}

impl<Name, Desc> Hash for TupleReq<&'_ Name, &'_ Desc>
where
	Name: Hash + ?Sized,
	Desc: Hash + ?Sized,
{
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.hash(state);
		self.1.hash(state);
	}
}

impl<'a, Name, Desc, DescBorrowed> indexmap::Equivalent<TupleKey<&'a Name, Desc>> for TupleReq<&'a Name, &'a DescBorrowed>
where
	Name: ?Sized + PartialEq + Eq,
	Desc: Borrow<DescBorrowed>,
	DescBorrowed: ?Sized + PartialEq + Eq,
{
	fn equivalent(&self, key: &TupleKey<&'a Name, Desc>) -> bool {
		self.0 == key.0 && self.1 == Borrow::borrow(&key.1)
	}
}

#[derive(Debug)]
struct BRemapperClass<'a> {
	name: &'a ObjClassName,
	fields: IndexMap<TupleKey<&'a FieldNameSlice, FieldDescriptor>, TupleKey<&'a FieldNameSlice, FieldDescriptor>>,
	methods: IndexMap<TupleKey<&'a MethodNameSlice, MethodDescriptor>, TupleKey<&'a MethodNameSlice, MethodDescriptor>>,
}

#[derive(Debug)]
pub struct BRemapperImpl<'a, 'i, const N: usize, I> {
	classes: IndexMap<&'a ObjClassNameSlice, BRemapperClass<'a>>,
	inheritance: &'i I,
}

impl<const N: usize, I> ARemapper for BRemapperImpl<'_, '_, N, I> {
	fn map_class_fail(&self, class: &ObjClassNameSlice) -> Result<Option<ObjClassName>> {
		match self.classes.get(class) {
			None => Ok(None),
			Some(class) => Ok(Some(class.name.clone())),
		}
	}
}

impl<'i, const N: usize, I: SuperClassProvider> BRemapper for BRemapperImpl<'_, 'i, N, I> {
	fn map_field_fail(&self, owner_name: &ObjClassNameSlice, field_name: &FieldNameSlice, field_desc: &FieldDescriptorSlice) -> Result<Option<FieldNameAndDesc>> {
		if let Some(class) = self.classes.get(owner_name) {
			let key = TupleReq(field_name, field_desc);
			if let Some(&TupleKey(name, ref desc)) = class.fields.get(&key) {
				let desc = desc.clone();
				let src = name.to_owned();
				return Ok(Some(FieldNameAndDesc { desc, name: src }));
			}

			if let Some(super_classes) = self.inheritance.get_super_classes(owner_name)? {
				for super_class in super_classes {
					if let Some(remapped) = self.map_field_fail(super_class, field_name, field_desc)? {
						return Ok(Some(remapped));
					}
				}
			}
		}

		Ok(None)
	}

	fn map_method_fail(&self, owner_name: &ObjClassNameSlice, method_name: &MethodNameSlice, method_desc: &MethodDescriptorSlice)
			-> Result<Option<MethodNameAndDesc>> {
		if let Some(class) = self.classes.get(owner_name) {
			let key = TupleReq(method_name, method_desc);
			if let Some(&TupleKey(name, ref desc)) = class.methods.get(&key) {
				let desc = desc.clone();
				let src = name.to_owned();
				return Ok(Some(MethodNameAndDesc { desc, name: src }));
			}

			if let Some(super_classes) = self.inheritance.get_super_classes(owner_name)? {
				for super_class in super_classes {
					if let Some(remapped) = self.map_method_fail(super_class, method_name, method_desc)? {
						return Ok(Some(remapped));
					}
				}
			}
		}

		Ok(None)
	}
}


impl<const N: usize> Mappings<N> {
	pub fn remapper_b<'i, I>(&self, from: Namespace<N>, to: Namespace<N>, inheritance: &'i I) -> Result<BRemapperImpl<'_, 'i, N, I>> {
		let remapper_a_from = self.remapper_a(Namespace::new(0)?, from)?;
		let remapper_a_to = self.remapper_a(Namespace::new(0)?, to)?;

		let mut classes = IndexMap::new();
		for class in self.classes.values() {
			if let (Some(name_from), Some(name_to)) = (&class.info.names[from], &class.info.names[to]) {
				let mut fields = IndexMap::new();
				for field in class.fields.values() {
					if let (Some(name_from), Some(name_to)) = (&field.info.names[from], &field.info.names[to]) {
						let desc_from = remapper_a_from.map_field_desc(&field.info.desc)?;
						let desc_to = remapper_a_to.map_field_desc(&field.info.desc)?;

						fields.insert(TupleKey(name_from.as_slice(), desc_from), TupleKey(name_to.as_slice(), desc_to));
					}
				}

				let mut methods = IndexMap::new();
				for method in class.methods.values() {
					if let (Some(name_from), Some(name_to)) = (&method.info.names[from], &method.info.names[to]) {
						let desc_from = remapper_a_from.map_method_desc(&method.info.desc)?;
						let desc_to = remapper_a_to.map_method_desc(&method.info.desc)?;

						methods.insert(TupleKey(name_from.as_slice(), desc_from), TupleKey(name_to.as_slice(), desc_to));
					}
				}

				classes.insert(name_from.as_slice(), BRemapperClass { name: name_to, fields, methods });
			}
		}
		Ok(BRemapperImpl { classes, inheritance })
	}
}

impl Mappings<2> {
	// TODO: this should probably not exist...
	pub fn remapper_b_first_to_second<'i, I>(&self, inheritance: &'i I) -> Result<BRemapperImpl<'_, 'i, 2, I>> {
		self.remapper_b(Namespace::new(0)?, Namespace::new(1)?, inheritance)
	}
}


pub struct JarSuperProv {
	pub super_classes: IndexMap<ObjClassName, IndexSet<ObjClassName>>,
}

impl JarSuperProv {
	pub fn remap(re: &impl ARemapper, prov: &Vec<JarSuperProv>) -> Result<Vec<JarSuperProv>> {
		let mut r = Vec::new();
		for i in prov {
			let mut super_classes = IndexMap::new();
			for (a, b) in &i.super_classes {
				let mut set = IndexSet::new();
				for j in b {
					set.insert(re.map_class(j)?);
				}
				super_classes.insert(re.map_class(a)?, set);
			}
			r.push(JarSuperProv { super_classes });
		}
		Ok(r)
	}
}


// TODO: I guess make a method ARemapper::to_b_remapper() to make a BRemapper out of an Self...
pub struct ARemapperAsBRemapper<T>(pub T) where T: ARemapper;

impl<T> ARemapper for ARemapperAsBRemapper<T> where T: ARemapper {
	fn map_class_fail(&self, class: &ObjClassNameSlice) -> Result<Option<ObjClassName>> {
		self.0.map_class_fail(class)
	}
}

impl<T> BRemapper for ARemapperAsBRemapper<T> where T: ARemapper {
	fn map_field_fail(&self, owner_name: &ObjClassNameSlice, field_name: &FieldNameSlice, field_desc: &FieldDescriptorSlice) -> Result<Option<FieldNameAndDesc>> {
		Ok(None)
	}

	fn map_method_fail(&self, owner_name: &ObjClassNameSlice, method_name: &MethodNameSlice, method_desc: &MethodDescriptorSlice)
			-> Result<Option<MethodNameAndDesc>> {
		Ok(None)
	}
}


pub trait SuperClassProvider {
	fn get_super_classes(&self, class: &ObjClassNameSlice) -> Result<Option<&IndexSet<ObjClassName>>>;
}

impl SuperClassProvider for JarSuperProv {
	fn get_super_classes(&self, class: &ObjClassNameSlice) -> Result<Option<&IndexSet<ObjClassName>>> {
		Ok(self.super_classes.get(class))
	}
}

impl<S: SuperClassProvider> SuperClassProvider for Vec<S> {
	fn get_super_classes(&self, class: &ObjClassNameSlice) -> Result<Option<&IndexSet<ObjClassName>>> {
		for i in self {
			if let Some(x) = i.get_super_classes(class)? {
				return Ok(Some(x));
			}
		}
		Ok(None)
	}
}

pub struct NoSuperClassProvider;

impl NoSuperClassProvider {
	pub fn new() -> &'static NoSuperClassProvider {
		static INSTANCE: NoSuperClassProvider = NoSuperClassProvider;
		&INSTANCE
	}
}

impl SuperClassProvider for NoSuperClassProvider {
	fn get_super_classes(&self, class: &ObjClassNameSlice) -> Result<Option<&IndexSet<ObjClassName>>> {
		Ok(None)
	}
}

#[cfg(test)]
mod testing {
	// TODO: test internals

	// TODO: test all methods, with array classes as well!
}
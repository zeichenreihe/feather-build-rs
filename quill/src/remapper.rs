//! Remappers for remapping class names, descriptors, fields and methods.
//!
//! For remapping just classes and descriptors, you're interested in [`ARemapper`].
//! If you also want to remap field names and method names, use the [`BRemapper`].
//!
//! Note that implementors of these traits can be created by the methods
//! [`Mappings::remapper_a`] and [`Mappings::remapper_b`] for remapping
//! between given namespaces.
//!
//! # What is a "remapper"?
//! A remapper answers the question for you "what is the name of X in namespace Y?"
//!

use anyhow::{bail, Result};
use indexmap::{IndexMap, IndexSet};
use duke::tree::class::ClassName;
use duke::tree::field::{FieldDescriptor, FieldName, FieldNameAndDesc, FieldRef};
use duke::tree::method::{MethodDescriptor, MethodName, MethodNameAndDesc, MethodRef};
use crate::tree::mappings::Mappings;
use crate::tree::names::Namespace;

/// A remapper supporting remapping of class names and descriptors.
pub trait ARemapper {
	fn map_class_fail(&self, class: &ClassName) -> Result<Option<ClassName>>;

	fn map_class(&self, class: &ClassName) -> Result<ClassName> {
		Ok(self.map_class_fail(class)?.unwrap_or_else(|| class.clone()))
	}

	fn map_field_desc(&self, desc: &FieldDescriptor) -> Result<FieldDescriptor> {
		Ok(self.map_desc(desc.as_str())?.into())
	}

	fn map_method_desc(&self, desc: &MethodDescriptor) -> Result<MethodDescriptor> {
		Ok(self.map_desc(desc.as_str())?.into())
	}

	fn map_desc(&self, desc: &str) -> Result<String> {
		let mut s = String::new();

		let mut iter = desc.chars();

		while let Some(ch) = iter.next() {
			s.push(ch);

			if ch == 'L' {
				let mut class_name = String::new();
				for ch in iter.by_ref() {
					class_name.push(ch);
					if ch == ';' {
						break;
					}
				}
				if class_name.pop() != Some(';') {
					bail!("descriptor {desc:?} has a missing semicolon somewhere");
				}

				let old_class_name = class_name.into();
				let new_class_name = self.map_class(&old_class_name)?;

				s.push_str(new_class_name.as_str());
				s.push(';');
			}
		}

		Ok(s)
	}
}

#[derive(Debug)]
pub struct ARemapperImpl<'a, const N: usize> {
	classes: IndexMap<&'a ClassName, &'a ClassName>,
}

impl<'a, const N: usize> ARemapper for ARemapperImpl<'a, N> {
	fn map_class_fail(&self, class: &ClassName) -> Result<Option<ClassName>> {
		match self.classes.get(class) {
			None => Ok(None),
			Some(&class) => Ok(Some(class.clone())),
		}
	}
}

impl<const N: usize> Mappings<N> {
	pub fn remapper_a(&self, from: Namespace<N>, to: Namespace<N>) -> Result<ARemapperImpl<'_, N>> {
		let mut classes = IndexMap::new();
		for class in self.classes.values() {
			if let (Some(from), Some(to)) = (&class.info.names[from], &class.info.names[to]) {
				classes.insert(from, to);
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
	fn map_field_fail(&self, owner_name: &ClassName, field_name: &FieldName, field_desc: &FieldDescriptor) -> Result<Option<FieldNameAndDesc>>;

	fn map_field(&self, class: &ClassName, field_name: &FieldName, field_desc: &FieldDescriptor) -> Result<FieldNameAndDesc> {
		self.map_field_fail(class, field_name, field_desc)?
			.map(Ok)
			.unwrap_or_else(|| Ok(FieldNameAndDesc {
				desc: self.map_field_desc(field_desc)?,
				name: field_name.clone(),
			}))
	}

	fn map_field_ref(&self, field_ref: &FieldRef) -> Result<FieldRef> {
		let field_key = self.map_field(&field_ref.class, &field_ref.name, &field_ref.desc)?;
		let class_name = self.map_class(&field_ref.class)?;

		Ok(FieldRef {
			class: class_name,
			name: field_key.name,
			desc: field_key.desc,
		})
	}

	fn map_method_fail(&self, owner_name: &ClassName, method_name: &MethodName, method_desc: &MethodDescriptor) -> Result<Option<MethodNameAndDesc>>;

	fn map_method(&self, class: &ClassName, method_name: &MethodName, method_desc: &MethodDescriptor) -> Result<MethodNameAndDesc> {
		self.map_method_fail(class, method_name, method_desc)?
			.map(Ok)
			.unwrap_or_else(|| Ok(MethodNameAndDesc {
				desc: self.map_method_desc(method_desc)?,
				name: method_name.clone(),
			}))
	}

	fn map_method_name_and_desc(&self, class: &ClassName, method_name_and_desc: &MethodNameAndDesc) -> Result<MethodNameAndDesc> {
		self.map_method(class, &method_name_and_desc.name, &method_name_and_desc.desc)
	}

	fn map_method_ref(&self, method_ref: &MethodRef) -> Result<MethodRef> {
		let method_key = self.map_method(&method_ref.class, &method_ref.name, &method_ref.desc)?;
		let class_name = self.map_class(&method_ref.class)?;

		Ok(MethodRef {
			class: class_name,
			name: method_key.name,
			desc: method_key.desc,
		})
	}
}

#[derive(Debug)]
struct BRemapperClass<'a> {
	name: &'a ClassName,
	fields: IndexMap<(&'a FieldName, FieldDescriptor), (&'a FieldName, FieldDescriptor)>,
	methods: IndexMap<(&'a MethodName, MethodDescriptor), (&'a MethodName, MethodDescriptor)>,
}

#[derive(Debug)]
pub struct BRemapperImpl<'a, 'i, const N: usize, I> {
	classes: IndexMap<&'a ClassName, BRemapperClass<'a>>,
	inheritance: &'i I,
}

impl<const N: usize, I> ARemapper for BRemapperImpl<'_, '_, N, I> {
	fn map_class_fail(&self, class: &ClassName) -> Result<Option<ClassName>> {
		match self.classes.get(class) {
			None => Ok(None),
			Some(class) => Ok(Some(class.name.clone())),
		}
	}
}

impl<'i, const N: usize, I: SuperClassProvider> BRemapper for BRemapperImpl<'_, 'i, N, I> {
	fn map_field_fail(&self, owner_name: &ClassName, field_name: &FieldName, field_desc: &FieldDescriptor) -> Result<Option<FieldNameAndDesc>> {
		assert!(!owner_name.as_str().is_empty());
		assert!(!owner_name.as_str().starts_with('['));

		if let Some(class) = self.classes.get(owner_name) {
			if let Some(&(name, ref desc)) = class.fields.get(&(field_name, field_desc.clone())) {
				let desc = desc.clone();
				let src = name.clone();
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

	fn map_method_fail(&self, owner_name: &ClassName, method_name: &MethodName, method_desc: &MethodDescriptor) -> Result<Option<MethodNameAndDesc>> {
		assert!(!owner_name.as_str().is_empty());
		assert!(!owner_name.as_str().starts_with('['));
		assert!(!method_name.as_str().is_empty());

		if let Some(class) = self.classes.get(owner_name) {
			if let Some(&(name, ref desc)) = class.methods.get(&(method_name, method_desc.clone())) {
				let desc = desc.clone();
				let src = name.clone();
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

						fields.insert((name_from, desc_from), (name_to, desc_to));
					}
				}

				let mut methods = IndexMap::new();
				for method in class.methods.values() {
					if let (Some(name_from), Some(name_to)) = (&method.info.names[from], &method.info.names[to]) {
						let desc_from = remapper_a_from.map_method_desc(&method.info.desc)?;
						let desc_to = remapper_a_to.map_method_desc(&method.info.desc)?;

						methods.insert((name_from, desc_from), (name_to, desc_to));
					}
				}

				classes.insert(name_from, BRemapperClass { name: name_to, fields, methods });
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
	pub super_classes: IndexMap<ClassName, IndexSet<ClassName>>,
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
	fn map_class_fail(&self, class: &ClassName) -> Result<Option<ClassName>> {
		self.0.map_class_fail(class)
	}
}

impl<T> BRemapper for ARemapperAsBRemapper<T> where T: ARemapper {
	fn map_field_fail(&self, owner_name: &ClassName, field_name: &FieldName, field_desc: &FieldDescriptor) -> Result<Option<FieldNameAndDesc>> {
		Ok(None)
	}

	fn map_method_fail(&self, owner_name: &ClassName, method_name: &MethodName, method_desc: &MethodDescriptor) -> Result<Option<MethodNameAndDesc>> {
		Ok(None)
	}
}


pub trait SuperClassProvider {
	fn get_super_classes(&self, class: &ClassName) -> Result<Option<&IndexSet<ClassName>>>;
}

impl SuperClassProvider for JarSuperProv {
	fn get_super_classes(&self, class: &ClassName) -> Result<Option<&IndexSet<ClassName>>> {
		Ok(self.super_classes.get(class))
	}
}

impl<S: SuperClassProvider> SuperClassProvider for Vec<S> {
	fn get_super_classes(&self, class: &ClassName) -> Result<Option<&IndexSet<ClassName>>> {
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
	fn get_super_classes(&self, class: &ClassName) -> Result<Option<&IndexSet<ClassName>>> {
		Ok(None)
	}
}

#[cfg(test)]
mod testing {
	// TODO: test internals
}
use anyhow::{bail, Result};
use indexmap::{IndexMap, IndexSet};
use class_file::tree::class::ClassName;
use class_file::tree::field::{FieldDescriptor, FieldName};
use class_file::tree::method::{MethodDescriptor, MethodName};
use crate::tree::mappings::{FieldKey, Mappings, MethodKey};
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
pub(crate) struct ARemapperImpl<'a, const N: usize> {
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
	pub(crate) fn remapper_a(&self, from: Namespace<N>, to: Namespace<N>) -> Result<ARemapperImpl<'_, N>> {
		let mut classes = IndexMap::new();
		for class in self.classes.values() {
			if let (Some(from), Some(to)) = (&class.info.names[from], &class.info.names[to]) {
				classes.insert(from, to);
			}
		}
		Ok(ARemapperImpl { classes })
	}
}

/// A remapper supporting remapping fields and methods, as well as class names and descriptors.
///
/// If you only want to remap class names and descriptors, consider using [ARemapper] instead.
pub trait BRemapper: ARemapper {
	fn map_field_fail(&self, owner_name: &ClassName, field_key: &FieldKey) -> Result<Option<FieldKey>>;

	fn map_field(&self, class: &ClassName, field: &FieldKey) -> Result<FieldKey> {
		Ok(self.map_field_fail(class, field)?.unwrap_or_else(|| field.clone()))
	}

	fn map_method_fail(&self, owner_name: &ClassName, method_key: &MethodKey) -> Result<Option<MethodKey>>;

	fn map_method(&self, class: &ClassName, method: &MethodKey) -> Result<MethodKey> {
		Ok(self.map_method_fail(class, method)?.unwrap_or_else(|| method.clone()))
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
	fn map_field_fail(&self, owner_name: &ClassName, field_key: &FieldKey) -> Result<Option<FieldKey>> {
		assert!(!owner_name.as_str().is_empty());
		assert!(!owner_name.as_str().starts_with('['));

		if let Some(class) = self.classes.get(owner_name) {
			if let Some(&(name, ref desc)) = class.fields.get(&(&field_key.name, field_key.desc.clone())) {
				let desc = desc.clone();
				let src = name.clone();
				return Ok(Some(FieldKey { desc, name: src }));
			}

			if let Some(super_classes) = self.inheritance.get_super_classes(owner_name)? {
				for super_class in super_classes {
					if let Some(remapped) = self.map_field_fail(super_class, field_key)? {
						return Ok(Some(remapped));
					}
				}
			}
		}

		Ok(None)
	}

	fn map_method_fail(&self, owner_name: &ClassName, method_key: &MethodKey) -> Result<Option<MethodKey>> {
		assert!(!owner_name.as_str().is_empty());
		assert!(!owner_name.as_str().starts_with('['));
		assert!(!method_key.name.as_str().is_empty());

		if let Some(class) = self.classes.get(owner_name) {
			if let Some(&(name, ref desc)) = class.methods.get(&(&method_key.name, method_key.desc.clone())) {
				let desc = desc.clone();
				let src = name.clone();
				return Ok(Some(MethodKey { desc, name: src }));
			}

			if let Some(super_classes) = self.inheritance.get_super_classes(owner_name)? {
				for super_class in super_classes {
					if let Some(remapped) = self.map_method_fail(super_class, method_key)? {
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

pub trait SuperClassProvider {
	fn get_super_classes(&self, class: &ClassName) -> Result<Option<&IndexSet<ClassName>>>;
}

#[cfg(test)]
mod testing {
	use anyhow::Result;
	use indexmap::{IndexMap, IndexSet};
	use pretty_assertions::assert_eq;
	use class_file::tree::class::ClassName;
	use crate::tree::action::remapper::{ARemapper, BRemapper, JarSuperProv};
	use crate::tree::mappings::{FieldKey, Mappings, MethodKey};

	#[test]
	fn remap() -> Result<()> {
		let input_a = include_str!("test/remap_input.tiny");

		let input_a: Mappings<2> = crate::tiny_v2::read(input_a.as_bytes())?;

		let super_classes_provider = JarSuperProv { super_classes: IndexMap::from([
			(ClassName::from("classS1"), IndexSet::from([
				ClassName::from("classS2"),
				ClassName::from("classS3"),
				ClassName::from("classS4"),
			])),
			(ClassName::from("classS2"), IndexSet::from([
				ClassName::from("classS5"),
			])),
			(ClassName::from("classS3"), IndexSet::from([
				ClassName::from("classS5"),
			])),
			(ClassName::from("classS4"), IndexSet::from([
				ClassName::from("classS5"),
			])),
			(ClassName::from("classS5"), IndexSet::from([
				ClassName::from("java/lang/Object"),
			])),
		]) };

		let from = input_a.get_namespace("namespaceA")?;
		let to = input_a.get_namespace("namespaceB")?;
		let remapper = input_a.remapper_b(from, to, &super_classes_provider)?;

		let class = |class: &'static str| -> Result<String> {
			let class = ClassName::from(class);

			let class_new = remapper.map_class(&class)?;

			Ok(class_new.into())
		};
		let field = |class: &'static str, field: &'static str, descriptor: &'static str| -> Result<(String, String, String)> {
			let class = ClassName::from(class);
			let field = FieldKey { desc: descriptor.into(), name: field.into() };

			let class_new = remapper.map_class(&class)?;
			let field_new = remapper.map_field(&class, &field)?;

			Ok((class_new.into(), field_new.name.into(), field_new.desc.into()))
		};
		let method = |class: &'static str, method: &'static str, descriptor: &'static str| -> Result<(String, String, String)> {
			let class = ClassName::from(class);
			let method = MethodKey { desc: descriptor.into(), name: method.into() };

			let class_new = remapper.map_class(&class)?;
			let method_new = remapper.map_method(&class, &method)?;

			Ok((class_new.into(), method_new.name.into(), method_new.desc.into()))
		};

		assert_eq!(class("classA1")?, "classB1");
		assert_eq!(class("classA2")?, "classB2");
		assert_eq!(class("classA2$innerA1")?, "classB2$innerB1");
		assert_eq!(class("classA3")?, "classB3");
		assert_eq!(class("classA4L")?, "classB4L");

		assert_eq!(field("classA1", "field1A1", "I")?,
			("classB1".into(), "field1B1".into(), "I".into()));
		assert_eq!(field("classA1", "field1A2", "Ljava/lang/Object;")?,
			("classB1".into(), "field1B2".into(), "Ljava/lang/Object;".into()));
		assert_eq!(field("classA1", "field1A3", "LclassA1;")?,
			("classB1".into(), "field1B3".into(), "LclassB1;".into()));
		assert_eq!(field("classA1", "field1A4", "LclassA2$innerA1;")?,
			("classB1".into(), "field1B4".into(), "LclassB2$innerB1;".into()));
		assert_eq!(field("classA1", "field1A1", "[I")?,
			("classB1".into(), "field1B1".into(), "[I".into()));
		assert_eq!(field("classA1", "field1A2", "[Ljava/lang/Object;")?,
			("classB1".into(), "field1B2".into(), "[Ljava/lang/Object;".into()));
		assert_eq!(field("classA1", "field1A3", "[LclassA1;")?,
			("classB1".into(), "field1B3".into(), "[LclassB1;".into()));
		assert_eq!(field("classA1", "field1A4", "[LclassA2$innerA1;")?,
			("classB1".into(), "field1B4".into(), "[LclassB2$innerB1;".into()));
		assert_eq!(field("classA1", "field1A1", "[[[[I")?,
			("classB1".into(), "field1B1".into(), "[[[[I".into()));
		assert_eq!(field("classA1", "field1A2", "[[[[Ljava/lang/Object;")?,
			("classB1".into(), "field1B2".into(), "[[[[Ljava/lang/Object;".into()));
		assert_eq!(field("classA1", "field1A3", "[[[[LclassA1;")?,
			("classB1".into(), "field1B3".into(), "[[[[LclassB1;".into()));
		assert_eq!(field("classA1", "field1A4", "[[[[LclassA2$innerA1;")?,
			("classB1".into(), "field1B4".into(), "[[[[LclassB2$innerB1;".into()));

		assert_eq!(method("classA2", "method2A1", "()V")?,
			("classB2".into(), "method2B1".into(), "()V".into()));
		assert_eq!(method("classA2", "method2A2", "(I)I")?,
			("classB2".into(), "method2B2".into(), "(I)I".into()));
		assert_eq!(method("classA2", "method2A3", "(Ljava/lang/Integer;)Ljava/lang/Object;")?,
			("classB2".into(), "method2B3".into(), "(Ljava/lang/Integer;)Ljava/lang/Object;".into()));

		assert_eq!(method("classA2$innerA1", "<init>", "()V")?,
			("classB2$innerB1".into(), "<init>".into(), "()V".into()));

		assert_eq!(method("classA3", "method3A1", "(BCDFJSZ)V")?,
			("classB3".into(), "method3B1".into(), "(BCDFJSZ)V".into()));
		assert_eq!(method("classA3", "method3A2", "(LclassA1;LclassA2$innerA1;LclassA2;)LclassA3;")?,
			("classB3".into(), "method3B2".into(), "(LclassB1;LclassB2$innerB1;LclassB2;)LclassB3;".into()));
		assert_eq!(method("classA3", "method3A2", "([LclassA1;[LclassA2$innerA1;[LclassA2;)[LclassA3;")?,
			("classB3".into(), "method3B2".into(), "([LclassB1;[LclassB2$innerB1;[LclassB2;)[LclassB3;".into()));
		assert_eq!(method("classA3", "method3A2", "([LclassA2$innerA1;LclassA2$innerA1;[[[LclassA2;)[[[LclassA3;")?,
			("classB3".into(), "method3B2".into(), "([LclassB2$innerB1;LclassB2$innerB1;[[[LclassB2;)[[[LclassB3;".into()));
		assert_eq!(method("classA3", "method3A2", "([LclassA1;[[[LclassA2$innerA1;LclassA2;)[[[LclassA2$innerA1;")?,
			("classB3".into(), "method3B2".into(), "([LclassB1;[[[LclassB2$innerB1;LclassB2;)[[[LclassB2$innerB1;".into()));
		assert_eq!(method("classA3", "method3A3", "([B[C[D[F[J[S[Z)I")?,
			("classB3".into(), "method3B3".into(), "([B[C[D[F[J[S[Z)I".into()));
		assert_eq!(method("classA3", "method3A3", "([[B[[C[[D[[F[[J[[S[[Z)[[I")?,
			("classB3".into(), "method3B3".into(), "([[B[[C[[D[[F[[J[[S[[Z)[[I".into()));

		assert_eq!(field("classA4L", "field4A1", "LclassA4L;")?,
			("classB4L".into(), "field4B1".into(), "LclassB4L;".into()));
		assert_eq!(method("classA4L", "method4A1", "(LclassA4L;)LclassA4L;")?,
			("classB4L".into(), "method4B1".into(), "(LclassB4L;)LclassB4L;".into()));

		// Tests for super classes:
		assert_eq!(class("classS1")?, "classS1_");
		assert_eq!(class("classS2")?, "classS2_");
		assert_eq!(class("classS3")?, "classS3_");
		assert_eq!(class("classS4")?, "classS4_");
		assert_eq!(class("classS5")?, "classS5_");

		assert_eq!(field("classS1", "fieldFromS1", "I")?, ("classS1_".into(), "fieldFromS1_".into(), "I".into()));
		assert_eq!(field("classS1", "fieldFromS2", "I")?, ("classS1_".into(), "fieldFromS2_".into(), "I".into()));
		assert_eq!(field("classS1", "fieldFromS3", "I")?, ("classS1_".into(), "fieldFromS3_".into(), "I".into()));
		assert_eq!(field("classS1", "fieldFromS4", "I")?, ("classS1_".into(), "fieldFromS4_".into(), "I".into()));
		assert_eq!(field("classS1", "fieldFromS5", "I")?, ("classS1_".into(), "fieldFromS5_".into(), "I".into()));
		assert_eq!(field("classS2", "fieldFromS2", "I")?, ("classS2_".into(), "fieldFromS2_".into(), "I".into()));
		assert_eq!(field("classS2", "fieldFromS5", "I")?, ("classS2_".into(), "fieldFromS5_".into(), "I".into()));
		assert_eq!(field("classS3", "fieldFromS3", "I")?, ("classS3_".into(), "fieldFromS3_".into(), "I".into()));
		assert_eq!(field("classS3", "fieldFromS5", "I")?, ("classS3_".into(), "fieldFromS5_".into(), "I".into()));
		assert_eq!(field("classS4", "fieldFromS4", "I")?, ("classS4_".into(), "fieldFromS4_".into(), "I".into()));
		assert_eq!(field("classS4", "fieldFromS5", "I")?, ("classS4_".into(), "fieldFromS5_".into(), "I".into()));
		assert_eq!(field("classS5", "fieldFromS5", "I")?, ("classS5_".into(), "fieldFromS5_".into(), "I".into()));

		assert_eq!(method("classS1", "methodFromS1", "(I)I")?, ("classS1_".into(), "methodFromS1_".into(), "(I)I".into()));
		assert_eq!(method("classS1", "methodFromS2", "(I)I")?, ("classS1_".into(), "methodFromS2_".into(), "(I)I".into()));
		assert_eq!(method("classS1", "methodFromS3", "(I)I")?, ("classS1_".into(), "methodFromS3_".into(), "(I)I".into()));
		assert_eq!(method("classS1", "methodFromS4", "(I)I")?, ("classS1_".into(), "methodFromS4_".into(), "(I)I".into()));
		assert_eq!(method("classS1", "methodFromS5", "(I)I")?, ("classS1_".into(), "methodFromS5_".into(), "(I)I".into()));
		assert_eq!(method("classS2", "methodFromS2", "(I)I")?, ("classS2_".into(), "methodFromS2_".into(), "(I)I".into()));
		assert_eq!(method("classS2", "methodFromS5", "(I)I")?, ("classS2_".into(), "methodFromS5_".into(), "(I)I".into()));
		assert_eq!(method("classS3", "methodFromS3", "(I)I")?, ("classS3_".into(), "methodFromS3_".into(), "(I)I".into()));
		assert_eq!(method("classS3", "methodFromS5", "(I)I")?, ("classS3_".into(), "methodFromS5_".into(), "(I)I".into()));
		assert_eq!(method("classS4", "methodFromS4", "(I)I")?, ("classS4_".into(), "methodFromS4_".into(), "(I)I".into()));
		assert_eq!(method("classS4", "methodFromS5", "(I)I")?, ("classS4_".into(), "methodFromS5_".into(), "(I)I".into()));
		assert_eq!(method("classS5", "methodFromS5", "(I)I")?, ("classS5_".into(), "methodFromS5_".into(), "(I)I".into()));

		// TODO: another test method: also test if failures are there

		Ok(())
	}
}
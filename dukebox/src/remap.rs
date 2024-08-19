use std::fmt::Debug;
use std::hash::Hash;
use anyhow::{bail, Result};
use indexmap::IndexMap;
use indexmap::map::Entry;
use duke::tree::annotation::{Annotation, ElementValue, ElementValuePair};
use duke::tree::class::{ClassFile, ClassName, ClassSignature, EnclosingMethod, InnerClass};
use duke::tree::field::{ConstantValue, Field, FieldDescriptor, FieldSignature};
use duke::tree::method::Method;
use duke::tree::type_annotation::{TargetInfoClass, TargetInfoField, TargetInfoMethod, TypeAnnotation};
use quill::remapper::BRemapper;
use quill::tree::mappings::{Mappings};
use crate::{Jar, JarEntry, OpenedJar};
use crate::lazy_duke::ClassRepr;
use crate::parsed::{ParsedJar, ParsedJarEntry};


// TODO: doc
pub fn remap(jar: impl Jar, remapper: impl BRemapper) -> Result<ParsedJar> {
	let mut opened = jar.open()?;

	let mut resulting_entries = IndexMap::new();

	for key in opened.entry_keys() {
		let entry = opened.by_entry_key(key)?;

		let name = entry.name().to_owned(); // TODO: also remap the entry name for a class!

		let entry = remap_jar_entry(entry.to_parsed_jar_entry()?, &remapper)?;

		resulting_entries.insert(name, entry);
	}

	Ok(ParsedJar { entries: resulting_entries })
}

pub fn remap_jar_entry_name(name: String, remapper: &impl BRemapper) -> Result<String> {
	return Ok(name);
	todo!("remap a name of a jar entry")
}

pub fn remap_jar_entry(entry: ParsedJarEntry, remapper: &impl BRemapper) -> Result<ParsedJarEntry> {
	Ok(match entry {
		ParsedJarEntry::Class { attr, class } => {
			let class = class.read()?.remap(remapper)?.into();
			ParsedJarEntry::Class { attr, class }
		},
		e @ ParsedJarEntry::Other { .. } => e,
		e @ ParsedJarEntry::Dir { .. } => e,
	})
}

trait Mappable: Sized {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self>;
}

impl<T> Mappable for Option<T> where T: Mappable {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		self.map(|x| x.remap(remapper)).transpose()
	}
}

impl<T> Mappable for Vec<T> where T: Mappable {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		self.into_iter()
			.map(|i| i.remap(remapper))
			.collect()
	}
}

impl Mappable for ClassName {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		remapper.map_class(&self)
	}
}

impl Mappable for ClassSignature {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for ClassFile {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(ClassFile {
			version: self.version,
			access: self.access,
			name: self.name.remap(remapper)?,
			super_class: self.super_class.remap(remapper)?,
			interfaces: self.interfaces.remap(remapper)?,

			fields: self.fields.into_iter()
				.map(|field| {
					Ok(Field {
						access: field.access,
						name: field.name, // TODO  // TODO: (takes in self.name as well)
						descriptor: field.descriptor, // TODO

						has_deprecated_attribute: field.has_deprecated_attribute,
						has_synthetic_attribute: field.has_synthetic_attribute,

						constant_value: field.constant_value.remap(remapper)?,
						signature: field.signature.remap(remapper)?,

						runtime_visible_annotations: field.runtime_visible_annotations.remap(remapper)?,
						runtime_invisible_annotations: field.runtime_invisible_annotations.remap(remapper)?,
						runtime_visible_type_annotations: field.runtime_visible_type_annotations.remap(remapper)?,
						runtime_invisible_type_annotations: field.runtime_invisible_type_annotations.remap(remapper)?,

						attributes: Vec::new(), // TODO
					})
				})
				.collect::<Result<_>>()?,
			methods: self.methods.into_iter()
				.map(|method| {
					Ok(Method {
						access: method.access,
						// TODO: (takes in self.name as well)
						name: method.name,
						descriptor: method.descriptor,

						has_deprecated_attribute: method.has_deprecated_attribute,
						has_synthetic_attribute: method.has_synthetic_attribute,

						// TODO: more fields
						code: None,
						exceptions: None,
						signature: None,

						runtime_visible_annotations: method.runtime_visible_annotations.remap(remapper)?,
						runtime_invisible_annotations: method.runtime_invisible_annotations.remap(remapper)?,
						runtime_visible_type_annotations: method.runtime_visible_type_annotations.remap(remapper)?,
						runtime_invisible_type_annotations: method.runtime_invisible_type_annotations.remap(remapper)?,

						annotation_default: None,
						method_parameters: None,

						attributes: Vec::new(),
					})
				})
				.collect::<Result<_>>()?,

			has_deprecated_attribute: self.has_deprecated_attribute,
			has_synthetic_attribute: self.has_synthetic_attribute,

			inner_classes: self.inner_classes.remap(remapper)?,
			enclosing_method: self.enclosing_method.remap(remapper)?,
			signature: self.signature.remap(remapper)?,

			source_file: self.source_file, // TODO
			source_debug_extension: self.source_debug_extension, // TODO

			runtime_visible_annotations: self.runtime_visible_annotations.remap(remapper)?,
			runtime_invisible_annotations: self.runtime_invisible_annotations.remap(remapper)?,
			runtime_visible_type_annotations: self.runtime_visible_type_annotations.remap(remapper)?,
			runtime_invisible_type_annotations: self.runtime_invisible_type_annotations.remap(remapper)?,

			module: None, // TODO
			module_packages: None, // TODO
			module_main_class: None, // TODO

			nest_host_class: None, // TODO
			nest_members: None, // TODO
			permitted_subclasses: None, // TODO

			record_components: Vec::new(), // TODO (takes in self.name as well)

			attributes: Vec::new(), // TODO
		})
	}
}

impl Mappable for FieldDescriptor {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		remapper.map_field_desc(&self)
	}
}

impl Mappable for FieldSignature {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for InnerClass {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(InnerClass {
			inner_class: remapper.map_class(&self.inner_class)?,
			outer_class: self.outer_class.as_ref().map(|outer_class| remapper.map_class(outer_class)).transpose()?,
			inner_name: self.inner_name.map(|inner_name| map_inner_class_name(
				remapper, &self.inner_class, self.outer_class.as_ref(), &inner_name)).transpose()?,
			flags: self.flags,
		})
	}
}

impl Mappable for EnclosingMethod {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(EnclosingMethod {
			class: remapper.map_class(&self.class)?,
			method: self.method.map(|method| remapper.map_method_name_and_desc(&self.class, &method)).transpose()?
		})
	}
}

impl Mappable for Annotation {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(Annotation {
			annotation_type: self.annotation_type.remap(remapper)?,
			element_value_pairs: self.element_value_pairs.remap(remapper)?,
		})
	}
}

impl Mappable for TypeAnnotation<TargetInfoClass> {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for TypeAnnotation<TargetInfoField> {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for TypeAnnotation<TargetInfoMethod> {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		todo!()
	}
}

impl Mappable for ElementValuePair {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(ElementValuePair {
			name: self.name,
			value: self.value.remap(remapper)?,
		})
	}
}

impl Mappable for ElementValue {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(match self {
			ElementValue::Object(x) => ElementValue::Object(x),
			ElementValue::Enum { type_name, const_name } => ElementValue::Enum { type_name, const_name }, // TODO: these two need remapping!
			ElementValue::Class(class_name) => ElementValue::Class(class_name), // TODO: this is actually a return descriptor, and should be remapped!
			ElementValue::AnnotationInterface(annotation) => ElementValue::AnnotationInterface(annotation.remap(remapper)?),
			ElementValue::ArrayType(vec) => ElementValue::ArrayType(vec.remap(remapper)?),
		})
	}
}

impl Mappable for ConstantValue {
	fn remap(self, remapper: &impl BRemapper) -> Result<Self> {
		Ok(self) // TODO: impl this
	}
}

fn map_inner_class_name(remapper: &impl BRemapper, name: &ClassName, outer_class: Option<&ClassName>, inner_name: &String) -> Result<String> {
	return Ok(inner_name.clone());
	todo!()
}
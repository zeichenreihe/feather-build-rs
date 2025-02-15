use std::collections::HashSet;
use anyhow::{Context, Result};
use indexmap::{IndexMap, IndexSet};
use java_string::{JavaCodePoint, JavaStr, JavaString};
use duke::tree::class::{ClassAccess, ClassFile, EnclosingMethod, InnerClass, ObjClassName, ObjClassNameSlice};
use duke::tree::method::{Method, MethodNameAndDesc};
use dukebox::storage::{BasicFileAttributes, ClassRepr, IsClass, IsOther, Jar, JarEntry, JarEntryEnum, OpenedJar, ParsedJar, ParsedJarEntry};
use quill::remapper::{ARemapper, ARemapperAsBRemapper};
use crate::nest::{Nest, Nests, NestType};

// we assume class_node.name matches the name of the JarEntry

pub fn nest_jar<A>(remap_option: bool, src: &impl Jar, nests: Nests<A>) -> Result<ParsedJar<ClassRepr, Vec<u8>>> {
	let mut class_version = None;
	let mut jar_new_classes = IndexMap::new();
	let mut methods_map: IndexMap<ObjClassName, HashSet<MethodNameAndDesc>> = IndexMap::new();
	let mut classes_in_jar: IndexSet<ObjClassName> = IndexSet::new();

	let mut opened_src = src.open()?;

	for key in opened_src.entry_keys() {
		let entry = opened_src.by_entry_key(key)?;

		// TODO: the original code reads this with a reader that SKIP_{FRAMES,CODE,DEBUG}
		//  my guess is that we could do even better by providing our own ClassVisitor impl here...
		if let JarEntryEnum::Class(class) = entry.to_jar_entry_enum()? {
			let class_node = class.read()?;

			if class_version.is_none() || class_version.is_some_and(|class_version| class_node.version < class_version) {
				class_version = Some(class_node.version);
			}

			let methods = class_node.methods.into_iter()
				.map(Method::into_name_and_desc)
				.collect();

			classes_in_jar.insert(class_node.name.clone());
			methods_map.insert(class_node.name, methods);
		}
	}

	let methods_map = methods_map;

	let class_version = class_version.context("no classes in input")?;

	let this_nests: IndexMap<_, _> = nests.all
		.into_iter()
		.filter(|(_, nest)| {
			// _ is nest.class_name!

			if !classes_in_jar.contains(&nest.class_name) {
				return false;
			}

			if !classes_in_jar.contains(&nest.encl_class_name) {
				jar_new_classes.entry(nest.encl_class_name.clone())
					.or_insert_with(|| ClassFile::new(
						class_version,
						ClassAccess {
							is_public: true,
							..ClassAccess::default()
						},
						nest.encl_class_name.clone(),
						Some(ObjClassName::JAVA_LANG_OBJECT.to_owned()),
						vec![]
					));

				classes_in_jar.insert(nest.encl_class_name.clone());
			}

			let has_encl_method = nest.encl_method.as_ref().and_then(|encl_method| {
				methods_map.get(&nest.encl_class_name)
					.map(|methods| methods.contains(encl_method))
			})
				.unwrap_or_default();

			match nest.nest_type {
				// anonymous class may have an enclosing method, they may not
				// for anonymous classes, the inner name is typically a number: their anonymous class index
				NestType::Anonymous => nest.inner_name.as_inner().parse::<i32>().map_or(false, |x| x >= 1),

				// inner classes NEVER have an enclosing method
				NestType::Inner => !has_encl_method,

				// local classes ALWAYS have an enclosing method
				NestType::Local => has_encl_method,
			}
		})
		.collect();

	let mut dst_resulting_entries = IndexMap::new();

	// only when remapping it's needed
	fn remap(this_nests: &IndexMap<ObjClassName, Nest>, corresponding_nest: &Nest) -> ObjClassName {
		let result = this_nests.get(&corresponding_nest.encl_class_name)
			.map(|nest| remap(this_nests, nest))
			.unwrap_or_else(|| corresponding_nest.encl_class_name.clone());

		let mut s: JavaString = result.into_inner();
		s.push('$');
		s.push_java_str(corresponding_nest.inner_name.as_inner());
		// TODO: redo this safety comment
		// SAFETY: Joining a class name with `$` and an inner name is always valid.
		unsafe { ObjClassName::from_inner_unchecked(s) }
	}

	let map = this_nests.iter()
		.map(|(old_name, nest)| (old_name.as_slice(), remap(&this_nests, nest)))
		.filter(|(old_name, new_name)| old_name != new_name)
		.collect();

	struct MyRemapper<'a>(IndexMap<&'a ObjClassNameSlice, ObjClassName>);
	impl ARemapper for MyRemapper<'_> {
		fn map_class_fail(&self, class: &ObjClassNameSlice) -> Result<Option<ObjClassName>> {
			Ok(self.0.get(class).cloned())
		}
	}

	let remapper = ARemapperAsBRemapper(MyRemapper(map));
	// end of that

	for new_class in jar_new_classes.into_values() {
		let new_class_name = new_class.name.as_inner();

		let entry_attr = BasicFileAttributes::default();

		let (name, class_node) = if remap_option {
			let name = dukebox::remap::remap_jar_entry_name_java(&new_class_name, &remapper)?
				.into_string().unwrap(); // TODO: unwrap
			let class_node = do_nested_class_attribute_class_visitor(&this_nests, new_class);
			let class_node = dukebox::remap::remap_class(&remapper, class_node)?;

			(name, class_node)
		} else {
			let name = new_class_name.to_owned().into_string().expect("a class name contained unmatched surrogate pairs") + ".class"; // TODO: unwrap
			let class_node = do_nested_class_attribute_class_visitor(&this_nests, new_class);
			(name, class_node)
		};

		let entry = ParsedJarEntry {
			attr: entry_attr,
			content: JarEntryEnum::Class(ClassRepr::Parsed { class: class_node }),
		};

		dst_resulting_entries.insert(name, entry);
	}


	for key in opened_src.entry_keys() {
		let entry = opened_src.by_entry_key(key)?;

		let name = entry.name().to_owned();
		let attr = entry.attrs();

		use JarEntryEnum::*;
		let (name, content) = match entry.to_jar_entry_enum()? {
			Dir => (name, Dir),
			Class(class) => {
				let class_node = class.read()?;

				let class_node = do_nested_class_attribute_class_visitor(&this_nests, class_node);

				let (name, class_node) = if remap_option {
					let name = dukebox::remap::remap_jar_entry_name(&name, &remapper)?;
					let class_node = dukebox::remap::remap_class(&remapper, class_node)?;

					(name, class_node)
				} else {
					(name, class_node)
				};
				let content = Class(ClassRepr::Parsed { class: class_node });

				(name, content)
			},
			Other(other) => (name, Other(other.get_data_owned())),
		};

		let entry = ParsedJarEntry {
			attr,
			content,
		};

		dst_resulting_entries.insert(name, entry);
	}

	Ok(ParsedJar { entries: dst_resulting_entries })

}

fn do_nested_class_attribute_class_visitor(this_nests: &IndexMap<ObjClassName, Nest>, mut class_node: ClassFile) -> ClassFile {

	if let Some(nest) = this_nests.get(&class_node.name) {
		if matches!(nest.nest_type, NestType::Anonymous | NestType::Local) {
			class_node.enclosing_method = Some(EnclosingMethod {
				class: nest.encl_class_name.clone().into(),
				method: nest.encl_method.clone(),
			});
		}

		class_node.inner_classes.get_or_insert_with(Vec::new)
			.push(InnerClass {
				inner_class: nest.class_name.clone().into(),
				outer_class: if matches!(nest.nest_type, NestType::Inner) {
					Some(nest.encl_class_name.clone().into())
				} else {
					None
				},
				inner_name: if matches!(nest.nest_type, NestType::Inner | NestType::Local) {
					Some(strip_local_class_prefix(nest.inner_name.as_inner()).to_owned())
				} else {
					None
				},
				flags: nest.inner_access,
			});
	}

	class_node
}


fn strip_local_class_prefix(inner_name: &JavaStr) -> &JavaStr {
	// local class names start with a number prefix
	// remove all of that
	let stripped = inner_name.trim_start_matches(|ch: JavaCodePoint| ch.is_ascii_digit());

	if stripped.is_empty() {
		// entire inner name is a number, so this class is anonymous, not local
		inner_name
	} else {
		stripped
	}
}


#[cfg(test)]
mod testing {
	use pretty_assertions::assert_eq;
	use super::strip_local_class_prefix;

	#[test]
	fn strip_local_class_prefix_test() {
		assert_eq!(strip_local_class_prefix("".into()), "");
		assert_eq!(strip_local_class_prefix("FooBar".into()), "FooBar");
		assert_eq!(strip_local_class_prefix("1234".into()), "1234");
		assert_eq!(strip_local_class_prefix("123Foo".into()), "Foo");
		assert_eq!(strip_local_class_prefix("123Bar4".into()), "Bar4");
	}
}


use std::collections::HashSet;
use anyhow::{Context, Result};
use indexmap::{IndexMap, IndexSet};
use duke::tree::class::{ClassAccess, ClassFile, ClassName, ClassNameSlice, EnclosingMethod, InnerClass, InnerClassFlags};
use duke::tree::method::MethodNameAndDesc;
use dukebox::storage::{BasicFileAttributes, ClassRepr, IsClass, IsOther, Jar, JarEntry, JarEntryEnum, OpenedJar, ParsedJar, ParsedJarEntry};
use quill::remapper::{ARemapper, ARemapperAsBRemapper, BRemapper, NoSuperClassProvider};
use quill::tree::mappings::Mappings;

mod io;

#[derive(Clone)]
pub(crate) enum NestType {
	Anonymous,
	Inner,
	Local,
}

pub(crate) struct Nest {
	pub(crate) nest_type: NestType,

	pub(crate) class_name: ClassName,
	pub(crate) encl_class_name: ClassName,
	pub(crate) encl_method: Option<MethodNameAndDesc>,

	pub(crate) inner_name: String,
	pub(crate) inner_access: InnerClassFlags,
}

pub struct Nests {
	pub(crate) all: IndexMap<ClassName, Nest>,
}

impl Nests {
	pub(crate) fn new() -> Nests {
		Nests { all: IndexMap::new() }
	}

	pub(crate) fn add(&mut self, nest: Nest) {
		self.all.insert(nest.class_name.clone(), nest);
	}
}

pub struct NesterOptions {
	silent: bool,
	remap: bool,
}

impl Default for NesterOptions {
	fn default() -> Self {
		NesterOptions { silent: false, remap: true }
	}
}

impl NesterOptions {
	pub fn silent(self, silent: bool) -> NesterOptions {
		NesterOptions { silent, ..self }
	}

	pub fn remap(self, remap: bool) -> NesterOptions {
		NesterOptions { remap, ..self }
	}
}

// we assume class_node.name matches the name of the JarEntry

pub fn nest_jar(options: NesterOptions, src: &impl Jar, nests: Nests) -> Result<ParsedJar<ClassRepr, Vec<u8>>> {
	let mut class_version = None;
	let mut jar_new_classes = IndexMap::new();
	let mut methods_map: IndexMap<ClassName, HashSet<MethodNameAndDesc>> = IndexMap::new();
	let mut classes_in_jar: IndexSet<ClassName> = IndexSet::new();

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
				.map(|m| MethodNameAndDesc {
					name: m.name,
					desc: m.descriptor,
				})
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
						Some(ClassName::JAVA_LANG_OBJECT.to_owned()),
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
				NestType::Anonymous => nest.inner_name.parse::<i32>().map_or(false, |x| x >= 1),

				// inner classes NEVER have an enclosing method
				NestType::Inner => !has_encl_method,

				// local classes ALWAYS have an enclosing method
				NestType::Local => has_encl_method,
			}
		})
		.collect();

	if !options.silent {
		println!("Prepared {} nests...", this_nests.len());
	}

	let mut dst_resulting_entries = IndexMap::new();

	// only when remapping it's needed
		fn remap(this_nests: &IndexMap<ClassName, Nest>, corresponding_nest: &Nest) -> ClassName {
			let result = this_nests.get(&corresponding_nest.encl_class_name)
				.map(|nest| remap(this_nests, nest))
				.unwrap_or_else(|| corresponding_nest.encl_class_name.clone());

			let mut s: String = result.into();
			s.push('$');
			s.push_str(corresponding_nest.inner_name.as_str());
			unsafe { ClassName::from_inner_unchecked(s) }
		}

		let map = this_nests.iter()
			.map(|(old_name, nest)| (old_name.as_slice(), remap(&this_nests, nest)))
			.filter(|(old_name, new_name)| old_name != new_name)
			.collect();

		struct MyRemapper<'a>(IndexMap<&'a ClassNameSlice, ClassName>);
		impl ARemapper for MyRemapper<'_> {
			fn map_class_fail(&self, class: &ClassNameSlice) -> Result<Option<ClassName>> {
				Ok(self.0.get(class).cloned())
			}
		}

		let remapper = ARemapperAsBRemapper(MyRemapper(map));
	// end of that

	for new_class in jar_new_classes.into_values() {
		let name = new_class.name.as_inner().to_owned() + ".class";

		let class_node = do_nested_class_attribute_class_visitor(&this_nests, new_class);

		let entry_attr = BasicFileAttributes::default();

		let (name, class_node) = if options.remap {
			let name = dukebox::remap::remap_jar_entry_name(&name, &remapper)?;
			let class_node = dukebox::remap::remap_class(&remapper, class_node)?;

			(name, class_node)
		} else {
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

				let (name, class_node) = if options.remap {
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

	// TODO: use log::_? also do we really need this?
	if !options.silent {
		println!("Applied nests...");
		if options.remap {
			println!("Remapped nested classes...");
		}
		println!("Moved over non-class files...");
		println!("Sorted class files...");
		println!("Done!");
	}

	Ok(ParsedJar { entries: dst_resulting_entries })

}

fn do_nested_class_attribute_class_visitor(this_nests: &IndexMap<ClassName, Nest>, mut class_node: ClassFile) -> ClassFile {

	if let Some(nest) = this_nests.get(&class_node.name) {
		if matches!(nest.nest_type, NestType::Anonymous | NestType::Local) {
			class_node.enclosing_method = Some(EnclosingMethod {
				class: nest.encl_class_name.clone(),
				method: nest.encl_method.clone(),
			});
		}

		class_node.inner_classes.get_or_insert_with(Vec::new)
			.push(InnerClass {
				inner_class: nest.class_name.clone(),
				outer_class: if matches!(nest.nest_type, NestType::Inner) {
					Some(nest.encl_class_name.clone())
				} else {
					None
				},
				inner_name: if matches!(nest.nest_type, NestType::Inner | NestType::Local) {
					Some(strip_local_class_prefix(&nest.inner_name).to_owned())
				} else {
					None
				},
				flags: nest.inner_access,
			});
	}

	class_node
}

fn strip_local_class_prefix(inner_name: &str) -> &str {
	// local class names start with a number prefix
	// remove all of that
	let stripped = inner_name.trim_start_matches(|ch: char| ch.is_ascii_digit());

	if stripped.is_empty() {
		// entire inner name is a number, so this class is anonymous, not local
		inner_name
	} else {
		stripped
	}
}

pub fn map_nests(mappings: &Mappings<2>, nests: Nests) -> Result<Nests> {
	let remapper = mappings.remapper_b_first_to_second(NoSuperClassProvider::new())?;

	let mut dst = Nests::new();

	for (_, nest) in nests.all {

		let mapped_name = remapper.map_class(&nest.class_name)?;

		let (encl_class_name, inner_name) = if let Some((encl_class_name, inner_name)) = mapped_name.as_inner().rsplit_once("__") {
			// provided mappings already use nesting
			(unsafe { ClassName::from_inner_unchecked(encl_class_name.to_owned()) }, inner_name.to_owned())
		} else {
			let encl_class_name = remapper.map_class(&nest.encl_class_name)?;

			let inner_name = {
				let i = nest.inner_name.char_indices()
					.take_while(|(_, ch)| ch.is_ascii_digit())
					.map(|(pos, _)| pos)
					.last()
					.unwrap_or(0);

				if i < nest.inner_name.len() {
					// TODO: dangerous direct access to string slice... (before touching this, write a test!)
					// local classes have a number prefix
					let prefix = &nest.inner_name[0..i];
					let simple_name = &nest.inner_name[i..];

					// make sure the class does not have custom inner name
					if nest.class_name.as_inner().ends_with(simple_name) {
						let mut s = String::new();
						s.push_str(prefix);
						s.push_str(if let Some((_, substring)) = mapped_name.as_inner().rsplit_once('/') {
							substring
						} else {
							mapped_name.as_inner()
						});
						s
					} else {
						nest.inner_name
					}
				} else {
					// anonymous class
					let simple_name = if let Some((_, substring)) = mapped_name.as_inner().rsplit_once('/') {
						substring
					} else {
						mapped_name.as_inner()
					};

					if let Some(number) = simple_name.strip_prefix("C_") {
						// mapped name is Calamus intermediary format C_<number>
						// we strip the C_ prefix and keep the number as the inner name
						number.to_owned()
					} else {
						// keep the inner name given by the nests file
						nest.inner_name
					}
				}
			};

			(encl_class_name, inner_name)
		};

		let nest = Nest {
			nest_type: nest.nest_type,
			class_name: mapped_name,
			encl_class_name,
			encl_method: nest.encl_method.map(|method_name_and_desc| {
				remapper.map_method_name_and_desc(&nest.encl_class_name, &method_name_and_desc)
			}).transpose()?,
			inner_name,
			inner_access: nest.inner_access,
		};

		dst.add(nest);
	}

	Ok(dst)
}


#[cfg(test)]
mod testing {
	use pretty_assertions::assert_eq;
	use crate::strip_local_class_prefix;

	#[test]
	fn strip_local_class_prefix_test() {
		assert_eq!(strip_local_class_prefix(""), "");
		assert_eq!(strip_local_class_prefix("FooBar"), "FooBar");
		assert_eq!(strip_local_class_prefix("1234"), "1234");
		assert_eq!(strip_local_class_prefix("123Foo"), "Foo");
		assert_eq!(strip_local_class_prefix("123Bar4"), "Bar4");
	}
}


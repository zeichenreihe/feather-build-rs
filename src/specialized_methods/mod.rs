use std::convert::Infallible;
use anyhow::{Context, Result};
use std::io::{Read, Seek};
use std::ops::ControlFlow;
use indexmap::{IndexMap, IndexSet};
use indexmap::map::Entry;
use class_file::tree::class::{ClassAccess, ClassName};
use class_file::tree::descriptor::Type;
use class_file::tree::field::{FieldAccess, FieldDescriptor, FieldName};
use class_file::tree::method::{Method, MethodAccess, MethodDescriptor, MethodName, MethodRef};
use class_file::tree::method::code::Instruction;
use class_file::tree::version::Version;
use class_file::visitor::MultiClassVisitor;
use class_file::visitor::simple::class::SimpleClassVisitor;
use crate::Jar;
use crate::tree::action::remapper::{BRemapper, JarSuperProv};
use crate::tree::mappings::{Mappings, MethodKey, MethodMapping, MethodNowodeMapping};
use crate::tree::names::Names;
use crate::tree::{NodeInfo, ToKey};

/// Stores all known entries
#[derive(Default)]
struct EntryIndex {
	classes: IndexSet<ClassName>,
	methods: IndexMap<MethodRef, MethodAccess>,
}

/// Stores parent and child class information
#[derive(Default)]
struct InheritanceIndex {
	parents: IndexMap<ClassName, IndexSet<ClassName>>,
	children: IndexMap<ClassName, IndexSet<ClassName>>,
}

impl InheritanceIndex {
	fn store(&mut self, name: &ClassName, super_class: Option<ClassName>, interfaces: Vec<ClassName>) {
		if let Some(super_class) = super_class {
			if &super_class != ClassName::JAVA_LANG_OBJECT {
				self.parents.entry(name.clone()).or_default().insert(super_class.clone());
				self.children.entry(super_class).or_default().insert(name.clone());
			}
		}

		for interface in interfaces {
			self.parents.entry(name.clone()).or_default().insert(interface.clone());
			self.children.entry(interface).or_default().insert(name.clone());
		}
	}

	fn get_ancestors(&self, class: &ClassName) -> Vec<&ClassName> {
		let mut ancestors = Vec::new();
		let mut queue = vec![class];
		while let Some(ancestor) = queue.pop() {
			if let Some(parents) = self.parents.get(ancestor) {
				for parent in parents {
					queue.push(parent);
					ancestors.push(parent);
				}
			}
		}
		ancestors
	}

	fn get_descendants(&self, class: &ClassName) -> Vec<&ClassName> {
		let mut descendants = Vec::new();
		let mut queue = vec![class];
		while let Some(descendant) = queue.pop() {
			if let Some(children) = self.children.get(descendant) {
				for child in children {
					queue.push(child);
					descendants.push(child);
				}
			}
		}
		descendants
	}
}

/// Stores what method referees to what methods, by calling them
#[derive(Default)]
struct ReferenceIndex {
	method_references: IndexMap<MethodRef, IndexSet<MethodRef>>,
}

#[derive(Debug, Clone)]
pub(crate) struct SpecializedMethods {
	bridge_to_specialized: IndexMap<MethodRef, MethodRef>,
	specialized_to_bridge: IndexMap<MethodRef, MethodRef>,
}

impl SpecializedMethods {
	fn remap(self, remapper: &impl BRemapper) -> Result<SpecializedMethods> {
		fn remap_method_ref(remapper: &impl BRemapper, method_ref: MethodRef) -> Result<MethodRef> {
			let (class_name, method_key) = ref_to_key_both(method_ref);

			let method_key_1 = remapper.map_method_fail(&class_name, &method_key)?.unwrap_or_else(|| {
				//eprintln!("cannot remap method {class_name:?} {method_key:?}");
				method_key.clone()
			});
			let class_name = remapper.map_class_fail(&class_name)?.unwrap_or_else(|| {
				//eprintln!("cannot remap class {class_name:?}");
				class_name.clone()
			});

			Ok(key_to_ref(class_name, method_key_1))

			// TODO: we include wrongly remapped methods here...
		}

		Ok(SpecializedMethods {
			bridge_to_specialized: self.bridge_to_specialized.into_iter()
				.map(|(bridge, specialized)| Ok((remap_method_ref(remapper, bridge)?, remap_method_ref(remapper, specialized)?)))
				.collect::<Result<_>>()?,
			specialized_to_bridge: self.specialized_to_bridge.into_iter()
				.map(|(specialized, bridge)| Ok((remap_method_ref(remapper, specialized)?, remap_method_ref(remapper, bridge)?)))
				.collect::<Result<_>>()?,
		})
	}
}

#[derive(Default)]
struct MultiClassVisitorImpl {
	entry: EntryIndex,
	inheritance: InheritanceIndex,
	reference: ReferenceIndex,
}

impl MultiClassVisitor for MultiClassVisitorImpl {
	type ClassVisitor = ClassVisitorImpl;
	type ClassResidual = ();

	fn visit_class(mut self, _version: Version, _access: ClassAccess, name: ClassName, super_class: Option<ClassName>, interfaces: Vec<ClassName>)
				   -> Result<ControlFlow<Self, (Self::ClassResidual, Self::ClassVisitor)>> {
		self.entry.classes.insert(name.clone());

		self.inheritance.store(&name, super_class, interfaces);

		Ok(ControlFlow::Continue(((), ClassVisitorImpl {
			name,
			visitor: self,
		})))
	}

	fn finish_class(_this: Self::ClassResidual, class_visitor: Self::ClassVisitor) -> Result<Self> {
		Ok(class_visitor.visitor)
	}
}

impl MultiClassVisitorImpl {
	fn get_specialized_methods(self) -> Result<SpecializedMethods> {
		fn are_types_bridge_compatible(visitor: &MultiClassVisitorImpl, bridge_desc: &Type, specialized_desc: &Type) -> bool {
			match (bridge_desc, specialized_desc) {
				(a, b) if a == b => true,
				(Type::Object(bridge), Type::Object(_)) if bridge == ClassName::JAVA_LANG_OBJECT => true,
				(Type::Object(bridge), Type::Object(_)) if !visitor.entry.classes.contains(bridge) => true,
				(Type::Object(bridge), Type::Object(specialized)) => {
					visitor.inheritance.get_ancestors(specialized).into_iter()
						.any(|ancestor| {
							bridge == ancestor || !visitor.entry.classes.contains(ancestor)
						})
				},
				_=> false
			}
		}

		fn is_potential_bridge(visitor: &MultiClassVisitorImpl, synthetic_method: &MethodRef, access: &MethodAccess, specialized_method: &MethodRef) -> Result<bool> {
			// Bridge methods only exist for inheritance purposes, if we're private, final, or static, we cannot be inherited
			if access.is_private || access.is_final || access.is_static {
				return Ok(false);
			}

			let synthetic_desc = synthetic_method.desc.parse()?;
			let specialized_desc = specialized_method.desc.parse()?;

			// A bridge method will always have the same number of arguments
			if synthetic_desc.parameter_descriptors.len() != specialized_desc.parameter_descriptors.len() {
				return Ok(false);
			}

			for i in 0..synthetic_desc.parameter_descriptors.len() {
				if !are_types_bridge_compatible(visitor, &synthetic_desc.parameter_descriptors[i], &specialized_desc.parameter_descriptors[i]) {
					return Ok(false);
				}
			}

			// Check that the return type is bridge-compatible
			Ok(match (synthetic_desc.return_descriptor, specialized_desc.return_descriptor) {
				(Some(bridge), Some(specialized)) => are_types_bridge_compatible(visitor, &bridge, &specialized),
				(None, None) => true,
				_ => false,
			})
		}

		fn get_higher_method(inheritance: &InheritanceIndex, bridge_1: &MethodRef, bridge_2: &MethodRef) -> MethodRef {
			if inheritance.get_descendants(&bridge_1.class).contains(&&bridge_2.class) {
				bridge_1.clone()
			} else {
				bridge_2.clone()
			}
		}

		//TODO:
		// create a complete test by hand, i.e.
		// - put in a mapping + some "common" bridge methods in two classes
		// - then let it run and look how the mappings should look like

		let mut bridge_to_specialized = IndexMap::new();
		let mut specialized_to_bridge = IndexMap::new();

		for (bridge, specialized) in self.entry.methods.iter()
			.filter(|(_, access)| access.is_synthetic)
			.filter_map(|(synthetic_method, synthetic_access)| {
				self.reference.method_references.get(synthetic_method)
					.and_then(|x| if x.len() == 1 { x.into_iter().next().cloned() } else { None })
					.map(|specialized_method| (synthetic_method, synthetic_access, specialized_method))
			})
			.filter(|(synthetic_method, synthetic_access, specialized_method)| {
				synthetic_access.is_bridge || is_potential_bridge(&self, synthetic_method, synthetic_access, specialized_method).unwrap_or(false)
			})
			.map(|(bridge, _, specialized)| (bridge, specialized))
		{
			if let Some(other_bridge) = specialized_to_bridge.get(&specialized) {
				// we already have a bridge for this method, so we keep the one higher in the hierarchy
				// can happen with a class inheriting from a superclass with one or more bridge method(s)

				let higher_bridge = get_higher_method(&self.inheritance, bridge, other_bridge);
				specialized_to_bridge.insert(specialized.clone(), higher_bridge);
			} else {
				specialized_to_bridge.insert(specialized.clone(), bridge.clone());
			}

			bridge_to_specialized.insert(bridge.clone(), specialized);
		}

		/*
		// TODO: for now we don't put in the "strange" reverses: (note there's also a todo on a test below)
		// Imagine a bridge method `setData(Ljava/lang/Object;)V` and the specialized being
		// `subNodeFunction(Ljava/lang/Integer;)V`. This would add `setData(Ljava/lang/Integer;)V`
		// as a specialized method to only the specialized->bridge mappings.
		let x = specialized_to_bridge.clone();
		for (specialized, bridge) in x {
			if specialized.name != bridge.name {
				eprintln!("Strange specialized/bridge methods: {specialized:?} and {bridge:?}");
				let renamed_specialized = MethodRef {
					class: specialized.class.clone(),
					name: bridge.name.clone(),
					desc: specialized.desc.clone(),
				};
				specialized_to_bridge.insert(
					renamed_specialized,
					specialized_to_bridge.get(&specialized).context("shouldn't fail, TODO: upstream doesn't have any null checks")?.clone()
				);
			}
		}
		 */

		Ok(SpecializedMethods { bridge_to_specialized, specialized_to_bridge })
	}
}

struct ClassVisitorImpl {
	name: ClassName,
	visitor: MultiClassVisitorImpl,
}

impl SimpleClassVisitor for ClassVisitorImpl {
	type FieldVisitor = Infallible;
	type MethodVisitor = Method;

	fn visit_field(&mut self, _access: FieldAccess, _name: FieldName, _descriptor: FieldDescriptor) -> Result<Option<Self::FieldVisitor>> {
		Ok(None)
	}
	fn finish_field(&mut self, _field_visitor: Self::FieldVisitor) -> Result<()> {
		Ok(())
	}

	fn visit_method(&mut self, access: MethodAccess, name: MethodName, descriptor: MethodDescriptor) -> Result<Option<Self::MethodVisitor>> {
		Ok(Some(Method::new(access, name, descriptor)))
	}

	fn finish_method(&mut self, method_visitor: Self::MethodVisitor) -> Result<()> {
		let method_ref = MethodRef {
			class: self.name.clone(),
			name: method_visitor.name,
			desc: method_visitor.descriptor,
		};

		self.visitor.entry.methods.insert(method_ref.clone(), method_visitor.access);

		if let Some(code) = method_visitor.code {
			let references = code.instructions.into_iter()
				.filter_map(|instruction| match instruction.instruction {
					Instruction::InvokeVirtual(method_ref) |
					Instruction::InvokeSpecial(method_ref, _) |
					Instruction::InvokeStatic(method_ref, _) |
					Instruction::InvokeInterface(method_ref) => Some(method_ref),
					// I think InvokeDynamic can't appear in bridge methods
					// TODO: it might can... seems like java 8 guava has quite a bunch of invokedynamic use
					_ => None,
				});
			self.visitor.reference.method_references.entry(method_ref)
				.or_default()
				.extend(references);
		}

		Ok(())
	}
}

fn key_to_ref(class_name: ClassName, method_key: MethodKey) -> MethodRef {
	MethodRef { class: class_name, name: method_key.name, desc: method_key.desc }
}

fn ref_to_key_both(method_ref: MethodRef) -> (ClassName, MethodKey) {
	(method_ref.class, MethodKey { name: method_ref.name, desc: method_ref.desc })
}

impl Jar {
	pub(crate) fn add_specialized_methods_to_mappings(
		main_jar: &Jar, // official
		calamus: &Mappings<2>, // official -> intermediary
		libraries: &[Jar], // official
		mappings: &Mappings<2> // intermediary -> named
	) -> Result<Mappings<2>> {
		let mut super_classes_provider = vec![main_jar.get_super_classes_provider()?];
		for library in libraries {
			super_classes_provider.push(library.get_super_classes_provider()?);
		}

		let remapper_calamus = calamus.remapper_b(
			calamus.get_namespace("official")?,
			calamus.get_namespace("intermediary")?,
			&super_classes_provider
		)?;
		let x = JarSuperProv::remap(&remapper_calamus, &super_classes_provider)?;
		let remapper_named = mappings.remapper_b(
			mappings.get_namespace("calamus")?,
			mappings.get_namespace("named")?,
			&x
		)?;

		let specialized_methods =
			main_jar.get_specialized_methods()?; // official
		//dbg!(specialized_methods.bridge_to_specialized.iter().filter(|x| x.0.class == "aaf").collect::<IndexMap<_, _>>());
		dbg!(specialized_methods.bridge_to_specialized.iter().filter(|x| x.0.class == "bka").collect::<IndexMap<_, _>>());
		let specialized_methods =
			specialized_methods.remap(&remapper_calamus)?; // intermediary
		//dbg!(specialized_methods.bridge_to_specialized.iter().filter(|x| x.0.class == "aso$1" || x.0.class == "net/minecraft/unmapped/C_1184371$1").collect::<IndexMap<_, _>>());
		dbg!(specialized_methods.bridge_to_specialized.iter().filter(|x| x.0.class == "bka" || x.0.class == "net/minecraft/unmapped/C_6254461").collect::<IndexMap<_, _>>());
		//panic!();

		//let x: IndexMap<_, _> = specialized_methods.bridge_to_specialized.iter()
		//	.filter(|(a, b)| a.name.as_str().len() < 3 || b.name.as_str().len() < 3)
		//	.collect();
		//dbg!(x);
		//panic!();


		//mappings.classes.iter().filter(|(k, _)| k.src == "aso$1" || k.src == "net/minecraft/unmapped/C_1184371$1").map(|x| dbg!(x)).for_each(|_| {});

/*
		let mappings = mappings.clone(); // "calamus", "named"
		let second = mappings.get_namespace("named")?;
		let mut mappings = Mappings {
			info: mappings.info,
			classes: mappings.classes.into_iter()
				.map(|(class_key, class)| Ok((class_key.clone(), ClassNowodeMapping {
					info: class.info,
					fields: class.fields,
					methods: {
						let mut index_map = IndexMap::new();

						for (method_key, method) in class.methods {
							let method_ref = key_to_ref(class_key.clone(), method_key);
							// If the names in the `named` namespace of the specialized and corresponding bridge method don't match,
							// set the name of the specialized method to the name of the bridge method.
							if let Some(bridge) = specialized_methods.specialized_to_bridge.get(&method_ref) {
								let specialized = method_ref;

								let name_bridge = {
									let (class_key, method_key) = ref_to_key_both(bridge.clone());
									remapper_named.map_method(&class_key, &method_key)?.src
								};
								let name_specialized = {
									let (class_key, method_key) = ref_to_key_both(specialized.clone());
									remapper_named.map_method(&class_key, &method_key)?.src
								};

								if name_specialized != name_bridge {
									//eprintln!("nonmatching names: bridge: {:?} and specialized: {:?}", name_bridge, name_specialized);

									// replace the name with the one from the specialized method
									let mut x = method.info.names.clone();
									if let Some(y) = x.get_mut(second).as_mut() {
										*y = name_bridge;
									}
									//method.info.names = x;
									//println!("from: {:?} to {:?}", method.info.names, x);
								}

								index_map.insert(method.info.get_key(), method);
							} else {
								index_map.insert(method.info.get_key(), method);
							}
						}

						index_map
					},
					javadoc: class.javadoc,
				})))
				.collect::<Result<_>>()?,
			javadoc: mappings.javadoc,
		};
 */
		// TODO: it seems like this doesn't make a difference...
		// (using this instead of the large commented out block above
		let mut mappings = mappings.clone();

		for (bridge, specialized) in specialized_methods.bridge_to_specialized {
			let named_specialized = {
				let (class_key, method_key) = ref_to_key_both(bridge.clone());
				let result = remapper_named.map_method(&class_key, &method_key)?;
				key_to_ref(class_key, result).name
			};

			let info = MethodMapping {
				names: Names::from([specialized.name, named_specialized]),
				desc: specialized.desc,
			};

			if let Some(class) = mappings.classes.get_mut(&bridge.class) {
				match class.methods.entry(info.get_key()) {
					Entry::Occupied(mut e) => {
						if e.get().info != info {
							//eprintln!("replaced old mapping: {:#?} with different {info:?}", e.get());
						} else {
						}

						// only replace the info, not the rest
						e.get_mut().info = info;
					},
					Entry::Vacant(e) => {
						e.insert(MethodNowodeMapping::new(info));
					},
				}
			} else {
				if bridge.class == "net/minecraft/unmapped/C_6254461" {
					dbg!("no class");
					// note: this can be solved by having more complete mappings...
					// note: blindly putting in a class mapping also doesn't work, this would add mappings that are not existant in the other mapping
					//       we merge later
				}
			}
		}

		//panic!();

		//mappings.classes.iter().filter(|(k, _)| k.src == "aso$1" || k.src == "net/minecraft/unmapped/C_1184371$1").map(|x| dbg!(x)).for_each(|_| {});
		//panic!();

		Ok(mappings)
	}

	pub(crate) fn get_specialized_methods(&self) -> Result<SpecializedMethods> {
		let visitor = MultiClassVisitorImpl::default();

		let visitor = self.read_into(visitor)?;

		visitor.get_specialized_methods()
	}
}

#[cfg(test)]
mod testing {
	use anyhow::Result;
	use std::io::Cursor;
	use indexmap::IndexMap;
	use class_file::tree::method::MethodRef;
	use raw_class_file::{AttributeInfo, ClassFile, CpInfo, FieldInfo, flags, insn, MethodInfo};
	use crate::specialized_methods::MultiClassVisitorImpl;

	#[test]
	fn class_files() -> Result<()> {
		let visitor = MultiClassVisitorImpl::default();

		let bytes = include_bytes!("test/MyNode.class");
		let mut cursor = Cursor::new(bytes);
		let visitor = class_file::read_class_multi(&mut cursor, visitor)?;

		let bytes = include_bytes!("test/Node.class");
		let mut cursor = Cursor::new(bytes);
		let visitor = class_file::read_class_multi(&mut cursor, visitor)?;

		let bytes = include_bytes!("test/SpecializedMethods.class");
		let mut cursor = Cursor::new(bytes);
		let visitor = class_file::read_class_multi(&mut cursor, visitor)?;


		let specialized_methods = visitor.get_specialized_methods()?;

		assert_eq!(
			specialized_methods.bridge_to_specialized,
			IndexMap::from([
				(MethodRef {
					class: "MyNode".into(),
					name: "setData".into(),
					desc: "(Ljava/lang/Object;)V".into(),
				}, MethodRef {
					class: "MyNode".into(),
					name: "setData".into(),
					desc: "(Ljava/lang/Integer;)V".into(),
				}),
			])
		);
		assert_eq!(
			specialized_methods.specialized_to_bridge,
			IndexMap::from([
				(MethodRef {
					class: "MyNode".into(),
					name: "setData".into(),
					desc: "(Ljava/lang/Integer;)V".into(),
				}, MethodRef {
					class: "MyNode".into(),
					name: "setData".into(),
					desc: "(Ljava/lang/Object;)V".into(),
				}),
			])
		);

		Ok(())
	}
}
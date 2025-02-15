use std::convert::Infallible;
use anyhow::Result;
use std::ops::ControlFlow;
use indexmap::{IndexMap, IndexSet};
use indexmap::map::Entry;
use duke::tree::class::{ClassAccess, ObjClassName, ObjClassNameSlice};
use duke::tree::descriptor::Type;
use duke::tree::field::{FieldAccess, FieldDescriptor, FieldName};
use duke::tree::method::{Method, MethodAccess, MethodDescriptor, MethodName, MethodRef, MethodRefObj};
use duke::tree::method::code::Instruction;
use duke::tree::version::Version;
use duke::visitor::MultiClassVisitor;
use duke::visitor::simple::class::SimpleClassVisitor;
use dukebox::storage::{Jar, OpenedJar};
use quill::remapper::{BRemapper, JarSuperProv};
use quill::tree::mappings::{Mappings, MethodMapping, MethodNowodeMapping};
use quill::tree::{NodeInfo, ToKey};
use crate::{Intermediary, Named, Official};

/// Stores all known entries
#[derive(Default)]
struct EntryIndex {
	classes: IndexSet<ObjClassName>,
	methods: IndexMap<MethodRefObj, MethodAccess>,
}

/// Stores parent and child class information
#[derive(Default)]
struct InheritanceIndex {
	parents: IndexMap<ObjClassName, IndexSet<ObjClassName>>,
	children: IndexMap<ObjClassName, IndexSet<ObjClassName>>,
}

impl InheritanceIndex {
	fn store(&mut self, name: &ObjClassName, super_class: Option<ObjClassName>, interfaces: Vec<ObjClassName>) {
		if let Some(super_class) = super_class {
			if super_class != ObjClassName::JAVA_LANG_OBJECT {
				self.parents.entry(name.clone()).or_default().insert(super_class.clone());
				self.children.entry(super_class).or_default().insert(name.clone());
			}
		}

		for interface in interfaces {
			self.parents.entry(name.clone()).or_default().insert(interface.clone());
			self.children.entry(interface).or_default().insert(name.clone());
		}
	}

	fn get_ancestors(&self, class: &ObjClassNameSlice) -> Vec<&ObjClassNameSlice> {
		let mut ancestors = Vec::new();
		let mut queue = vec![class];
		while let Some(ancestor) = queue.pop() {
			if let Some(parents) = self.parents.get(ancestor) {
				for parent in parents {
					queue.push(parent);
					ancestors.push(parent.as_slice());
				}
			}
		}
		ancestors
	}

	fn get_descendants(&self, class: &ObjClassNameSlice) -> Vec<&ObjClassNameSlice> {
		let mut descendants = Vec::new();
		let mut queue = vec![class];
		while let Some(descendant) = queue.pop() {
			if let Some(children) = self.children.get(descendant) {
				for child in children {
					queue.push(child);
					descendants.push(child.as_slice());
				}
			}
		}
		descendants
	}
}

/// Stores what method referees to what methods, by calling them
#[derive(Default)]
struct ReferenceIndex {
	method_references: IndexMap<MethodRefObj, IndexSet<MethodRefObj>>,
}

#[derive(Debug, Clone)]
pub(crate) struct SpecializedMethods {
	pub(crate) bridge_to_specialized: IndexMap<MethodRefObj, MethodRefObj>,
	specialized_to_bridge: IndexMap<MethodRefObj, MethodRefObj>,
}

impl SpecializedMethods {
	pub(crate) fn remap(self, remapper: &impl BRemapper) -> Result<SpecializedMethods> {
		Ok(SpecializedMethods {
			bridge_to_specialized: self.bridge_to_specialized.into_iter()
				.map(|(bridge, specialized)| Ok((
					remapper.map_method_ref_obj(&bridge)?,
					remapper.map_method_ref_obj(&specialized)?)
				))
				.collect::<Result<_>>()?,
			specialized_to_bridge: self.specialized_to_bridge.into_iter()
				.map(|(specialized, bridge)| Ok((
					remapper.map_method_ref_obj(&specialized)?,
					remapper.map_method_ref_obj(&bridge)?)
				))
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

	fn visit_class(mut self, _version: Version, _access: ClassAccess, name: ObjClassName, super_class: Option<ObjClassName>, interfaces: Vec<ObjClassName>)
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
				(Type::Object(bridge), Type::Object(_)) if bridge == ObjClassName::JAVA_LANG_OBJECT => true,
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

		fn is_potential_bridge(visitor: &MultiClassVisitorImpl, synthetic_method: &MethodRefObj, access: &MethodAccess, specialized_method: &MethodRefObj)
				-> Result<bool> {
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

		fn get_higher_method<'a>(inheritance: &InheritanceIndex, bridge_1: &'a MethodRefObj, bridge_2: &'a MethodRefObj) -> &'a MethodRefObj {
			if inheritance.get_descendants(&bridge_1.class).contains(&bridge_2.class.as_slice()) {
				bridge_1
			} else {
				bridge_2
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
			let bridge_to_insert = if let Some(other_bridge) = specialized_to_bridge.get(&specialized) {
				// we already have a bridge for this method, so we keep the one higher in the hierarchy
				// can happen with a class inheriting from a superclass with one or more bridge method(s)

				get_higher_method(&self.inheritance, bridge, other_bridge)
			} else {
				bridge
			};
			specialized_to_bridge.insert(specialized.clone(), bridge_to_insert.clone());

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
	name: ObjClassName,
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
		let method_ref = method_visitor.as_name_and_desc().with_class_obj(self.name.clone());

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
				})
				// Filter out array method references. Array method references can't appear since then this class itself
				// would need to be an array class, which is impossible.
				.filter_map(|MethodRef { class, name, desc }| class.into_obj()
					.map(|class| MethodRefObj { class, name, desc })
				);
			self.visitor.reference.method_references.entry(method_ref)
				.or_default()
				.extend(references);
		}

		Ok(())
	}
}

pub(crate) fn add_specialized_methods_to_mappings(
	main_jar: &impl Jar, // official
	calamus: &Mappings<2, (Official, Intermediary)>, // official -> intermediary
	libraries: &[impl Jar], // official
	mappings: &Mappings<2, (Intermediary, Named)> // intermediary -> named
) -> Result<Mappings<2, (Intermediary, Named)>> {
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
		mappings.get_namespace("intermediary")?,
		mappings.get_namespace("named")?,
		&x
	)?;

	let specialized_methods =
		main_jar.get_specialized_methods()? // official
			.remap(&remapper_calamus)?; // intermediary

	let mut mappings = mappings.clone();

	for (bridge, specialized) in specialized_methods.bridge_to_specialized {
		let named_specialized = remapper_named.map_method_ref_obj(&bridge)?.name;

		let info = MethodMapping {
			names: [specialized.name, named_specialized].into(),
			desc: specialized.desc,
		};

		if let Some(class) = mappings.classes.get_mut(&bridge.class) {
			match class.methods.entry(info.get_key()?) {
				Entry::Occupied(mut e) => {
					// only replace the info, not the rest
					e.get_mut().info = info;
				},
				Entry::Vacant(e) => {
					e.insert(MethodNowodeMapping::new(info));
				},
			}
		}
	}

	Ok(mappings)
}

pub(crate) trait GetSpecializedMethods {
	fn get_specialized_methods(&self) -> Result<SpecializedMethods>;
}

impl<J: Jar> GetSpecializedMethods for J {
	fn get_specialized_methods(&self) -> Result<SpecializedMethods> {
		let visitor = MultiClassVisitorImpl::default();

		let visitor = self.open()?.read_classes_into(visitor)?;

		visitor.get_specialized_methods()
	}
}

#[cfg(test)]
mod testing {
	use anyhow::Result;
	use std::io::Cursor;
	use indexmap::IndexMap;
	use duke::tree::class::ObjClassName;
	use duke::tree::method::{MethodDescriptor, MethodName, MethodRefObj};
	use raw_class_file::{AttributeInfo, ClassFile, CpInfo, FieldInfo, flags, insn, MethodInfo};
	use crate::specialized_methods::MultiClassVisitorImpl;

	#[test]
	fn class_files() -> Result<()> {
		let visitor = MultiClassVisitorImpl::default();

		let bytes = include_bytes!("test/MyNode.class");
		let mut cursor = Cursor::new(bytes);
		let visitor = duke::read_class_multi(&mut cursor, visitor)?;

		let bytes = include_bytes!("test/Node.class");
		let mut cursor = Cursor::new(bytes);
		let visitor = duke::read_class_multi(&mut cursor, visitor)?;

		let bytes = include_bytes!("test/SpecializedMethods.class");
		let mut cursor = Cursor::new(bytes);
		let visitor = duke::read_class_multi(&mut cursor, visitor)?;


		let specialized_methods = visitor.get_specialized_methods()?;

		assert_eq!(
			specialized_methods.bridge_to_specialized,
			IndexMap::from([
				(MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Object;)V".to_owned().into()) },
				}, MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Integer;)V".to_owned().into()) },
				}),
			])
		);
		assert_eq!(
			specialized_methods.specialized_to_bridge,
			IndexMap::from([
				(MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Integer;)V".to_owned().into()) },
				}, MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Object;)V".to_owned().into()) },
				}),
			])
		);

		Ok(())
	}

	/// Tests having a specialized method with a different name than the bridge method.
	/// Classes are written by hand to not worry about javac.
	/// ```txt,ignore
	/// class MyNode extends Node {
	///     public MyNode(java.lang.Integer a) { super(a); }
	///     @Override
	///     public void setData(java.lang.Object a) { // this is the bridge method
	///         this.specialized((java.lang.Integer) a);
	///     }
	///     public synthetic void specialized(java.lang.Integer a) { } // the actual override
	/// }
	/// class Node<T> { // note that we don't construct signature attributes
	///     public T data;
	///     public Node(T data) { super(); }
	///     public void setData(T a) { self.data = a; }
	/// }
	/// ```
	#[test]
	fn class_files_2() -> Result<()> {
		// TODO: use the raw_class_file crate to write these out manually
		// TODO: nah instead commit the .java and .class files...
		let visitor = MultiClassVisitorImpl::default();

		let bytes = ClassFile {
			minor_version: 0,
			major_version: 52,
			constant_pool: vec![
				CpInfo::Utf8 { bytes: b"MyNode".to_vec() }, // 1
				CpInfo::Class { name_index: 1 }, // 2
				CpInfo::Utf8 { bytes: b"Node".to_vec() }, // 3
				CpInfo::Class { name_index: 3 }, // 4
				CpInfo::Utf8 { bytes: b"<init>".to_vec() }, // 5
				CpInfo::Utf8 { bytes: b"(Ljava/lang/Integer;)V".to_vec() }, // 6
				CpInfo::Utf8 { bytes: b"Code".to_vec() }, // 7
				CpInfo::NameAndType { name_index: 5, descriptor_index: 6 }, // 8
				CpInfo::Methodref { class_index: 4, name_and_type_index: 8 }, // 9
				CpInfo::Utf8 { bytes: b"specialized".to_vec() }, // 10
				CpInfo::Utf8 { bytes: b"setData".to_vec() }, // 11
				CpInfo::Utf8 { bytes: b"(Ljava/lang/Object;)V".to_vec() }, // 12
				CpInfo::Utf8 { bytes: b"java/lang/Integer".to_vec() }, // 13
				CpInfo::Class { name_index: 13 }, // 14
				CpInfo::NameAndType { name_index: 10, descriptor_index: 6 }, // 15
				CpInfo::Methodref { class_index: 2, name_and_type_index: 15 }, // 16
			],
			access_flags: flags::ACC_SUPER,
			this_class: 2,
			super_class: 4,
			interfaces: vec![],
			fields: vec![],
			methods: vec![
				MethodInfo {
					access_flags: flags::ACC_PUBLIC,
					name_index: 5,
					descriptor_index: 6,
					attributes: vec![
						AttributeInfo::Code {
							attribute_name_index: 7,
							max_stack: 2,
							max_locals: 2,
							code: vec![
								insn::aload_0,
								insn::aload_1,
								insn::invokespecial, 0, 9,
								insn::r#return,
							],
							exception_table: vec![],
							attributes: vec![],
						},
					],
				},
				MethodInfo {
					access_flags: flags::ACC_PUBLIC,
					name_index: 10,
					descriptor_index: 6,
					attributes: vec![
						AttributeInfo::Code {
							attribute_name_index: 7,
							max_stack: 0,
							max_locals: 2,
							code: vec![
								insn::r#return,
							],
							exception_table: vec![],
							attributes: vec![],
						},
					]
				},
				MethodInfo {
					access_flags: flags::ACC_PUBLIC | flags::ACC_SYNTHETIC,
					name_index: 11,
					descriptor_index: 12,
					attributes: vec![
						AttributeInfo::Code {
							attribute_name_index: 7,
							max_stack: 2,
							max_locals: 2,
							code: vec![
								insn::aload_0,
								insn::aload_1,
								insn::checkcast, 0, 14,
								insn::invokevirtual, 0, 16,
								insn::r#return,
							],
							exception_table: vec![],
							attributes: vec![],
						},
					],
				},
			],
			attributes: vec![],
		}.to_bytes();
		let mut cursor = Cursor::new(bytes);
		let visitor = duke::read_class_multi(&mut cursor, visitor)?;

		let bytes = ClassFile {
			minor_version: 0,
			major_version: 52,
			constant_pool: vec![
				CpInfo::Utf8 { bytes: b"Node".to_vec() }, // 1
				CpInfo::Class { name_index: 1 }, // 2
				CpInfo::Utf8 { bytes: b"java/lang/Object".to_vec() }, // 3
				CpInfo::Class { name_index: 3 }, // 4
				CpInfo::Utf8 { bytes: b"data".to_vec() }, // 5
				CpInfo::Utf8 { bytes: b"Ljava/lang/Object;".to_vec() }, // 6
				CpInfo::Utf8 { bytes: b"<init>".to_vec() }, // 7
				CpInfo::Utf8 { bytes: b"(Ljava/lang/Integer;)V".to_vec() }, // 8
				CpInfo::Utf8 { bytes: b"Code".to_vec() }, // 9
				CpInfo::Utf8 { bytes: b"()V".to_vec() }, // 10
				CpInfo::NameAndType { name_index: 7, descriptor_index: 10 }, // 11
				CpInfo::Methodref { class_index: 4, name_and_type_index: 11 }, // 12
				CpInfo::Utf8 { bytes: b"setData".to_vec() }, // 13
				CpInfo::Utf8 { bytes: b"(Ljava/lang/Object;)V".to_vec() }, // 14
				CpInfo::NameAndType { name_index: 5, descriptor_index: 6 }, // 15
				CpInfo::Fieldref { class_index: 2, name_and_type_index: 15 }, // 16
			],
			access_flags: flags::ACC_SUPER,
			this_class: 2,
			super_class: 4,
			interfaces: vec![],
			fields: vec![
				FieldInfo {
					access_flags: flags::ACC_PUBLIC,
					name_index: 5,
					descriptor_index: 6,
					attributes: vec![], // Signature: TT;
				}
			],
			methods: vec![
				MethodInfo {
					access_flags: flags::ACC_PUBLIC,
					name_index: 7,
					descriptor_index: 8,
					attributes: vec![
						AttributeInfo::Code {
							attribute_name_index: 9,
							max_stack: 1,
							max_locals: 2,
							code: vec![
								insn::aload_0,
								insn::invokespecial, 0, 12,
								insn::r#return,
							],
							exception_table: vec![],
							attributes: vec![],
						},
					], // Signature: (TT;)V
				},
				MethodInfo {
					access_flags: flags::ACC_PUBLIC,
					name_index: 13,
					descriptor_index: 14,
					attributes: vec![
						AttributeInfo::Code {
							attribute_name_index: 9,
							max_stack: 2,
							max_locals: 2,
							code: vec![
								insn::aload_0,
								insn::aload_1,
								insn::putfield, 0, 16,
								insn::r#return,
							],
							exception_table: vec![],
							attributes: vec![],
						},
					], // Signature: (TT;)V
				},
			],
			attributes: vec![], // Signature: <T:Ljava/lang/Object;>Ljava/lang/Object;
		}.to_bytes();
		let mut cursor = Cursor::new(bytes);
		let visitor = duke::read_class_multi(&mut cursor, visitor)?;

		let specialized_methods = visitor.get_specialized_methods()?;

		assert_eq!(
			specialized_methods.bridge_to_specialized,
			IndexMap::from([
				(MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Object;)V".to_owned().into()) },
				}, MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("specialized".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Integer;)V".to_owned().into()) },
				}),
			])
		);
		assert_eq!(
			specialized_methods.specialized_to_bridge,
			IndexMap::from([
				(MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("specialized".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Integer;)V".to_owned().into()) },
				}, MethodRefObj {
					class: unsafe { ObjClassName::from_inner_unchecked("MyNode".to_owned().into()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned().into()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Object;)V".to_owned().into()) },
				}),
				/*
				// TODO: see todo about putting more stuff into that map
				(MethodRef {
					class: unsafe { ClassName::from_inner_unchecked("MyNode".to_owned()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Integer;)V".to_owned()) },
				}, MethodRef {
					class: unsafe { ClassName::from_inner_unchecked("MyNode".to_owned()) },
					name: unsafe { MethodName::from_inner_unchecked("setData".to_owned()) },
					desc: unsafe { MethodDescriptor::from_inner_unchecked("(Ljava/lang/Object;)V".to_owned()) },
				}),
				 */
			])
		);

		Ok(())
	}
}
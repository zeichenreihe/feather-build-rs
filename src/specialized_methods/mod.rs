mod class_file;
use anyhow::Result;
use indexmap::IndexMap;
use indexmap::map::Entry;
use crate::Jar;
use crate::specialized_methods::class_file::{ClassFile, MethodInfo};
use crate::tree::mappings::{ClassKey, MethodKey};

impl MethodInfo {
	fn references(&self, other: &MethodInfo) -> bool {
		self.code_analysis.invoke_interface.iter()
			.chain(self.code_analysis.invoke_static.iter())
			.chain(self.code_analysis.invoke_special.iter())
			.chain(self.code_analysis.invoke_virtual.iter())
			.find(|(_, name, descriptor)| name == &other.name && descriptor == &other.descriptor)
			.is_some()
	}
}

fn find_specialized_method<'a>(class: &'a ClassFile, bridge: &MethodInfo) -> Option<&'a MethodInfo> {
	let methods: Vec<_> = class.methods
		.iter()
		.filter(|x| bridge.references(x))
		.collect();

	if methods.len() != 1 {
		None
	} else {
		Some(methods[0])
	}
}

fn is_potential_bridge(bridge: &MethodInfo, specialized: &MethodInfo) -> bool {
	println!("WARN: found potential non-bridge, but synthetic method: {bridge:#?} with corresponding specialized method {specialized:#?}");
	!specialized.access_flags.is_private &&
		!specialized.access_flags.is_final &&
		!specialized.access_flags.is_static &&
		bridge.descriptor.len() == specialized.descriptor.len() &&
		false /* for every arg in bridge.arg, specialized.arg: bridge_compatible */

	// bridge_compatible:
	//   // a == b => true
	//   // a.is_type && b.is_type
	//       // must be related with a generic somehow!?
	//   // => false
}

#[derive(Debug, Clone)]
pub(crate) struct SpecializedMethods {
	pub(crate) classes: IndexMap<ClassKey, SpecializedMethodsClass>,
}

#[derive(Debug, Clone)]
pub(crate) struct SpecializedMethodsClass {
	/// Map from the bridge methods to the specialized methods they map to
	pub(crate) methods: IndexMap<MethodKey, MethodKey>,
}

fn get_specialized_methods() -> SpecializedMethods {
	todo!()
}

impl Jar {
	pub(crate) fn get_specialized_methods(&self) -> Result<SpecializedMethods> {
		todo!()
	}
}

impl ClassFile {
	fn get_specialized_methods(&self) -> Result<SpecializedMethodsClass> {
		let mut methods = IndexMap::new();

		for bridge in &self.methods {
			if bridge.access_flags.is_synthetic {
				if let Some(specialized) = find_specialized_method(&self, bridge) {
					if bridge.access_flags.is_bridge || is_potential_bridge(bridge, specialized) {
						match methods.entry(bridge.into()) {
							Entry::Occupied(e) => {
								// we already have a bridge for this method, so we keep the one higher in the hierarchy
								// can happen with a class inheriting from a superclass with one or more bridge method(s)
								println!("entry already full: {:?}", e.get());
								todo!()
							},
							Entry::Vacant(e) => {
								e.insert(specialized.into());
							},
						}
					}
				}
			}
		}

		Ok(SpecializedMethodsClass { methods })
	}
}

#[cfg(test)]
mod testing {
	use crate::specialized_methods::class_file::ClassFile;
	use crate::tree::mappings::MethodKey;

	#[test]
	fn class_file() {
		let bytes = include_bytes!("test/MyNode.class");

		let class = ClassFile::parse(&mut bytes.as_slice()).unwrap();

		let s = class.get_specialized_methods().unwrap();

		let vec: Vec<_> = s.methods.into_iter().collect();

		assert_eq!(
			vec,
			vec![(
				MethodKey::new(String::from("(Ljava/lang/Object;)V"), String::from("setData")),
				MethodKey::new(String::from("(Ljava/lang/Integer;)V"), String::from("setData"))
			)]
		);
	}
}
//#![allow(unused)] // TODO: remove

mod class_file;

use std::io::{Bytes, Read};
use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use crate::specialized_methods::class_file::MethodInfo;
use crate::tree::mappings::{ClassKey, MethodKey};

#[derive(Debug, Clone)]
struct SpecializedMethods {
	classes: IndexMap<ClassKey, SpecializedMethodsClass>,
}

#[derive(Debug, Clone)]
struct SpecializedMethodsClass {
	methods: IndexMap<MethodKey, MethodKey>,
}

fn get_specialized_methods() -> SpecializedMethods {
	todo!()
}


fn read_class(mut reader: impl Read) -> Result<()> {
	let class = class_file::ClassFile::parse(&mut reader)?;

	fn find_specialized_method<'a>(methods: &'a Vec<MethodInfo>, bridge: &MethodInfo) -> Option<&'a MethodInfo> {

		for method in methods {
			if method.name == bridge.name {
				return Some(method)
			}
		}
		None

		// (this = bridge)
		// find specialized method as the one that only has one call to this one!
	}

	fn is_potential_bridge(bridge: &MethodInfo, specialized: &MethodInfo) -> bool {
		!specialized.access_flags.is_private &&
			!specialized.access_flags.is_final &&
			!specialized.access_flags.is_static &&
			true /* same number of arguments */ &&
			true /* for every arg in bridge.arg, specialized.arg: bridge_compatible */

		// bridge_compatible:
		//   // a == b => true
		//   // a.is_type && b.is_type
		//       // must be related with a generic somehow!?
		//   // => false
	}

	let methods = &class.methods;

	for bridge in methods {
		if bridge.access_flags.is_synthetic {
			if let Some(specialized) = find_specialized_method(methods, bridge) {
				if bridge.access_flags.is_bridge || is_potential_bridge(bridge, specialized) {

					// store it with uniqueness check (might be relevant in the future)
					// following comment: (for failure of the check)
					// // we already have a bridge for this method, so we keep the one higher in the hierarchy
					// // can happen with a class inheriting from a superclass with one or more bridge method(s)

					println!("{:?}", bridge);
					println!("-> {:?}", specialized);
				}
			}
		}
	}

	Ok(())
}

#[cfg(test)]
mod testing {
	#[test]
	fn class_file() {
		let bytes = include_bytes!("test/MyNode.class");

		crate::specialized_methods::read_class(bytes.as_slice()).unwrap();
	}
}
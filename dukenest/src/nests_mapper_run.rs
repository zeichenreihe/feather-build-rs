use anyhow::Result;
use java_string::JavaString;
use duke::tree::class::ObjClassName;
use quill::remapper::{ARemapper, BRemapper, NoSuperClassProvider};
use quill::tree::mappings::Mappings;
use crate::nest::{Nest, Nests};


// implementation of NestsMapper.run()
pub fn map_nests(mappings: &Mappings<2>, nests: Nests) -> Result<Nests> {
	let remapper = mappings.remapper_b_first_to_second(NoSuperClassProvider::new())?;

	let mut dst = Nests::default();

	for (_, nest) in nests.all {

		let mapped_name = remapper.map_class(&nest.class_name)?;

		let (encl_class_name, inner_name) = if let Some((encl_class_name, inner_name)) = mapped_name.as_inner().rsplit_once("__") {
			// provided mappings already use nesting
			// SAFETY: todo
			(unsafe { ObjClassName::from_inner_unchecked(encl_class_name.to_owned()) }, inner_name.to_owned())
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
						let mut s = JavaString::new();
						s.push_java_str(prefix);
						s.push_java_str(if let Some((_, substring)) = mapped_name.as_inner().rsplit_once('/') {
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

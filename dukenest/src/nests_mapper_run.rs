use anyhow::{anyhow, bail, Context, Result};
use java_string::JavaString;
use duke::tree::class::{ObjClassName, ObjClassNameSlice};
use quill::remapper::{ARemapper, BRemapper, NoSuperClassProvider};
use quill::tree::mappings::Mappings;
use crate::nest::{Nest, Nests};


// implementation of NestsMapper.run()
pub fn map_nests(nests: &Nests, mappings: &Mappings<2>) -> Result<Nests> {
	let remapper = mappings.remapper_b_first_to_second(NoSuperClassProvider::new())?;

	let mut dst = Nests::default();

	for (_, nest) in &nests.all {

		let mapped_name = remapper.map_class(&nest.class_name)?;

		let (encl_class_name, inner_name) =
		if let Some((encl_class_name, inner_name)) = rsplit_underscore(&mapped_name)
				.with_context(|| anyhow!("for nest {:?}", nest.class_name))? {
			// provided mappings already use nesting
			(encl_class_name, inner_name)
		} else {
			let encl_class_name = remapper.map_class(&nest.encl_class_name)?;

			let inner_name = {
				let i = nest.inner_name.as_inner().char_indices()
					.take_while(|(_, ch)| ch.is_ascii_digit())
					.map(|(pos, _)| pos)
					.last()
					.unwrap_or(0);

				if i < nest.inner_name.as_inner().len() {
					// TODO: dangerous direct access to string slice... (before touching this, write a test!)
					// local classes have a number prefix
					let prefix = &nest.inner_name.as_inner()[0..i];
					let simple_name = &nest.inner_name.as_inner()[i..];

					// make sure the class does not have custom inner name
					if nest.class_name.as_inner().ends_with(simple_name) {
						let mut s = JavaString::new();
						s.push_java_str(prefix);
						s.push_java_str(if let Some((_, substring)) = mapped_name.as_inner().rsplit_once('/') {
							substring
						} else {
							mapped_name.as_inner()
						});
						let inner_name = s;
						// TODO: safety
						unsafe { ObjClassName::from_inner_unchecked(inner_name) }
					} else {
						nest.inner_name.clone()
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
						let inner_name = number.to_owned();
						// TODO: safety
						unsafe { ObjClassName::from_inner_unchecked(inner_name) }
					} else {
						// keep the inner name given by the nests file
						nest.inner_name.clone()
					}
				}
			};

			(encl_class_name, inner_name)
		};

		let nest = Nest {
			nest_type: nest.nest_type,
			class_name: mapped_name,
			encl_class_name,
			encl_method: nest.encl_method.as_ref().map(|method_name_and_desc| {
				remapper.map_method_name_and_desc(&nest.encl_class_name, &method_name_and_desc)
			}).transpose()?,
			inner_name,
			inner_access: nest.inner_access,
		};

		dst.add(nest);
	}

	Ok(dst)
}

fn rsplit_underscore(name: &ObjClassNameSlice) -> Result<Option<(&ObjClassNameSlice, &ObjClassNameSlice)>> {
	if let Some((encl_class_name, inner_name)) = name.as_inner().rsplit_once("__") {
		if encl_class_name.ends_with('/') {
			bail!("cannot split class name {name:?} into enclosing and inner name (at last `__`): invalid enclosing name: {encl_class_name:?}");
		}
		// SAFETY: Since it came from a valid class name, it won't contain illegal characters. We only have to check that
		// each segment is nonempty, and that could only happen for the last segment. This would be the case iff it would
		// end with `/`, which we checked above. Therefore it's a valid object class name.
		let encl_class_name = unsafe { ObjClassNameSlice::from_inner_unchecked(encl_class_name) };

		if inner_name.starts_with('/') {
			bail!("cannot split class name {name:?} into enclosing and inner name (at last `__`): invalid inner name: {inner_name:?}");
		}
		// SAFETY: Since it came from a valid class name, it won't contain illegal characters. We only have to check that
		// each segment is nonempty, and that could only happen for the first segment. This would be the case iff it would
		// start with `/`, which we checked above. Therefore it's a valid object class name.
		let inner_name = unsafe { ObjClassNameSlice::from_inner_unchecked(inner_name) };

		Ok(Some((encl_class_name, inner_name)))
	} else {
		Ok(None)
	}
}

use anyhow::{anyhow, bail, Context, Result};
use java_string::{JavaCodePoint, JavaString};
use duke::tree::class::{ObjClassName, ObjClassNameSlice};
use quill::remapper::{ARemapper, BRemapper, NoSuperClassProvider};
use quill::tree::mappings::Mappings;
use crate::nest::{Nest, Nests};


// implementation of NestsMapper.run()
pub fn map_nests<A, B>(nests: &Nests<A>, mappings: &Mappings<2, (A, B)>) -> Result<Nests<B>> {
	let remapper = mappings.remapper_b_first_to_second(NoSuperClassProvider::new())?;

	let mut dst = Nests::default();

	for (_, nest) in &nests.all {

		let mapped_name = remapper.map_class(&nest.class_name)?;

		let (encl_class_name, inner_name) = rsplit_underscore(&mapped_name)
			.with_context(|| anyhow!("for nest {:?}", nest.class_name))?
			.map_or_else(
				|| -> Result<_> {
					let encl_class_name = remapper.map_class(&nest.encl_class_name)?;

					let inner_name = inner_name(&nest.class_name, &nest.inner_name, &mapped_name)?;

					Ok((encl_class_name, inner_name))
				},
				|(encl_name, inner_name)| {
					// provided mappings already use nesting
					Ok((encl_name.to_owned(), inner_name.to_owned()))
				}
			).with_context(|| anyhow!("for nest {:?}", nest.class_name))?;

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

#[derive(Debug, PartialEq)]
enum NestTypeA<'a> {
	Anonymous(&'a ObjClassNameSlice),
	Inner(&'a ObjClassNameSlice),
	Local(&'a ObjClassNameSlice, &'a ObjClassNameSlice),
}

impl<'a> NestTypeA<'a> {
	fn new(inner_name: &'a ObjClassNameSlice) -> NestTypeA<'a> {
		if let Some((pos, _)) = inner_name.as_inner().char_indices().skip_while(|(_, ch)| ch.is_ascii_digit()).next() {
			if pos == 0 { // no numbers prefix
				NestTypeA::Inner(inner_name)
			} else {
				let (prefix, simple) = inner_name.as_inner().split_at(pos);

				// TODO: could potentially contain a /!
				let prefix = unsafe { ObjClassNameSlice::from_inner_unchecked(prefix) };
				let simple = unsafe { ObjClassNameSlice::from_inner_unchecked(simple) };

				NestTypeA::Local(prefix, simple)
			}
		} else { // there was no non-digit char, so only digits
			NestTypeA::Anonymous(inner_name)
		}
	}
}

fn inner_name(nest_class_name: &ObjClassNameSlice, nest_inner_name: &ObjClassNameSlice, mapped_name: &ObjClassNameSlice) -> Result<ObjClassName> {
	match NestTypeA::new(nest_inner_name) {
		NestTypeA::Anonymous(number) => {
			if let Some(number) = mapped_name.get_simple_name().as_inner().strip_prefix("C_") {
				// mapped name is Calamus intermediary format C_<number>
				// strip the C_ prefix and keep the number as the inner name
				construct_inner_name_from_anonymous_number(number.to_owned())
			} else {
				// keep the inner name given by the nests file
				Ok(number.to_owned())
			}
		},
		NestTypeA::Inner(inner_name) => {
			// make sure the class does not have custom inner name
			if nest_class_name.as_inner().ends_with(inner_name.as_inner()) {
				let mapped_simple = mapped_name.get_simple_name();

				Ok(mapped_simple.to_owned())
			} else {
				Ok(inner_name.to_owned())
			}
		},
		NestTypeA::Local(prefix, simple_name) => {
			// local classes have a number prefix

			// make sure the class does not have custom inner name
			if nest_class_name.as_inner().ends_with(simple_name.as_inner()) {
				let mapped_simple = mapped_name.get_simple_name();

				let mut s = JavaString::new();
				s.push_java_str(prefix.as_inner());
				s.push_java_str(mapped_simple.as_inner());
				let inner_name = s;
				// TODO: safety
				Ok(unsafe { ObjClassName::from_inner_unchecked(inner_name) })
			} else {
				Ok(nest_inner_name.to_owned())
			}
		},
	}
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

fn construct_inner_name_from_anonymous_number(number: JavaString) -> Result<ObjClassName> {
	if number.chars().all(JavaCodePoint::is_ascii_digit) {
		// SAFETY: A name only consisting out of numbers is not a valid class name in the Java Language, but the
		// Java Virtual Machine Specification allows it.
		Ok(unsafe { ObjClassName::from_inner_unchecked(number) })
	} else {
		bail!("inner class name {number:?} contained [^0-9]");
	}
}


#[cfg(test)]
mod testing {
	use anyhow::Result;
	use duke_macros::obj_class_name;
	use crate::nests_mapper_run::{inner_name, NestTypeA};

	#[test]
	fn test_new_nest_type_a() {
		let s = NestTypeA::new;

		assert_eq!(NestTypeA::Inner(obj_class_name!("InnerName")), s(obj_class_name!("InnerName")));
		assert_eq!(NestTypeA::Local(obj_class_name!("123"), obj_class_name!("Local")), s(obj_class_name!("123Local")));
		assert_eq!(NestTypeA::Anonymous(obj_class_name!("12345")), s(obj_class_name!("12345")));

		assert_eq!(NestTypeA::Inner(obj_class_name!("I")), s(obj_class_name!("I")));
		assert_eq!(NestTypeA::Local(obj_class_name!("1"), obj_class_name!("L")), s(obj_class_name!("1L")));
		assert_eq!(NestTypeA::Anonymous(obj_class_name!("12")), s(obj_class_name!("12")));
	}

	#[test]
	fn test_inner_name() -> Result<()> {
		assert_eq!(obj_class_name!("Normal"), inner_name(obj_class_name!("Foo"), obj_class_name!("Normal"), obj_class_name!("MAPPED"))?);
		assert_eq!(obj_class_name!("MAPPED"), inner_name(obj_class_name!("Foo$Normal"), obj_class_name!("Normal"), obj_class_name!("MAPPED"))?);
		assert_eq!(obj_class_name!("123Local"), inner_name(obj_class_name!("Foo"), obj_class_name!("123Local"), obj_class_name!("MAPPED"))?);
		assert_eq!(obj_class_name!("123MAPPED"), inner_name(obj_class_name!("Foo$Local"), obj_class_name!("123Local"), obj_class_name!("MAPPED"))?);
		assert_eq!(obj_class_name!("12345"), inner_name(obj_class_name!("Foo"), obj_class_name!("12345"), obj_class_name!("MAPPED"))?);
		assert_eq!(obj_class_name!("12345"), inner_name(obj_class_name!("Foo$Inner"), obj_class_name!("12345"), obj_class_name!("MAPPED"))?);

		assert_eq!(obj_class_name!("Normal"), inner_name(obj_class_name!("Foo"), obj_class_name!("Normal"), obj_class_name!("MAPPED/C_9876"))?);
		assert_eq!(obj_class_name!("C_9876"), inner_name(obj_class_name!("Foo$Normal"), obj_class_name!("Normal"), obj_class_name!("MAPPED/C_9876"))?);
		assert_eq!(obj_class_name!("123Local"), inner_name(obj_class_name!("Foo"), obj_class_name!("123Local"), obj_class_name!("MAPPED/C_9876"))?);
		assert_eq!(obj_class_name!("123C_9876"), inner_name(obj_class_name!("Foo$Local"), obj_class_name!("123Local"), obj_class_name!("MAPPED/C_9876"))?);
		assert_eq!(obj_class_name!("9876"), inner_name(obj_class_name!("Foo"), obj_class_name!("12345"), obj_class_name!("MAPPED/C_9876"))?);
		assert_eq!(obj_class_name!("9876"), inner_name(obj_class_name!("Foo$Inner"), obj_class_name!("12345"), obj_class_name!("MAPPED/C_9876"))?);

		Ok(())
	}
}


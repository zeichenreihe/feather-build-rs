use anyhow::Result;
use indexmap::IndexMap;
use duke::tree::class::{ObjClassName, ObjClassNameSlice};
use crate::nest::Nests;
use quill::remapper::ARemapper;
use quill::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, map_with_key_from_result_iter, Mappings, MethodMapping, MethodNowodeMapping};

pub(crate) fn apply_nests_to_mappings<A, B>(mappings: Mappings<2, (A, B)>, nests: &Nests<A>) -> Result<Mappings<2, (A, B)>> {
	let mapped_nests = crate::nests_mapper_run::map_nests(nests, &mappings)?;

	let translator = MyRemapper::new(nests, true);
	let mapped_translator = MyRemapper::new(&mapped_nests, true);

	Ok(Mappings {
		info: mappings.info,
		classes: map_with_key_from_result_iter(mappings.classes.into_iter()
			.map(|(class_key, class)| {
				Ok(ClassNowodeMapping {
					info: ClassMapping {
						names: {
							let src = class_key;
							let [_, dst] = class.info.names.into();
							let dst = dst.unwrap(); // TODO: unwrap

							let src = translator.map_class(&src)?;
							let dst = mapped_translator.map_class(&dst)?;

							[src, dst].into()
						},
					},
					fields: map_with_key_from_result_iter(class.fields.into_values()
						.map(|field| Ok(FieldNowodeMapping {
							info: FieldMapping {
								desc: translator.map_field_desc(&field.info.desc)?,
								names: field.info.names,
							},
							javadoc: field.javadoc,
						}))
					)?,
					methods: map_with_key_from_result_iter(class.methods.into_values()
						.map(|method| Ok(MethodNowodeMapping {
							info: MethodMapping {
								desc: translator.map_method_desc(&method.info.desc)?,
								names: method.info.names,
							},
							parameters: method.parameters,
							javadoc: method.javadoc,
						}))
					)?,
					javadoc: class.javadoc,
				})
			})
		)?,
		javadoc: mappings.javadoc,
	})
}
pub(crate) fn undo_nests_to_mappings<A, B>(mappings: Mappings<2, (A, B)>, nests: &Nests<A>) -> Result<Mappings<2, (A, B)>> {
	let mapped_nests = Nests::<B>::default();

	let translator = MyRemapper::new(nests, false);
	let mapped_translator = MyRemapper::new(&mapped_nests, false);

	Ok(Mappings {
		info: mappings.info,
		classes: map_with_key_from_result_iter(mappings.classes.into_iter()
			.map(|(class_key, class)| {
				Ok(ClassNowodeMapping {
					info: ClassMapping {
						names: {
							let src = class_key;
							let [_, dst] = class.info.names.into();
							let dst = dst.unwrap(); // TODO: unwrap

							let src = translator.map_class(&src)?;
							let dst = mapped_translator.map_class(&dst)?;

							// we get the nests by a class un-nested names
							let dst = if nests.all.contains_key(&dst) {
								replace_double_underscore_with_dollar(&dst)
							} else {
								dst
							};

							[src, dst].into()
						},
					},
					fields: map_with_key_from_result_iter(class.fields.into_values()
						.map(|field| Ok(FieldNowodeMapping {
							info: FieldMapping {
								desc: translator.map_field_desc(&field.info.desc)?,
								names: field.info.names,
							},
							javadoc: field.javadoc,
						}))
					)?,
					methods: map_with_key_from_result_iter(class.methods.into_values()
						.map(|method| Ok(MethodNowodeMapping {
							info: MethodMapping {
								desc: translator.map_method_desc(&method.info.desc)?,
								names: method.info.names,
							},
							parameters: method.parameters,
							javadoc: method.javadoc,
						}))
					)?,
					javadoc: class.javadoc,
				})
			})
		)?,
		javadoc: mappings.javadoc,
	})
}


struct MyRemapper(IndexMap<ObjClassName, ObjClassName>);

impl MyRemapper {
	fn new<A>(nests: &Nests<A>, apply: bool) -> Self {
		let map = nests.all.iter()
			.map(|(class_name, nest)| {
				fn build_translation<A>(nests: &Nests<A>, class_name: &ObjClassName) -> ObjClassName {
					if let Some(nest) = nests.all.get(class_name) {
						let a = build_translation(nests, &nest.encl_class_name);
						ObjClassName::from_inner_class(a, &nest.inner_name)
					} else {
						class_name.to_owned()
					}
				}

				let a = build_translation(nests, &nest.encl_class_name);
				let value = ObjClassName::from_inner_class(a, &nest.inner_name);

				(class_name.to_owned(), value)
			})
			.map(|(k, v)| if apply { (k, v) } else { (v, k) } )
			.collect();
		MyRemapper(map)
	}
}

impl ARemapper for MyRemapper {
	fn map_class_fail(&self, class: &ObjClassNameSlice) -> Result<Option<ObjClassName>> {
		Ok(self.0.get(class).cloned())
	}
}

fn replace_double_underscore_with_dollar(name: &ObjClassNameSlice) -> ObjClassName {
	let replaced = name.as_inner().replace('$', "__");
	// SAFETY: Replacing all `$` with `__` doesn't make any valid object class name invalid.
	unsafe { ObjClassName::from_inner_unchecked(replaced) }
}


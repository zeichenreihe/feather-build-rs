use anyhow::Result;
use indexmap::IndexMap;
use duke::tree::class::{ObjClassName, ObjClassNameSlice};
use crate::nest::Nests;
use quill::remapper::ARemapper;
use quill::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, map_with_key_from_result_iter, Mappings, MethodMapping, MethodNowodeMapping};

pub fn nester_run(mappings: Mappings<2>, nests: &Nests, apply: bool) -> Result<Mappings<2>> {
	let mapped_nests = if apply {
		crate::nests_mapper_run::map_nests(&mappings, nests.clone() /* todo: remove this clone? */)?
	} else {
		Nests::default()
	};

	struct MyRemapper(IndexMap<ObjClassName, ObjClassName>);
	impl ARemapper for MyRemapper {
		fn map_class_fail(&self, class: &ObjClassNameSlice) -> Result<Option<ObjClassName>> {
			Ok(self.0.get(class).cloned())
		}
	}

	let translator = MyRemapper(build_translations(nests, apply));
	let mapped_translator = MyRemapper(build_translations(&mapped_nests, apply));

	// run

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

							let dst = if !apply {
								// we get the nests by a class un-nested names
								let nest = nests.all.get(&dst);
								if nest.is_some() {
									let x = dst.into_inner().replace('$', "__");
									// SAFETY: Replacing all `$` with `__` doesn't make any valid object class name invalid.
									unsafe { ObjClassName::from_inner_unchecked(x) }
								} else {
									dst
								}
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

fn build_translations(nests: &Nests, apply: bool) -> IndexMap<ObjClassName, ObjClassName> {
	nests.all.iter()
		.map(|(class_name, nest)| {
			fn build_translation(nests: &Nests, class_name: &ObjClassName) -> ObjClassName {
				if let Some(nest) = nests.all.get(class_name) {
					let a = build_translation(nests, &nest.encl_class_name);
					// SAFETY: todo!
					let inner_name = unsafe { ObjClassNameSlice::from_inner_unchecked(&nest.inner_name) };
					ObjClassName::from_inner_class(a, inner_name)
				} else {
					class_name.to_owned()
				}
			}

			let a = build_translation(nests, &nest.encl_class_name);
			// SAFETY: todo!
			let inner_name = unsafe { ObjClassNameSlice::from_inner_unchecked(&nest.inner_name) };
			let value = ObjClassName::from_inner_class(a, inner_name);

			(class_name.to_owned(), value)
		})
		.map(|(k, v)| if apply { (k, v) } else { (v, k) } )
		.collect()
}


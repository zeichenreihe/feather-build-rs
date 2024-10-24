use anyhow::{anyhow, Context, Result};
use crate::remapper::ARemapper;
use crate::tree::names::Namespace;
use crate::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, map_with_key_from_result_iter, MappingInfo, Mappings, MethodMapping, MethodNowodeMapping, ParameterMapping, ParameterNowodeMapping};

impl<const N: usize> Mappings<N> {
	#[allow(clippy::tabs_in_doc_comments)]
	/// Reorders the namespaces to the given order.
	/// # Example
	/// If you call this on a mapping like
	/// ```txt,ignore
	/// tiny	2	0	namespaceA	namespaceB	namespaceC
	/// c	A	B	C
	/// 	m	(LA;)V	a	b	c
	/// 	f	LA;	a	b	c
	/// ```
	/// with the given namespaces being `["namespaceC", "namespaceB", "namespaceA"]`, you get:
	/// ```txt,ignore
	/// tiny	2	0	namespaceC	namespaceB	namespaceA
	/// c	C	B	A
	/// 	m	(LC;)V	c	b	a
	/// 	f	LC;	c	b	a
	/// ```
	///
	/// You could do it like this for example:
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// let input = "\
	/// tiny	2	0	namespaceC	namespaceB	namespaceA
	/// c	C	B	A
	/// 	f	LC;	c	b	a
	/// 	m	(LC;)V	c	b	a
	/// ";
	/// let output = "\
	/// tiny	2	0	namespaceA	namespaceB	namespaceC
	/// c	A	B	C
	/// 	f	LA;	a	b	c
	/// 	m	(LA;)V	a	b	c
	/// ";
	/// let b = quill::tiny_v2::read(input.as_bytes()).unwrap()
	/// 	.reorder(["namespaceA", "namespaceB", "namespaceC"]).unwrap();
	/// let c = quill::tiny_v2::write_string(&b).unwrap();
	/// assert_eq!(output, c);
	/// ```
	pub fn reorder(&self, namespaces: [&str; N]) -> Result<Mappings<N>> {
		// at each position we have the namespace (and therefore the old index) to look to find the name
		let mut table = [Namespace::new(0)?; N];
		for i in 0..N {
			table[i] = self.get_namespace(namespaces[i])?;
		}

		let remapper = self.remapper_a(Namespace::new(0)?, table[0])?;

		Ok(Mappings {
			info: MappingInfo {
				namespaces: self.info.namespaces.reorder(table),
			},
			classes: map_with_key_from_result_iter(self.classes.values()
				.map(|class| {
					Ok(ClassNowodeMapping {
						info: ClassMapping {
							names: class.info.names.reorder(table)
								.with_context(|| anyhow!("failed to reorder names for class {:?}", class.info.names))?,
						},
						fields: map_with_key_from_result_iter(class.fields.values()
							.map(|field| Ok(FieldNowodeMapping {
								info: FieldMapping {
									desc: remapper.map_field_desc(&field.info.desc)?,
									names: field.info.names.reorder(table)
										.with_context(|| anyhow!("failed to reorder names for field {:?}", field.info.names))?,
								},
								javadoc: field.javadoc.clone(),
							}))
						)
							.with_context(|| anyhow!("in class {:?}", class.info.names))?,
						methods: map_with_key_from_result_iter(class.methods.values()
							.map(|method| Ok(MethodNowodeMapping {
								info: MethodMapping {
									desc: remapper.map_method_desc(&method.info.desc)?,
									names: method.info.names.reorder(table)
										.with_context(|| anyhow!("failed to reorder names for method {:?}", method.info.names))?,
								},
								parameters: map_with_key_from_result_iter(method.parameters.values()
									.map(|parameter| Ok(ParameterNowodeMapping {
										info: ParameterMapping {
											index: parameter.info.index,
											names: parameter.info.names.reorder(table)
												.with_context(|| anyhow!("failed to reorder names for parameter {:?}", parameter.info.names))?,
										},
										javadoc: parameter.javadoc.clone(),
									}))
								)
									.with_context(|| anyhow!("in method {:?}", method.info))?,
								javadoc: method.javadoc.clone(),
							}))
						)
							.with_context(|| anyhow!("in class {:?}", class.info.names))?,
						javadoc: class.javadoc.clone(),
					})
				})
			)?,
			javadoc: self.javadoc.clone(),
		})
	}
}

#[cfg(test)]
mod testing {
	// TODO: test internals
}
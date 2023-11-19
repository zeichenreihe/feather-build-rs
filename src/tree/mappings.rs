use crate::tree::{ClassNowode, FieldNowode, MappingNowode, MethodNowode, ParameterNowode};

pub(crate) type Mappings<T = String> = MappingNowode<
	MappingInfo<T>,
	ClassKey, ClassMapping<T>,
	FieldKey, FieldMapping<T>,
	MethodKey, MethodMapping<T>,
	ParameterKey, ParameterMapping<T>,
	JavadocMapping
>;
pub(crate) type ClassNowodeMapping<T = String> = ClassNowode<
	ClassMapping<T>,
	FieldKey, FieldMapping<T>,
	MethodKey, MethodMapping<T>,
	ParameterKey, ParameterMapping<T>,
	JavadocMapping
>;
pub(crate) type FieldNowodeMapping<T = String> = FieldNowode<
	FieldMapping<T>,
	JavadocMapping
>;
pub(crate) type MethodNowodeMapping<T = String> = MethodNowode<
	MethodMapping<T>,
	ParameterKey, ParameterMapping<T>,
	JavadocMapping,
>;
pub(crate) type ParameterNowodeMapping<T = String> = ParameterNowode<
	ParameterMapping<T>,
	JavadocMapping
>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MappingInfo<T = String> {
	pub(crate) src_namespace: String,
	pub(crate) dst_namespace: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClassKey {
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClassMapping<T = String> {
	pub(crate) src: String,
	pub(crate) dst: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldKey {
	pub(crate) desc: String,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FieldMapping<T = String> {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodKey {
	pub(crate) desc: String,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MethodMapping<T = String> {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ParameterKey {
	pub(crate) index: usize,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParameterMapping<T = String> {
	pub(crate) index: usize,
	pub(crate) src: String,
	pub(crate) dst: T,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct JavadocMapping {
	pub(crate) jav: String,
}

impl Mappings {
	pub (crate) fn remove_dummy(&mut self) {
		self.classes.retain(|_, v| !{
			v.fields.retain(|_, v| !{
				v.javadoc.is_none() && v.inner.dst.starts_with("f_")
			});

			v.methods.retain(|_, v| !{
				v.parameters.retain(|_, v| !{
					v.javadoc.is_none() && v.inner.dst.starts_with("p_")
				});

				v.javadoc.is_none() && v.parameters.is_empty() && (
					v.inner.dst.starts_with("m_") ||
						v.inner.dst == "<init>" ||
						v.inner.dst == "<clinit>"
				)
			});

			v.javadoc.is_none() && v.fields.is_empty() && v.methods.is_empty() && (
				v.inner.dst.starts_with("C_") ||
					v.inner.dst.starts_with("net/minecraft/unmapped/C_")
			)
		});
	}

	pub(crate) fn merge(a: &Mappings, b: &Mappings) -> Mappings<(String, String)> {
		// new CommandMergeTinyV2().run([
		//   invertCalamusV2.output.getAbsolutePath(), // mappings_b
		//   buildFeatherTiny.v2Output.getAbsolutePath(), // mappings_a
		//   mergedV2.getAbsolutePath(), // return this
		//   "intermediary",
		//   "official"
		// ])

		todo!()
	}

	pub(crate) fn invert(&self) -> Mappings {
		// new CommandReorderTinyV2().run([
		//   v2Input.getAbsolutePath(),
		//   output.getAbsolutePath(),
		//   namespace, "official"
		// ])

		todo!()
	}
}
use crate::tree::{ClassNowode, FieldNowode, Mapping, MethodNowode, ParameterNowode};

pub(crate) type Mappings = Mapping<
	MappingInfo,
	ClassKey, ClassMapping,
	FieldKey, FieldMapping,
	MethodKey, MethodMapping,
	ParameterKey, ParameterMapping,
	JavadocMapping
>;
pub(crate) type ClassNowodeMapping = ClassNowode<
	ClassMapping,
	FieldKey, FieldMapping,
	MethodKey, MethodMapping,
	ParameterKey, ParameterMapping,
	JavadocMapping
>;
pub(crate) type FieldNowodeMapping = FieldNowode<FieldMapping, JavadocMapping>;
pub(crate) type MethodNowodeMapping = MethodNowode<
	MethodMapping,
	ParameterKey, ParameterMapping,
	JavadocMapping,
>;
pub(crate) type ParameterNowodeMapping = ParameterNowode<ParameterMapping, JavadocMapping>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MappingInfo {
	pub(crate) src_namespace: String,
	pub(crate) dst_namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClassKey {
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClassMapping {
	pub(crate) src: String,
	pub(crate) dst: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldKey {
	pub(crate) desc: String,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FieldMapping {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodKey {
	pub(crate) desc: String,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MethodMapping {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ParameterKey {
	pub(crate) index: usize,
	pub(crate) src: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParameterMapping {
	pub(crate) index: usize,
	pub(crate) src: String,
	pub(crate) dst: String,
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
}
use crate::tree::Mapping;

pub(crate) type TinyV2Mappings = Mapping<MappingInfo, ClassMapping, FieldMapping, MethodMapping, ParameterMapping>;

#[derive(Debug, Clone)]
pub(crate) struct MappingInfo {
	pub(crate) src_namespace: String,
	pub(crate) dst_namespace: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ClassMapping {
	pub(crate) src: String,
	pub(crate) dst: String,
	pub(crate) jav: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct FieldMapping {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: String,
	pub(crate) jav: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct MethodMapping {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: String,
	pub(crate) jav: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParameterMapping {
	pub(crate) index: usize,
	pub(crate) src: String,
	pub(crate) dst: String,
	pub(crate) jav: Option<String>,
}
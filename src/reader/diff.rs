use crate::tree::Mapping;

#[derive(Debug, Clone)]
pub(crate) enum Action<T> {
	Add(T),
	Remove(T),
	Change(T, T),
	None,
}

impl Action<String> {
	pub(crate) fn new(a: String, b: String) -> Action<String> {
		match (a.is_empty(), b.is_empty()) {
			(false, false) if a != b => Action::Change(a, b),
			(false, false) => Action::None,
			(false,  true) => Action::Remove(a),
			( true, false) => Action::Add(b),
			( true,  true) => Action::None,
		}
	}
}

pub(crate) type TinyV2Diff = Mapping<MappingInfo, ClassDiff, FieldDiff, MethodDiff, ParameterDiff>;

#[derive(Debug, Clone)]
pub(crate) struct MappingInfo {
}

#[derive(Debug, Clone)]
pub(crate) struct ClassDiff {
	pub(crate) src: String,
	pub(crate) dst: Action<String>,
	pub(crate) jav: Action<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct FieldDiff {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: Action<String>,
	pub(crate) jav: Action<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct MethodDiff {
	pub(crate) desc: String,
	pub(crate) src: String,
	pub(crate) dst: Action<String>,
	pub(crate) jav: Action<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParameterDiff {
	pub(crate) index: usize,
	pub(crate) src: String,
	pub(crate) dst: Action<String>,
	pub(crate) jav: Action<String>,
}
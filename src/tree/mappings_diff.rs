use crate::tree::{ClassNowode, FieldNowode, MappingNowode, MethodNowode, ParameterNowode};
use crate::tree::mappings::{ClassKey, ClassMapping, FieldKey, FieldMapping, JavadocMapping, MappingInfo, MethodKey, MethodMapping, ParameterKey, ParameterMapping};

#[derive(Debug, Clone, Default)]
pub(crate) enum Action<T> {
	Add(T),
	Remove(T),
	Edit(T, T),
	#[default]
	None,
}

pub(crate) type MappingsDiff = MappingNowode<
	Action<MappingInfo<2>>,
	ClassKey, Action<ClassMapping<2>>,
	FieldKey, Action<FieldMapping<2>>,
	MethodKey, Action<MethodMapping<2>>,
	ParameterKey, Action<ParameterMapping<2>>,
	Action<JavadocMapping>
>;
pub(crate) type ClassNowodeDiff = ClassNowode<
	Action<ClassMapping<2>>,
	FieldKey, Action<FieldMapping<2>>,
	MethodKey, Action<MethodMapping<2>>,
	ParameterKey, Action<ParameterMapping<2>>,
	Action<JavadocMapping>
>;
pub(crate) type FieldNowodeDiff = FieldNowode<
	Action<FieldMapping<2>>,
	Action<JavadocMapping>
>;
pub(crate) type MethodNowodeDiff = MethodNowode<
	Action<MethodMapping<2>>,
	ParameterKey, Action<ParameterMapping<2>>,
	Action<JavadocMapping>
>;
pub(crate) type ParameterNowodeDiff = ParameterNowode<
	Action<ParameterMapping<2>>,
	Action<JavadocMapping>
>;

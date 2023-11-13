mod mappings;
mod class_mapping;
mod field_mapping;
mod method_mapping;
mod parameter_mapping;
mod javadoc_mapping;

pub(crate) use mappings::*;
pub(crate) use class_mapping::*;
pub(crate) use field_mapping::*;
pub(crate) use method_mapping::*;
pub(crate) use parameter_mapping::*;
pub(crate) use javadoc_mapping::*;
use crate::tiny::{AddMember, GetJavadoc, Member, SetJavadoc};

pub(crate) trait WriteableTree<C, F, M, P, J>
where
	Self: AddMember<C>,
	C: SetJavadoc<J> + AddMember<F> + AddMember<M>,
	F: SetJavadoc<J>,
	M: SetJavadoc<J> + AddMember<P>,
	P: SetJavadoc<J>,
{}

pub(crate) trait ReadableTree<C, F, M, P, J>
where
	Self: Member<String, C>,
	C: GetJavadoc<J> + Member<(String, String), F> + Member<(String, String), M>,
	F: GetJavadoc<J>,
	M: GetJavadoc<J> + Member<usize, P>,
	P: GetJavadoc<J>,
{}


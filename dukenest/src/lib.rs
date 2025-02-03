use anyhow::Result;
use dukebox::storage::{ClassRepr, Jar, ParsedJar};
use quill::tree::mappings::Mappings;
use crate::nest::Nests;

mod io;
mod nester_run;
pub mod nest;
mod nester_jar;
mod nests_mapper_run;

// TODO: doc
pub fn nest_jar<Namespace>(remap: bool, src: &impl Jar, nests: Nests<Namespace>)
		-> Result<ParsedJar<ClassRepr, Vec<u8>>> {
	nester_jar::nest_jar(remap, src, nests)
}

// TODO: doc
pub fn apply_nests_to_mappings<A, B>(mappings: Mappings<2, (A, B)>, nests: &Nests<A>) -> Result<Mappings<2, (A, B)>> {
	nester_run::apply_nests_to_mappings(mappings, nests)
}
pub fn undo_nests_to_mappings<A, B>(mappings: Mappings<2, (A, B)>, nests: &Nests<A>) -> Result<Mappings<2, (A, B)>> {
	nester_run::undo_nests_to_mappings(mappings, nests)
}

// TODO: doc
pub fn remap_nests<A, B>(nests: &Nests<A>, mappings: &Mappings<2, (A, B)>) -> Result<Nests<B>> {
	nests_mapper_run::map_nests(nests, mappings)
}



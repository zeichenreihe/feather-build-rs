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
pub fn nest_jar(silent: bool, remap: bool, src: &impl Jar, nests: Nests)
		-> Result<ParsedJar<ClassRepr, Vec<u8>>> {
	let options = nester_jar::NesterOptions { silent, remap };
	nester_jar::nest_jar(options, src, nests)
}

// TODO: doc
pub fn apply_nests_to_mappings(mappings: Mappings<2>, nests: &Nests) -> Result<Mappings<2>> {
	nester_run::nester_run(mappings, nests, true)
}
pub fn undo_nests_to_mappings(mappings: Mappings<2>, nests: &Nests) -> Result<Mappings<2>> {
	nester_run::nester_run(mappings, nests, false)
}

// TODO: doc
pub fn remap_nests(nests: Nests, mappings: &Mappings<2>) -> Result<Nests> {
	nests_mapper_run::map_nests(mappings, nests)
}



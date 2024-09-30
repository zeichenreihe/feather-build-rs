//! Crate for reading and writing mapping files, as well as running some operations on the mappings.
//!
//! Currently this crate supports reading and writing Tiny v2 (`.tiny`) files, and reading so called Tiny-diff
//! (`.tinydiff`) files. The the documentation of the [`tiny_v2`] and of the [`tiny_v2_diff`] modules for more.
//!
// TODO: document actions here

mod lines;

pub mod tiny_v2;
pub mod tiny_v2_diff;

pub mod enigma_dir;
pub mod enigma_file;

pub mod tree;
mod action;

pub mod remapper;


/// NOT PART OF PUBLIC API!
///
/// pub here bc it's used in feather-build-rs for applying diffs in insert_mappings.rs
pub fn apply_diff_option<T>(
	diff: &Option<tree::mappings_diff::Action<T>>,
	target: Option<T>,
) -> anyhow::Result<Option<T>>
	where
		T: std::fmt::Debug + Clone + PartialEq,
{
	action::apply_diff::apply_diff_option(diff, target)
}
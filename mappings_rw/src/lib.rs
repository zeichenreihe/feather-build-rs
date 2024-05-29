//! Crate for reading and writing mapping files, as well as running some operations on the mappings.
//!
//! Currently this crate supports reading and writing Tiny v2 (`.tiny`) files, and reading so called Tiny-diff
//! (`.tinydiff`) files. The the documentation of the [`tiny_v2`] and of the [`tiny_v2_diff`] modules for more.
//!
// TODO: document actions here

mod tiny_v2_line;
pub mod tiny_v2;
pub mod tiny_v2_diff;

pub mod tree;
mod action;

pub mod remapper;

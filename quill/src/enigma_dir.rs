//! Functions to read and write mappings in the "enigma directory" format.
//!
//! # Reading
// TODO: doc
//!
//! # Writing
// TODO: doc

use std::fs::File;
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use walkdir::WalkDir;
use crate::tree::mappings::{MappingInfo, Mappings};
use crate::tree::names::Namespaces;
use crate::tree::NodeInfo;

const MAPPING_EXTENSION: &str = "mapping";

pub fn read(path: impl AsRef<Path>, namespaces: Namespaces<2>) -> Result<Mappings<2>> {
	read_(path.as_ref(), namespaces)
		.with_context(|| anyhow!("cannot read enigma mappings (directory based) from path {:?}", path.as_ref()))
}
fn read_(path: &Path, namespaces: Namespaces<2>) -> Result<Mappings<2>> {
	if !path.try_exists().context("failed to check existence of path")? {
		bail!("path doesn't exist") // verified not existing
	}
	WalkDir::new(path)
		.sort_by_file_name() // make it deterministic
		.into_iter()
		.filter_map(|res| {
			res.map(|entry| {
				// skip non enigma mapping files
				if !entry.file_type().is_dir() && entry.path().extension().is_some_and(|ex| ex == MAPPING_EXTENSION) {
					Some(entry.into_path())
				} else {
					None
				}
			}).transpose()
		})
		.try_fold(
			Mappings::new(MappingInfo { namespaces }),
			|mut mappings, path| {
				let path = path?;
				crate::enigma_file::read_file_into(path, &mut mappings)?;
				Ok(mappings)
			}
		)
}

// TODO: doc
pub fn write(mappings: &Mappings<2>, path: impl AsRef<Path>) -> Result<()> {
	let path = path.as_ref();

	crate::enigma_file::write_all_for_each(mappings, |file_name| {
		if file_name.contains('.') {
			bail!("class name (dst) {file_name:?} contains '.'");
		}
		let file_name = Path::new(file_name);
		if file_name.is_absolute() {
			bail!("path relative to target write path {path:?} is absolute: {file_name:?}");
		}

		let mut target = path.join(file_name);
		target.set_extension(MAPPING_EXTENSION);

		if let Some(parent) = target.parent() {
			std::fs::create_dir_all(parent)
				.with_context(|| anyhow!("failed to create parent directories for mapping file {target:?}"))?;
		}

		File::create(&target)
			.with_context(|| anyhow!("failed to create mappings file {target:?}"))
	})
		.with_context(|| anyhow!("failed to write mappings to directory {path:?}"))
}


// TODO: tests

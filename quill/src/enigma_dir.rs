//! Functions to read and write mappings in the "enigma directory" format.
//!
//! # Reading
// TODO: doc
//!
//! # Writing
// TODO: doc

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Result};
use crate::tree::mappings::{MappingInfo, Mappings};
use crate::tree::names::Namespaces;
use crate::tree::NodeInfo;

pub fn read(path: impl AsRef<Path>, namespaces: Namespaces<2>) -> Result<Mappings<2>> {
	fn walk_dir(dir: &Path) -> Result<Vec<PathBuf>> {
		let mut paths = Vec::new();

		let mut queue: VecDeque<_> = vec![dir.to_owned()].into();
		while let Some(dir) = queue.pop_front() {
			for entry in std::fs::read_dir(dir)? {
				let entry = entry?;
				let path = entry.path();
				if path.metadata()?.is_dir() {
					queue.push_back(path);
				} else {
					paths.push(path);
				}
			}
		}

		Ok(paths)
	}

	let paths_1 = walk_dir(path.as_ref())
		.with_context(|| anyhow!("failed to read mappings dir {:?}", path.as_ref()))?;

	let mut paths_2 = Vec::new();
	for path in paths_1 {
		let file_name = path.file_name().and_then(|x| x.to_str())
			.with_context(|| anyhow!("can't get file name as string: {path:?}"))?;
		// skip non enigma mapping files
		if file_name.ends_with(".mapping") {
			paths_2.push(path);
		}
	}

	// make it deterministic
	paths_2.sort();

	let mut mappings = Mappings::new(MappingInfo { namespaces });
	for path in paths_2 {
		crate::enigma_file::read_file_into(path, &mut mappings)?;
	}

	Ok(mappings)
}


// TODO: tests

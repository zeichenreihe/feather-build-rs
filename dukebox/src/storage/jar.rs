use std::path::Path;
use anyhow::Result;
use quill::remapper::JarSuperProv;
use crate::storage::OpenedJar;

/// Represents a `.jar` in some form.
///
/// This can be in memory, like [`NamedMemJar`] and [`UnnamedMemJar`]. It can also be from a
/// file, like [`FileJar`]. It can also exist parsed in memory like [`ParsedJar`], which has the
/// entries already parsed.
///
/// You can [`open`][Jar::open] a jar to get to it's content. See [`OpenedJar`] for more.
///
/// A [`Jar`] also provides a method to store it to a suggested path. Note that the suggested path may
/// also not be used (like [`FileJar`] does).
pub trait Jar {
	type Opened<'a>: OpenedJar where Self: 'a;

	/// Opens the jar for reading.
	fn open(&self) -> Result<Self::Opened<'_>>;

	/// Asks the jar implementation for storing the jar to the suggested path.
	///
	/// Returns the path the jar was actually stored to.
	///
	/// The reason for a suggested path only is that some implementors (like [`FileJar`]) are already
	/// stored on disk, and this would require copying the file.
	fn put_to_file<'a>(&'a self, suggested: &'a Path) -> Result<&'a Path>;

	fn get_super_classes_provider(&self) -> Result<JarSuperProv> {
		self.open()?.get_super_classes_provider()
	}
}

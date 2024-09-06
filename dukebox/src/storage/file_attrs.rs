use zip::DateTime;
use zip::write::{ExtendedFileOptions, FileOptions};

/// The file times attribute.
///
/// Use the [`Default`] implementation for having [`None`] everywhere.
#[derive(Clone, Copy, Debug, Default)]
pub struct BasicFileAttributes {
	pub last_modified: Option<DateTime>,
	pub mtime: Option<u32>,
	pub atime: Option<u32>,
	pub ctime: Option<u32>,
}

impl BasicFileAttributes {
	pub(crate) fn to_file_options<'k>(self) -> FileOptions<'k, ExtendedFileOptions> {
		let mut file_options = FileOptions::default();

		if let Some(last_modified) = self.last_modified {
			file_options = file_options.last_modified_time(last_modified);
		}
		// TODO: awaiting lib support: set the ctime, atime, mtime to the ones from self

		file_options
	}
}
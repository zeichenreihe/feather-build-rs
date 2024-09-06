use std::fmt::{Debug, Formatter};
use anyhow::Result;
use crate::storage::{BasicFileAttributes, IsClass, IsOther};

pub trait JarEntry {
	fn name(&self) -> &str;

	fn attrs(&self) -> BasicFileAttributes;

	type Class: IsClass;
	type Other: IsOther;
	fn to_jar_entry_enum(self) -> Result<JarEntryEnum<Self::Class, Self::Other>>;
}


/// The data of an entry of a jar.
///
/// The [`Debug`] implementation doesn't try to print the contents.
pub enum JarEntryEnum<Class, Other> {
	Dir,
	Class(Class),
	Other(Other),
}

impl<Class, Other> JarEntryEnum<Class, Other> {
	pub(crate) fn try_map_both<NewClass, NewOther>(
		self,
		class_f: impl FnOnce(Class) -> Result<NewClass>,
		other_f: impl FnOnce(Other) -> Result<NewOther>,
	) -> Result<JarEntryEnum<NewClass, NewOther>> {
		use JarEntryEnum::*;
		Ok(match self {
			Dir => Dir,
			Class(class) => Class(class_f(class)?),
			Other(other) => Other(other_f(other)?),
		})
	}
}

/// [`Debug`] only prints the type, not the contents.
impl<Class, Other> Debug for JarEntryEnum<Class, Other> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		use JarEntryEnum::*;
		match self {
			Dir => write!(f, "Dir"),
			Class(_) => write!(f, "Class"),
			Other(_) => write!(f, "Other"),
		}
	}
}


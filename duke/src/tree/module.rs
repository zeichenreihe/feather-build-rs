use std::fmt::{Debug, Display, Formatter};
use java_string::{JavaStr, JavaString};
use crate::macros::make_string_str_like;
use crate::tree::class::ClassName;

//TODO: consider making a "ModuleVersion" kind of make_string_str_like! struct, but first figure out
// if there's a format checked by javac for module versions that could be parsed (like Field/MethodDescriptor)

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
	pub(crate) name: ModuleName,
	pub(crate) flags: ModuleFlags,
	pub(crate) version: Option<JavaString>, // represents a module version...
	pub(crate) requires: Vec<ModuleRequires>,
	pub(crate) exports: Vec<ModuleExports>,
	pub(crate) opens: Vec<ModuleOpens>,
	pub(crate) uses: Vec<ClassName>,
	pub(crate) provides: Vec<ModuleProvides>,
}

make_string_str_like!(
	pub ModuleName(JavaString);
	pub ModuleNameSlice(JavaStr);
	is_valid(s) = Ok(()); // TODO: see JVMS 4.2.3
);

impl Display for ModuleName {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		Display::fmt(self.as_slice(), f)
	}
}
impl Display for ModuleNameSlice {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_inner())
	}
}

make_string_str_like!(
	pub PackageName(JavaString);
	pub PackageNameSlice(JavaStr);
	is_valid(s) = Ok(()); // TODO: see JVMS 4.2.3
);

#[derive(Copy, Clone, PartialEq)]
pub struct ModuleFlags {
	pub(crate) is_open: bool,
	pub(crate) is_synthetic: bool,
	pub(crate) is_mandated: bool,
}

impl Debug for ModuleFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("ModuleFlags { ")?;
		if self.is_open      { f.write_str("open ")?; }
		if self.is_synthetic { f.write_str("synthetic ")?; }
		if self.is_mandated  { f.write_str("mandated ")?; }
		f.write_str("}")
	}
}

impl From<u16> for ModuleFlags {
	fn from(value: u16) -> Self {
		ModuleFlags {
			is_open:      value & 0x0010 != 0,
			is_synthetic: value & 0x1000 != 0,
			is_mandated:  value & 0x8000 != 0,
		}
	}
}

impl From<ModuleFlags> for u16 {
	fn from(value: ModuleFlags) -> Self {
		(if value.is_open      { 0x0010 } else { 0 }) |
		(if value.is_synthetic { 0x1000 } else { 0 }) |
		(if value.is_mandated  { 0x8000 } else { 0 })
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleRequires {
	pub(crate) name: ModuleName,
	pub(crate) flags: ModuleRequiresFlags,
	pub(crate) version: Option<JavaString>, // represents a module version...
}

#[derive(Copy, Clone, PartialEq)]
pub struct ModuleRequiresFlags {
	pub(crate) is_transitive: bool,
	pub(crate) is_static_phase: bool,
	pub(crate) is_synthetic: bool,
	pub(crate) is_mandated: bool,
}

impl Debug for ModuleRequiresFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("ModuleRequiresFlags { ")?;
		if self.is_transitive   { f.write_str("transitive ")?; }
		if self.is_static_phase { f.write_str("static-phase ")?; }
		if self.is_synthetic    { f.write_str("synthetic ")?; }
		if self.is_mandated     { f.write_str("mandated ")?; }
		f.write_str("}")
	}
}

impl From<u16> for ModuleRequiresFlags {
	fn from(value: u16) -> Self {
		ModuleRequiresFlags {
			is_transitive:   value & 0x0020 != 0,
			is_static_phase: value & 0x0040 != 0,
			is_synthetic:    value & 0x1000 != 0,
			is_mandated:     value & 0x8000 != 0,
		}
	}
}

impl From<ModuleRequiresFlags> for u16 {
	fn from(value: ModuleRequiresFlags) -> Self {
		(if value.is_transitive   { 0x0020 } else { 0 }) |
		(if value.is_static_phase { 0x0040 } else { 0 }) |
		(if value.is_synthetic    { 0x1000 } else { 0 }) |
		(if value.is_mandated     { 0x8000 } else { 0 })
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleExports {
	pub(crate) name: PackageName,
	pub(crate) flags: ModuleExportsFlags,
	pub(crate) exports_to: Vec<ModuleName>,
}

#[derive(Copy, Clone, PartialEq)]
pub struct ModuleExportsFlags {
	pub(crate) is_synthetic: bool,
	pub(crate) is_mandated: bool,
}

impl Debug for ModuleExportsFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("ModuleExportsFlags { ")?;
		if self.is_synthetic { f.write_str("synthetic ")?; }
		if self.is_mandated  { f.write_str("mandated ")?; }
		f.write_str("}")
	}
}

impl From<u16> for ModuleExportsFlags {
	fn from(value: u16) -> Self {
		ModuleExportsFlags {
			is_synthetic: value & 0x1000 != 0,
			is_mandated:  value & 0x8000 != 0,
		}
	}
}

impl From<ModuleExportsFlags> for u16 {
	fn from(value: ModuleExportsFlags) -> Self {
		(if value.is_synthetic { 0x1000 } else { 0 }) |
		(if value.is_mandated  { 0x8000 } else { 0 })
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleOpens {
	pub(crate) name: PackageName,
	pub(crate) flags: ModuleOpensFlags,
	pub(crate) opens_to: Vec<ModuleName>,
}

#[derive(Copy, Clone, PartialEq)]
pub struct ModuleOpensFlags {
	pub(crate) is_synthetic: bool,
	pub(crate) is_mandated: bool,
}

impl Debug for ModuleOpensFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("ModuleOpensFlags { ")?;
		if self.is_synthetic { f.write_str("synthetic ")?; }
		if self.is_mandated  { f.write_str("mandated ")?; }
		f.write_str("}")
	}
}

impl From<u16> for ModuleOpensFlags {
	fn from(value: u16) -> Self {
		ModuleOpensFlags {
			is_synthetic: value & 0x1000 != 0,
			is_mandated:  value & 0x8000 != 0,
		}
	}
}

impl From<ModuleOpensFlags> for u16 {
	fn from(value: ModuleOpensFlags) -> Self {
		(if value.is_synthetic { 0x1000 } else { 0 }) |
		(if value.is_mandated  { 0x8000 } else { 0 })
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleProvides {
	pub(crate) name: ClassName,
	pub(crate) provides_with: Vec<ClassName>,
}
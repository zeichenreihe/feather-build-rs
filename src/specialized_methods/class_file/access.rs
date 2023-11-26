use std::fmt::{Debug, Formatter};

#[derive(Clone, PartialEq)]
pub(crate) struct ClassInfoAccess {
	pub(crate) is_public: bool,
	pub(crate) is_final: bool,
	pub(crate) is_super: bool, // consider this true for every class file...
	pub(crate) is_interface: bool,
	pub(crate) is_abstract: bool,
	pub(crate) is_synthetic: bool,
	pub(crate) is_annotation: bool,
	pub(crate) is_enum: bool,
}

impl ClassInfoAccess {
	pub(crate) fn parse(access_flags: u16) -> ClassInfoAccess {
		let is_public     = access_flags & 0x0001 != 0;
		let is_final      = access_flags & 0x0010 != 0;
		let is_super      = access_flags & 0x0020 != 0;
		let is_interface  = access_flags & 0x0200 != 0;
		let is_abstract   = access_flags & 0x0400 != 0;
		let is_synthetic  = access_flags & 0x1000 != 0;
		let is_annotation = access_flags & 0x2000 != 0;
		let is_enum       = access_flags & 0x4000 != 0;
		// other bits: reserved for future use

		ClassInfoAccess { is_public, is_final, is_super, is_interface, is_abstract, is_synthetic, is_annotation, is_enum }
	}
}

impl Debug for ClassInfoAccess {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("ClassInfoAccess { ")?;
		if self.is_public     { f.write_str("public ")?; }
		if self.is_final      { f.write_str("final ")?; }
		if self.is_super      { f.write_str("super ")?; }
		if self.is_interface  { f.write_str("interface ")?; }
		if self.is_abstract   { f.write_str("abstract ")?; }
		if self.is_synthetic  { f.write_str("synthetic ")?; }
		if self.is_annotation { f.write_str("annotation ")?; }
		if self.is_enum       { f.write_str("enum ")?; }
		f.write_str("}")
	}
}


#[derive(Clone, PartialEq)]
pub(crate) struct FieldInfoAccess {
	pub(crate) is_public: bool,
	pub(crate) is_private: bool,
	pub(crate) is_protected: bool,
	pub(crate) is_static: bool,
	pub(crate) is_final: bool,
	pub(crate) is_volatile: bool,
	pub(crate) is_transient: bool,
	pub(crate) is_synthetic: bool,
	pub(crate) is_enum: bool,
}

impl FieldInfoAccess {
	pub(crate) fn parse(access_flags: u16) -> FieldInfoAccess {
		let is_public    = access_flags & 0x0001 != 0;
		let is_private   = access_flags & 0x0002 != 0;
		let is_protected = access_flags & 0x0004 != 0;
		let is_static    = access_flags & 0x0008 != 0;
		let is_final     = access_flags & 0x0010 != 0;
		let is_volatile  = access_flags & 0x0040 != 0;
		let is_transient = access_flags & 0x0080 != 0;
		let is_synthetic = access_flags & 0x1000 != 0;
		let is_enum      = access_flags & 0x4000 != 0;
		// other bits: reserved for future use

		FieldInfoAccess { is_public, is_private, is_protected, is_static, is_final, is_volatile, is_transient, is_synthetic, is_enum }
	}
}

impl Debug for FieldInfoAccess {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("FieldInfoAccess { ")?;
		if self.is_public    { f.write_str("public ")?; }
		if self.is_private   { f.write_str("private ")?; }
		if self.is_protected { f.write_str("protected ")?; }
		if self.is_static    { f.write_str("static ")?; }
		if self.is_final     { f.write_str("final ")?; }
		if self.is_volatile  { f.write_str("volatile ")?; }
		if self.is_transient { f.write_str("transient ")?; }
		if self.is_synthetic { f.write_str("synthetic ")?; }
		if self.is_enum      { f.write_str("enum ")?; }
		f.write_str("}")
	}
}



#[derive(Clone, PartialEq)]
pub(crate) struct MethodInfoAccess {
	pub(crate) is_public: bool,
	pub(crate) is_private: bool,
	pub(crate) is_protected: bool,
	pub(crate) is_static: bool,
	pub(crate) is_final: bool,
	pub(crate) is_synchronised: bool,
	pub(crate) is_bridge: bool,
	pub(crate) is_varargs: bool,
	pub(crate) is_native: bool,
	pub(crate) is_abstract: bool,
	pub(crate) is_strict: bool,
	pub(crate) is_synthetic: bool,
}

impl MethodInfoAccess {
	pub(crate) fn parse(access_flags: u16) -> MethodInfoAccess {
		let is_public       = access_flags & 0x0001 != 0;
		let is_private      = access_flags & 0x0002 != 0;
		let is_protected    = access_flags & 0x0004 != 0;
		let is_static       = access_flags & 0x0008 != 0;
		let is_final        = access_flags & 0x0010 != 0;
		let is_synchronised = access_flags & 0x0020 != 0;
		let is_bridge       = access_flags & 0x0040 != 0;
		let is_varargs      = access_flags & 0x0080 != 0;
		let is_native       = access_flags & 0x0100 != 0;
		let is_abstract     = access_flags & 0x0400 != 0;
		let is_strict       = access_flags & 0x0800 != 0;
		let is_synthetic    = access_flags & 0x1000 != 0;
		// other bits: reserved for future use

		MethodInfoAccess { is_public, is_private, is_protected, is_static, is_final, is_synchronised, is_bridge, is_varargs, is_native, is_abstract, is_strict, is_synthetic }
	}
}

impl Debug for MethodInfoAccess {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("MethodInfoAccess { ")?;
		if self.is_public       { f.write_str("public ")?; }
		if self.is_private      { f.write_str("private ")?; }
		if self.is_protected    { f.write_str("protected ")?; }
		if self.is_static       { f.write_str("static ")?; }
		if self.is_final        { f.write_str("final ")?; }
		if self.is_synchronised { f.write_str("synchronised ")?; }
		if self.is_bridge       { f.write_str("bridge ")?; }
		if self.is_varargs      { f.write_str("varargs ")?; }
		if self.is_native       { f.write_str("native ")?; }
		if self.is_abstract     { f.write_str("abstract ")?; }
		if self.is_strict       { f.write_str("strict ")?; }
		if self.is_synthetic    { f.write_str("synthetic ")?; }
		f.write_str("}")
	}
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ParameterAccess {
	pub(crate) is_final: bool,
	pub(crate) is_synthetic: bool,
	pub(crate) is_mandated: bool,
}

impl ParameterAccess {
	pub(crate) fn parse(access_flags: u16) -> ParameterAccess {
		let is_final     = access_flags & 0x0010 != 0;
		let is_synthetic = access_flags & 0x1000 != 0;
		let is_mandated  = access_flags & 0x8000 != 0;
		// other bits are reserved for future use

		ParameterAccess { is_final, is_synthetic, is_mandated, }
	}
}

impl Debug for ParameterAccess {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("MethodInfoAccess { ")?;
		if self.is_final        { f.write_str("final ")?; }
		if self.is_synthetic    { f.write_str("synthetic ")?; }
		if self.is_mandated     { f.write_str("mandated ")?; }
		f.write_str("}")
	}
}

use std::fmt::{Debug, Formatter};

#[derive(Clone)]
pub(crate) struct ClassAccessFlags {
	pub(crate) is_public: bool,
	pub(crate) is_final: bool,
	pub(crate) is_super: bool,
	pub(crate) is_interface: bool,
	pub(crate) is_abstract: bool,
	pub(crate) is_synthetic: bool,
	pub(crate) is_annotation: bool,
	pub(crate) is_enum: bool,
}

impl Debug for ClassAccessFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("{ ")?;
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

#[derive(Clone)]
pub(crate) struct FieldAccessFlags {
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

impl Debug for FieldAccessFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("{ ")?;
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

#[derive(Clone)]
pub(crate) struct MethodAccessFlags {
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

impl Debug for MethodAccessFlags {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str("{ ")?;
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
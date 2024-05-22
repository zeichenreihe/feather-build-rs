//! This crate contains a direct binary representation of a java class file.
//!
//! Use the [Java Virtual Machine Specification, Chapter 4](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html)
//! to build a class file. No format checking is done when creating a `Vec<u8>`.
//!
//! This code creates the same class as `javac` version `1.8_402` would, if ran on the class
//! ```java,ignore
//! package org.example;
//!
//! class Main {}
//! ```
//!
//! You can write this to a `Vec<u8>` by using the [ClassFile::to_bytes] function.
//! ```
//! # use pretty_assertions::assert_eq;
//! use raw_class_file::{AttributeInfo, ClassFile, CpInfo, flags, insn, LineNumberTableEntry, MethodInfo};
//! let class = ClassFile {
//!     minor_version: 0,
//!     major_version: 52,
//!     constant_pool: vec![
//!         // the constant pool indices start at 1, not at 0
//!         CpInfo::Methodref { class_index: 3,name_and_type_index: 10 },
//!         CpInfo::Class { name_index: 11 },
//!         CpInfo::Class { name_index: 12 },
//!         CpInfo::Utf8 { bytes: b"<init>".to_vec() },
//!         CpInfo::Utf8 { bytes: b"()V".to_vec() },
//!         CpInfo::Utf8 { bytes: b"Code".to_vec() },
//!         CpInfo::Utf8 { bytes: b"LineNumberTable".to_vec() },
//!         CpInfo::Utf8 { bytes: b"SourceFile".to_vec() },
//!         CpInfo::Utf8 { bytes: b"Main.java".to_vec() },
//!         CpInfo::NameAndType { name_index: 4, descriptor_index: 5 },
//!         CpInfo::Utf8 { bytes: b"org/example/Main".to_vec() },
//!         CpInfo::Utf8 { bytes: b"java/lang/Object".to_vec() },
//!     ],
//!     access_flags: flags::ACC_SUPER,
//!     this_class: 2,
//!     super_class: 3,
//!     interfaces: vec![],
//!     fields: vec![],
//!     methods: vec![
//!         MethodInfo {
//!             access_flags: 0,
//!             name_index: 4,
//!             descriptor_index: 5,
//!             attributes: vec![
//!                 AttributeInfo::Code {
//!                     attribute_name_index: 6,
//!                     max_stack: 1,
//!                     max_locals: 1,
//!                     code: vec![
//!                         insn::aload_0,
//!                         insn::invokespecial, 0, 1,
//!                         insn::r#return,
//!                     ],
//!                     exception_table: vec![],
//!                     attributes: vec![
//!                         AttributeInfo::LineNumberTable {
//!                             attribute_name_index: 7,
//!                             line_number_table: vec![
//!                                 LineNumberTableEntry { start_pc: 0,line_number: 3 },
//!                             ],
//!                         }
//!                     ],
//!                 },
//!             ],
//!         }
//!     ],
//!     attributes: vec![
//!         AttributeInfo::SourceFile {
//!             attribute_name_index: 8,
//!             sourcefile_index: 9,
//!         }
//!     ],
//! };
//!
//! let bytes = class.to_bytes();
//!
//! let class_2 = ClassFile::read(&mut std::io::Cursor::new(&bytes)).unwrap();
//!
//! assert_eq!(bytes.len(), class_2.length());
//!
//! let mut bytes_2 = Vec::new();
//! class_2.write(&mut bytes_2).unwrap();
//!
//! assert_eq!(bytes, bytes_2);
//! ```
//!
//! As you can see, [ClassFile] also has [ClassFile::read] and [ClassFile::write] to interface with things implementing [std::io::Read] and
//! [std::io::Write].
//!
//! The [ClassFile::length] function gives the computed length of a class file, useful for allocating sufficient memory for buffers.
use macros::notation;

mod macros;

impl ClassFile {
	/// Converts the class file to binary representation.
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut vec = Vec::with_capacity(self.length());
		self._write(&mut vec).expect("Writing to a Vec<u8> should never fail");
		vec
	}

	pub fn write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
		self._write(writer)
	}

	pub fn read(reader: &mut impl std::io::Read) -> std::io::Result<ClassFile> {
		ClassFile::_read(reader, None)
	}

	/// The length of the class file produced by [`Self::to_bytes`], in bytes.
	pub fn length(&self) -> usize {
		self._len() as usize
	}
}

pub mod flags {
	pub const ACC_PUBLIC: u16       = 0x0001; // class, field, method, inner class
	pub const ACC_PRIVATE: u16      = 0x0002; // field, method, inner class
	pub const ACC_PROTECTED: u16    = 0x0004; // field, method, inner class
	pub const ACC_STATIC: u16       = 0x0008; // field, method, inner class
	pub const ACC_FINAL: u16        = 0x0010; // class, field, method, inner class, parameter
	pub const ACC_SUPER: u16        = 0x0020; // class
	pub const ACC_OPEN: u16         = 0x0020; // module
	pub const ACC_TRANSITIVE: u16   = 0x0020; // module requires
	pub const ACC_SYNCHRONIZED: u16 = 0x0020; // method
	pub const ACC_VOLATILE: u16     = 0x0040; // field
	pub const ACC_BRIDGE: u16       = 0x0040; // method
	pub const ACC_STATIC_PHASE: u16 = 0x0040; // module requires
	pub const ACC_TRANSIENT: u16    = 0x0080; // field
	pub const ACC_VARARGS: u16      = 0x0080; // method
	pub const ACC_NATIVE: u16       = 0x0100; // method
	pub const ACC_INTERFACE: u16    = 0x0200; // class, inner class
	pub const ACC_ABSTRACT: u16     = 0x0400; // class, method, inner class
	pub const ACC_STRICT: u16       = 0x0800; // method
	pub const ACC_SYNTHETIC: u16    = 0x1000; // class, field, method, inner class, parameter, module, module requires, module exports, module opens
	pub const ACC_ANNOTATION: u16   = 0x2000; // class, inner class
	pub const ACC_ENUM: u16         = 0x4000; // class, field, inner class
	pub const ACC_MODULE: u16       = 0x8000; // class
	pub const ACC_MANDATED: u16     = 0x8000; // parameter, module, module requires, module exports, module opens
}

notation!(
	struct ClassFile this {
		const magic: u32 = 0xCAFEBABEu32,
		mut minor_version: u16,
		mut major_version: u16,
		const constant_pool_count: u16 = this.constant_pool.len() + 1,
		mut constant_pool: Vec<CpInfo> {constant_pool_count - 1}; Some(&constant_pool),
		mut access_flags: u16,
		mut this_class: u16,
		mut super_class: u16,
		//interfaces_count: u16,
		mut interfaces: Vec<u16> [u16],
		//fields_count: u16,
		mut fields: Vec<FieldInfo> [u16],
		//methods_count: u16,
		mut methods: Vec<MethodInfo> [u16],
		//attributes_count: u16,
		mut attributes: Vec<AttributeInfo> [u16],
	}
);

notation!(
	enum CpInfo {
		tag: u8,
		Class {
			= 7 => 7,
			mut name_index: u16,
		},
		Fieldref {
			= 9 => 9,
			mut class_index: u16,
			mut name_and_type_index: u16,
		},
		Methodref {
			= 10 => 10,
			mut class_index: u16,
			mut name_and_type_index: u16,
		},
		InterfaceMethodref {
			= 11 => 11,
			mut class_index: u16,
			mut name_and_type_index: u16,
		},
		String {
			= 8 => 8,
			mut string_index: u16,
		},
		Integer {
			= 3 => 3,
			mut bytes: u32,
		},
		Float {
			= 4 => 4,
			mut bytes: u32,
		},
		Long {
			= 5 => 5,
			mut high_bytes: u32,
			mut low_bytes: u32,
		},
		Double {
			= 6 => 6,
			mut high_bytes: u32,
			mut low_bytes: u32,
		},
		NameAndType {
			= 12 => 12,
			mut name_index: u16,
			mut descriptor_index: u16,
		},
		Utf8 {
			= 1 => 1,
			//length: u16,
			mut bytes: Vec<u8> [u16],
		},
		MethodHandle {
			= 15 => 15,
			mut reference_kind: u8,
			mut reference_index: u16,
		},
		MethodType {
			= 16 => 16,
			mut descriptor_index: u16,
		},
		Dynamic {
			= 17 => 17,
			mut bootstrap_method_attr_index: u16,
			mut name_and_type_index: u16,
		},
		InvokeDynamic {
			= 18 => 18,
			mut bootstrap_method_attr_index: u16,
			mut name_and_type_index: u16,
		},
		Module {
			= 19 => 19,
			mut name_index: u16,
		},
		Package {
			= 20 => 20,
			mut name_index: u16,
		},
		_ {
			tag => Err(std::io::Error::other(format!("Unexpected constant pool tag {}", tag))),
		},
	}
);

notation!(
	struct FieldInfo {
		mut access_flags: u16,
		mut name_index: u16,
		mut descriptor_index: u16,
		//attributes_count: u16,
		mut attributes: Vec<AttributeInfo> [u16],
	}
);


notation!(
	struct MethodInfo {
		mut access_flags: u16,
		mut name_index: u16,
		mut descriptor_index: u16,
		//attributes_count: u16,
		mut attributes: Vec<AttributeInfo> [u16],
	}
);

fn pool_has_utf8(pool: Option<&Vec<CpInfo>>, index: u16, value: &[u8]) -> Result<bool, std::io::Error> {
	let Some(pool) = pool else {
		return Err(std::io::Error::other("Expected to have constant pool at this point of reading"));
	};
	let Some(entry) = pool.get((index - 1 ) as usize) else {
		return Err(std::io::Error::other(format!("No constant pool entry at position {}", index)));
	};
	let CpInfo::Utf8 { bytes } = entry else {
		return Err(std::io::Error::other(format!("Expected constant pool entry Utf8 at position {}, got {:?}", index, entry)));
	};
	Ok(bytes.as_slice() == value)
}

notation!(
	enum AttributeInfo [pool] {
		attribute_name_index: u16,
		ConstantValue {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"ConstantValue")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2,
			mut constantvalue_index: u16,
		},
		Code this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"Code")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			mut max_stack: u16,
			mut max_locals: u16,
			//code_length: u32,
			mut code: Vec<u8> [u32],
			//exception_table_length: u16,
			mut exception_table: Vec<ExceptionTableEntry> [u16],
			//attributes_count: u16,
			mut attributes: Vec<AttributeInfo> [u16],
		},
		StackMapTable this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"StackMapTable")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//number_of_entries: u16,
			mut entries: Vec<StackMapFrame> [u16],
		},
		Exceptions {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"Exceptions")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2 + 2 * exception_index_table.len(),
			//number_of_exceptions: u16,
			mut exception_index_table: Vec<u16> [u16],
		},
		InnerClasses this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"InnerClasses")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//number_of_classes: u16,
			mut classes: Vec<InnerClassesEntry> [u16],
		},
		EnclosingMethod {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"EnclosingMethod")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 4,
			mut class_index: u16,
			mut method_index: u16,
		},
		Synthetic {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"Synthetic")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 0,
		},
		Signature {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"Signature")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2,
			mut signature_index: u16,
		},
		SourceFile {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"SourceFile")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2,
			mut sourcefile_index: u16,
		},
		SourceDebugExtension {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"SourceDebugExtension")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			//attribute_length: u32,
			mut debug_extension: Vec<u8> [u32],
		},
		LineNumberTable this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"LineNumberTable")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//line_number_table_length: u16,
			mut line_number_table: Vec<LineNumberTableEntry> [u16],
		},
		LocalVariableTable this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"LocalVariableTable")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//local_variable_table_length: u16,
			mut local_variable_table: Vec<LocalVariableTableEntry> [u16],
		},
		LocalVariableTypeTable this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"LocalVariableTypeTable")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//local_variable_type_table_length: u16,
			mut local_variable_type_table: Vec<LocalVariableTypeTableEntry> [u16],
		},
		Deprecated {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"Deprecated")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 0,
		},
		RuntimeVisibleAnnotations this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"RuntimeVisibleAnnotations")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//num_annotations: u16,
			mut annotations: Vec<Annotation> [u16],
		},
		RuntimeInvisibleAnnotations this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"RuntimeInvisibleAnnotations")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//num_annotations: u16,
			mut annotations: Vec<Annotation> [u16],
		},
		RuntimeVisibleParameterAnnotations this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"RuntimeVisibleParameterAnnotations")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//num_parameters: u8,
			mut parameter_annotations: Vec<ParameterAnnotationEntry> [u8],
		},
		RuntimeInvisibleParameterAnnotations this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"RuntimeInvisibleParameterAnnotations")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//num_parameters: u8,
			mut parameter_annotations: Vec<ParameterAnnotationEntry> [u8],
		},
		// TODO: RuntimeVisibleTypeAnnotations_attribute
		// TODO: RuntimeInvisibleTypeAnnotations_attribute
		AnnotationDefault this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"AnnotationDefault")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			mut default_value: ElementValue,
		},
		BootstrapMethods this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"BootstrapMethods")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//num_bootstrap_methods: u16,
			mut bootstrap_methods: Vec<BootstrapMethodsEntry> [u16],
		},
		MethodParameters this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"MethodParameters")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//parameters_count: u16,
			mut parameters: Vec<MethodParametersEntry> [u16],
		},
		Module this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"Module")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,

			mut module_name_index: u16,
			mut module_flags: u16,
			mut module_version_index: u16,

			//requires_count: u16,
			mut requires: Vec<ModuleRequiresEntry> [u16],

			//exports_count: u16,
			mut exports: Vec<ModuleExportsEntry> [u16],

			//opens_count: u16,
			mut opens: Vec<ModuleOpensEntry> [u16],

			//uses_count: u16,
			mut uses_index: Vec<u16> [u16],

			//provides_count: u16,
			mut provides: Vec<ModuleProvidesEntry> [u16],
		},
		ModulePackages {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"ModulePackages")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2 + 2 * package_index.len(),
			//package_count: u16,
			mut package_index: Vec<u16> [u16],
		},
		ModuleMainClass {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"ModuleMainClass")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2,
			mut main_class_index: u16,
		},
		NestHost {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"NestHost")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2,
			mut host_class_index: u16,
		},
		NestMembers {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"NestMembers")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2 * classes.len(),
			//number_of_classes: u16,
			mut classes: Vec<u16> [u16],
		},
		Record this {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"Record")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = this._len() - 6,
			//components_count: u16,
			mut components: Vec<RecordComponentInfo> [u16],
		},
		PermittedSubclasses {
			= *attribute_name_index => attribute_name_index if pool_has_utf8(pool, attribute_name_index, b"PermittedSubclasses")?,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			const attribute_length: u32 = 2 + 2 * classes.len(),
			//number_of_classes: u16,
			mut classes: Vec<u16> [u16],
		},
		Other {
			= *attribute_name_index => attribute_name_index,
			mut attribute_name_index: u16 nowrite = attribute_name_index,
			//attribute_length: u32,
			mut info: Vec<u8> [u32],
		},
	}
);

#[allow(non_upper_case_globals)]
pub mod insn {
	pub const nop: u8 = 0x00;
	pub const aconst_null: u8 = 0x01;
	pub const iconst_m1: u8 = 0x02;
	pub const iconst_0: u8 = 0x03;
	pub const iconst_1: u8 = 0x04;
	pub const iconst_2: u8 = 0x05;
	pub const iconst_3: u8 = 0x06;
	pub const iconst_4: u8 = 0x07;
	pub const iconst_5: u8 = 0x08;
	pub const lconst_0: u8 = 0x09;
	pub const lconst_1: u8 = 0x0a;
	pub const fconst_0: u8 = 0x0b;
	pub const fconst_1: u8 = 0x0c;
	pub const fconst_2: u8 = 0x0d;
	pub const dconst_0: u8 = 0x0e;
	pub const dconst_1: u8 = 0x0f;
	pub const bipush: u8 = 0x10;
	pub const sipush: u8 = 0x11;
	pub const ldc: u8 = 0x12;
	pub const ldc_w: u8 = 0x13;
	pub const ldc2_w: u8 = 0x14;
	pub const iload: u8 = 0x15;
	pub const lload: u8 = 0x16;
	pub const fload: u8 = 0x17;
	pub const dload: u8 = 0x18;
	pub const aload: u8 = 0x19;
	pub const iload_0: u8 = 0x1a;
	pub const iload_1: u8 = 0x1b;
	pub const iload_2: u8 = 0x1c;
	pub const iload_3: u8 = 0x1d;
	pub const lload_0: u8 = 0x1e;
	pub const lload_1: u8 = 0x1f;
	pub const lload_2: u8 = 0x20;
	pub const lload_3: u8 = 0x21;
	pub const fload_0: u8 = 0x22;
	pub const fload_1: u8 = 0x23;
	pub const fload_2: u8 = 0x24;
	pub const fload_3: u8 = 0x25;
	pub const dload_0: u8 = 0x26;
	pub const dload_1: u8 = 0x27;
	pub const dload_2: u8 = 0x28;
	pub const dload_3: u8 = 0x29;
	pub const aload_0: u8 = 0x2a;
	pub const aload_1: u8 = 0x2b;
	pub const aload_2: u8 = 0x2c;
	pub const aload_3: u8 = 0x2d;
	pub const iaload: u8 = 0x2e;
	pub const laload: u8 = 0x2f;
	pub const faload: u8 = 0x30;
	pub const daload: u8 = 0x31;
	pub const aaload: u8 = 0x32;
	pub const baload: u8 = 0x33;
	pub const caload: u8 = 0x34;
	pub const saload: u8 = 0x35;
	pub const istore: u8 = 0x36;
	pub const lstore: u8 = 0x37;
	pub const fstore: u8 = 0x38;
	pub const dstore: u8 = 0x39;
	pub const astore: u8 = 0x3a;
	pub const istore_0: u8 = 0x3b;
	pub const istore_1: u8 = 0x3c;
	pub const istore_2: u8 = 0x3d;
	pub const istore_3: u8 = 0x3e;
	pub const lstore_0: u8 = 0x3f;
	pub const lstore_1: u8 = 0x40;
	pub const lstore_2: u8 = 0x41;
	pub const lstore_3: u8 = 0x42;
	pub const fstore_0: u8 = 0x43;
	pub const fstore_1: u8 = 0x44;
	pub const fstore_2: u8 = 0x45;
	pub const fstore_3: u8 = 0x46;
	pub const dstore_0: u8 = 0x47;
	pub const dstore_1: u8 = 0x48;
	pub const dstore_2: u8 = 0x49;
	pub const dstore_3: u8 = 0x4a;
	pub const astore_0: u8 = 0x4b;
	pub const astore_1: u8 = 0x4c;
	pub const astore_2: u8 = 0x4d;
	pub const astore_3: u8 = 0x4e;
	pub const iatore: u8 = 0x4f;
	pub const latore: u8 = 0x50;
	pub const fatore: u8 = 0x51;
	pub const datore: u8 = 0x52;
	pub const aatore: u8 = 0x53;
	pub const batore: u8 = 0x54;
	pub const catore: u8 = 0x55;
	pub const satore: u8 = 0x56;
	pub const pop: u8 = 0x57;
	pub const pop2: u8 = 0x58;
	pub const dup: u8 = 0x59;
	pub const dup_x1: u8 = 0x5a;
	pub const dup_x2: u8 = 0x5b;
	pub const dup2: u8 = 0x5c;
	pub const dup2_x1: u8 = 0x5d;
	pub const dup2_x2: u8 = 0x5e;
	pub const swap: u8 = 0x5f;
	pub const iadd: u8 = 0x60;
	pub const ladd: u8 = 0x61;
	pub const fadd: u8 = 0x62;
	pub const dadd: u8 = 0x63;
	pub const isub: u8 = 0x64;
	pub const lsub: u8 = 0x65;
	pub const fsub: u8 = 0x66;
	pub const dsub: u8 = 0x67;
	pub const imut: u8 = 0x68;
	pub const lmut: u8 = 0x69;
	pub const fmut: u8 = 0x6a;
	pub const dmut: u8 = 0x6b;
	pub const idiv: u8 = 0x6c;
	pub const ldiv: u8 = 0x6d;
	pub const fdiv: u8 = 0x6e;
	pub const ddiv: u8 = 0x6f;
	pub const irem: u8 = 0x70;
	pub const lrem: u8 = 0x71;
	pub const frem: u8 = 0x72;
	pub const drem: u8 = 0x73;
	pub const ineg: u8 = 0x74;
	pub const lneg: u8 = 0x75;
	pub const fneg: u8 = 0x76;
	pub const dneg: u8 = 0x77;
	pub const ishl: u8 = 0x78;
	pub const lshl: u8 = 0x79;
	pub const ishr: u8 = 0x7a;
	pub const lshr: u8 = 0x7b;
	pub const iushr: u8 = 0x7c;
	pub const lushr: u8 = 0x7d;
	pub const iand: u8 = 0x7e;
	pub const land: u8 = 0x7f;
	pub const ior: u8 = 0x80;
	pub const lor: u8 = 0x81;
	pub const ixor: u8 = 0x82;
	pub const lxor: u8 = 0x83;
	pub const iinc: u8 = 0x84;
	pub const i2l: u8 = 0x85;
	pub const i2f: u8 = 0x86;
	pub const i2d: u8 = 0x87;
	pub const l2i: u8 = 0x88;
	pub const l2f: u8 = 0x89;
	pub const l2d: u8 = 0x8a;
	pub const f2i: u8 = 0x8b;
	pub const f2l: u8 = 0x8c;
	pub const f2d: u8 = 0x8d;
	pub const d2i: u8 = 0x8e;
	pub const d2l: u8 = 0x8f;
	pub const d2f: u8 = 0x90;
	pub const i2b: u8 = 0x91;
	pub const i2c: u8 = 0x92;
	pub const i2s: u8 = 0x93;
	pub const lcmp: u8 = 0x94;
	pub const fcmpl: u8 = 0x95;
	pub const fcmpg: u8 = 0x96;
	pub const dcmpl: u8 = 0x97;
	pub const dcmpg: u8 = 0x98;
	pub const ifeq: u8 = 0x99;
	pub const ifne: u8 = 0x9a;
	pub const iflt: u8 = 0x9b;
	pub const ifge: u8 = 0x9c;
	pub const ifgt: u8 = 0x9d;
	pub const ifle: u8 = 0x9e;
	pub const if_icmpeq: u8 = 0x9f;
	pub const if_icmpne: u8 = 0xa0;
	pub const if_icmplt: u8 = 0xa1;
	pub const if_icmpge: u8 = 0xa2;
	pub const if_icmpgt: u8 = 0xa3;
	pub const if_icmple: u8 = 0xa4;
	pub const if_acmpeq: u8 = 0xa5;
	pub const if_acmpne: u8 = 0xa6;
	pub const goto: u8 = 0xa7;
	pub const jsr: u8 = 0xa8;
	pub const ret: u8 = 0xa9;
	pub const tableswitch: u8 = 0xaa;
	pub const lookupswitch: u8 = 0xab;
	pub const ireturn: u8 = 0xac;
	pub const lreturn: u8 = 0xad;
	pub const freturn: u8 = 0xae;
	pub const dreturn: u8 = 0xaf;
	pub const areturn: u8 = 0xb0;
	pub const r#return: u8 = 0xb1;
	pub const getstatic: u8 = 0xb2;
	pub const putstatic: u8 = 0xb3;
	pub const getfield: u8 = 0xb4;
	pub const putfield: u8 = 0xb5;
	pub const invokevirtual: u8 = 0xb6;
	pub const invokespecial: u8 = 0xb7;
	pub const invokestatic: u8 = 0xb8;
	pub const invokeinterface: u8 = 0xb9;
	pub const invokedynamic: u8 = 0xba;
	pub const new: u8 = 0xbb;
	pub const newarray: u8 = 0xbc;
	pub const anewarray: u8 = 0xbd;
	pub const arraylength: u8 = 0xbe;
	pub const athrow: u8 = 0xbf;
	pub const checkcast: u8 = 0xc0;
	pub const instanceof: u8 = 0xc1;
	pub const monitorenter: u8 = 0xc2;
	pub const monitorexit: u8 = 0xc3;
	pub const wide: u8 = 0xc4;
	pub const multianewarray: u8 = 0xc5;
	pub const ifnull: u8 = 0xc6;
	pub const ifnonnull: u8 = 0xc7;
	pub const goto_w: u8 = 0xc8;
	pub const jsw_w: u8 = 0xc9;

	pub const breakpoint: u8 = 0xca;
	pub const impdep1: u8 = 0xfe;
	pub const impdep2: u8 = 0xff;
}

notation!(
	struct ExceptionTableEntry {
		mut start_pc: u16,
		mut end_pc: u16,
		mut handler_pc: u16,
		mut catch_type: u16,
	}
);

notation!(
	enum VerificationTypeInfo {
		tag: u8,
		Top {
			= 0 => 0,
		},
		Integer {
			= 1 => 1,
		},
		Float {
			= 2 => 2,
		},
		Null {
			= 5 => 5,
		},
		UnintializedThis {
			= 6 => 6,
		},
		Object {
			= 7 => 7,
			mut cpool_index: u16,
		},
		Unintialized {
			= 8 => 8,
			mut offset: u16,
		},
		Long {
			= 4 => 4,
		},
		Double {
			= 3 => 3,
		},
		_ {
			tag => Err(std::io::Error::other(format!("Unexpected verification type info tag {}", tag))),
		},
	}
);

notation!(
	enum StackMapFrame {
		frame_type: u8,
		SameFrame {
			= *offset_delta => frame_type @ 0..=63,
			mut offset_delta: u8 nowrite = frame_type,
		},
		SameLocals1StackItemFrame {
			= offset_delta + 64 => frame_type @ 64..=127,
			mut offset_delta: u8 nowrite = frame_type - 64,
			mut stack: VerificationTypeInfo,
		},
		SameLocals1StackItemFrameExtended {
			= 247 => 247,
			mut offset_delta: u16,
			mut stack: VerificationTypeInfo,
		},
		ChopFrame {
			= 251 - k => frame_type @ 248..=250,
			mut k: u8 nowrite = 251 - frame_type,
			mut offset_delta: u16,
		},
		SameFrameExtended {
			= 251 => 251,
			mut offset_delta: u16,
		},
		AppendFrame {
			= locals.len() + 251 => k @ 252..=254,
			mut offset_delta: u16,
			mut locals: Vec<VerificationTypeInfo> {k - 251},
		},
		FullFrame {
			= 255 => 255,
			mut offset_delta: u16,
			//number_of_locals: u16,
			mut locals: Vec<VerificationTypeInfo> [u16],
			//number_of_stack: u16,
			mut stack: Vec<VerificationTypeInfo> [u16],
		},
		_ {
			tag => Err(std::io::Error::other(format!("Unexpected stack map frame tag {}", tag))),
		},
	}
);

notation!(
	struct InnerClassesEntry {
		mut inner_class_info_index: u16,
		mut outer_class_info_index: u16,
		mut inner_name_index: u16,
		mut inner_class_access_flags: u16,
	}
);

notation!(
	struct LineNumberTableEntry {
		mut start_pc: u16,
		mut line_number: u16,
	}
);

notation!(
	struct LocalVariableTableEntry {
		mut start_pc: u16,
		mut length: u16,
		mut name_index: u16,
		mut descriptor_index: u16,
		mut index: u16,
	}
);

notation!(
	struct LocalVariableTypeTableEntry {
		mut start_pc: u16,
		mut length: u16,
		mut name_index: u16,
		mut signature_index: u16,
		mut index: u16,
	}
);

notation!(
	struct Annotation {
		mut type_index: u16,
		//num_element_value_pairs: u16,
		mut element_value_pairs: Vec<ElementValuePairsEntry> [u16],
	}
);

notation!(
	struct ElementValuePairsEntry {
		mut element_name_index: u16,
		mut value: ElementValue,
	}
);

notation!(
	enum ElementValue {
		tag: u8,
		Byte {
			= b'B' => b'B',
			mut const_value_index: u16,
		},
		Char {
			= b'C' => b'C',
			mut const_value_index: u16,
		},
		Double {
			= b'D' => b'D',
			mut const_value_index: u16,
		},
		Float {
			= b'F' => b'F',
			mut const_value_index: u16,
		},
		Integer {
			= b'I' => b'I',
			mut const_value_index: u16,
		},
		Long {
			= b'J' => b'J',
			mut const_value_index: u16,
		},
		Short {
			= b'S' => b'S',
			mut const_value_index: u16,
		},
		Boolean {
			= b'Z' => b'Z',
			mut const_value_index: u16,
		},
		String {
			= b's' => b's',
			mut const_value_index: u16,
		},
		Enum {
			= b'e' => b'e',
			mut type_name_index: u16,
			mut const_name_index: u16,
		},
		Class {
			= b'c' => b'c',
			mut class_info_index: u16,
		},
		Annotation {
			= b'@' => b'@',
			mut annotation_value: Annotation,
		},
		Array {
			= b'[' => b'[',
			//num_values: u16,
			mut values: Vec<ElementValue> [u16],
		},
		_ {
			tag => Err(std::io::Error::other(format!("Unexpected element value tag {}", tag))),
		},
	}
);

notation!(
	struct ParameterAnnotationEntry {
		//num_annotation: u16,
		mut annotations: Vec<Annotation> [u16],
	}
);

// TODO: type_annotation struct

notation!(
	struct BootstrapMethodsEntry {
		mut bootstrap_method_ref: u16,
		//num_bootstrap_arguments: u16,
		mut boostrap_arguments: Vec<u16> [u16],
	}
);

notation!(
	struct MethodParametersEntry {
		mut name_index: u16,
		mut access_flags: u16,
	}
);

notation!(
	struct ModuleRequiresEntry {
		mut requires_index: u16,
		mut requires_flags: u16,
		mut requires_version_index: u16,
	}
);

notation!(
	struct ModuleExportsEntry {
		mut exports_index: u16,
		mut exports_flags: u16,
		//exports_to_count: u16,
		mut exports_to_index: Vec<u16> [u16],
	}
);

notation!(
	struct ModuleOpensEntry {
		mut opens_index: u16,
		mut opens_flags: u16,
		//opens_to_count: u16,
		mut opens_to_index: Vec<u16> [u16],
	}
);

notation!(
	struct ModuleProvidesEntry {
		mut provides_index: u16,
		//provides_with_count: u16,
		mut provides_with_index: Vec<u16> [u16],
	}
);

notation!(
	struct RecordComponentInfo {
		mut name_index: u16,
		mut descriptor_index: u16,
		//attributes_count: u16,
		mut attributes: Vec<AttributeInfo> [u16],
	}
);


use std::fmt::{Debug, Formatter};
use anyhow::{anyhow, bail, Context, Result};
use crate::class_constants::pool;
use crate::{ClassRead, jstring};
use crate::class_constants::pool::method_handle_reference;
use crate::tree::class::ClassName;
use crate::tree::field::{ConstantValue, FieldRef};
use crate::tree::method::{MethodDescriptor, MethodRef};
use crate::tree::method::code::{ConstantDynamic, Handle, InvokeDynamic, Loadable};
use crate::tree::module::{ModuleName, PackageName};


/// A small helper struct for reading. Represents a bootstrap method, but doesn't parse the arguments yet.
#[derive(Debug, PartialEq)]
pub(crate) struct BootstrapMethodRead {
	pub(crate) handle: Handle,
	/// The arguments to the boostrap method. We store the raw constant pool indices here, since the argument of a bootstrap method may be from another
	/// bootstrap method.
	///
	/// Each index must be loadable with [`PoolRead::get_loadable`] according to the specification of the `BootstrapMethods_attribute`.
	pub(crate) arguments: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PoolEntry {
	Class { name_index: u16 },
	FieldRef { class_index: u16, name_and_type_index: u16 },
	MethodRef { class_index: u16, name_and_type_index: u16 },
	InterfaceMethodRef { class_index: u16, name_and_type_index: u16 },
	String { string_index: u16 },
	Integer { bytes: i32 },
	Float { bytes: u32 },
	Long { bytes: i64 },
	Double { bytes: u64 },
	NameAndType { name_index: u16, descriptor_index: u16 },
	Utf8 { string: String },
	MethodHandle { reference_kind: u8, reference_index: u16 },
	MethodType { descriptor_index: u16 },
	Dynamic { bootstrap_method_attribute_index: u16, name_and_type_index: u16 },
	InvokeDynamic { bootstrap_method_attribute_index: u16, name_and_type_index: u16 },
	Module { name_index: u16 },
	Package { name_index: u16 },
}

impl PoolEntry {
	fn as_utf8(&self) -> Result<&String> {
		let PoolEntry::Utf8 { string } = self else {
			bail!("pool entry not `Utf8`: {self:?}");
		};
		Ok(string)
	}

	fn as_string(&self, pool: &PoolRead) -> Result<String> {
		let PoolEntry::String { string_index } = *self else {
			bail!("pool entry not `String`: {self:?}");
		};
		pool.get_utf8(string_index)
	}

	fn as_class(&self, pool: &PoolRead) -> Result<ClassName> {
		let PoolEntry::Class { name_index } = *self else {
			bail!("pool entry not `Class`: {self:?}");
		};
		let s = pool.get_utf8(name_index)?;
		Ok(s.into())
	}

	fn as_name_and_type<'a>(&self, pool: &'a PoolRead) -> Result<(&'a String, &'a String)> {
		let PoolEntry::NameAndType { name_index, descriptor_index } = *self else {
			bail!("pool entry not `NameAndType`: {self:?}");
		};
		let name = pool.get_utf8_ref(name_index)?;
		let descriptor = pool.get_utf8_ref(descriptor_index)?;
		Ok((name, descriptor))
	}

	fn as_field_ref(&self, pool: &PoolRead) -> Result<FieldRef> {
		let PoolEntry::FieldRef { class_index, name_and_type_index } = *self else {
			bail!("pool entry not `FieldRef`: {self:?}");
		};
		let class = pool.get_class(class_index)?;
		let (name, desc) = pool.get_name_and_type(name_and_type_index)?;
		Ok(FieldRef { class, name, desc })
	}

	fn as_method_ref(&self, pool: &PoolRead) -> Result<MethodRef> {
		let PoolEntry::MethodRef { class_index, name_and_type_index } = *self else {
			bail!("pool entry not `MethodRef`: {self:?}");
		};
		let class = pool.get_class(class_index)?;
		let (name, desc) = pool.get_name_and_type(name_and_type_index)?;
		Ok(MethodRef { class, name, desc })
	}

	fn as_interface_method_ref(&self, pool: &PoolRead) -> Result<MethodRef> {
		let PoolEntry::InterfaceMethodRef { class_index, name_and_type_index } = *self else {
			bail!("pool entry not `InterfaceMethodRef`: {self:?}");
		};
		let class = pool.get_class(class_index)?;
		let (name, desc) = pool.get_name_and_type(name_and_type_index)?;
		Ok(MethodRef { class, name, desc })
	}

	/// `true` índicates it was an [`PoolEntry::InterfaceMethodRef`], `false` that it was a [`PoolEntry::MethodRef`]
	fn as_method_ref_or_interface_method_ref(&self, pool: &PoolRead) -> Result<(MethodRef, bool)> {
		let (class_index, name_and_type_index, is_interface) = match *self {
			PoolEntry::MethodRef { class_index, name_and_type_index } => (class_index, name_and_type_index, false),
			PoolEntry::InterfaceMethodRef { class_index, name_and_type_index } => (class_index, name_and_type_index, true),
			_ => bail!("pool entry not `MethodRef` or `InterfaceMethodRef`: {self:?}"),
		};

		let class = pool.get_class(class_index)?;
		let (name, desc) = pool.get_name_and_type(name_and_type_index)?;
		let method = MethodRef { class, name, desc };
		Ok((method, is_interface))
	}

	fn as_package(&self, pool: &PoolRead) -> Result<PackageName> {
		let PoolEntry::Package { name_index } = *self else {
			bail!("pool entry not `Package`: {self:?}");
		};
		let s = pool.get_utf8(name_index)?;
		Ok(s.into())
	}

	fn as_module(&self, pool: &PoolRead) -> Result<ModuleName> {
		let PoolEntry::Module { name_index } = *self else {
			bail!("pool entry not `Module`: {self:?}");
		};
		let s = pool.get_utf8(name_index)?;
		Ok(s.into())
	}

	fn as_integer(&self) -> Result<i32> {
		let PoolEntry::Integer { bytes } = *self else {
			bail!("pool entry not `Integer`: {self:?}");
		};
		Ok(bytes)
	}

	fn as_long(&self) -> Result<i64> {
		let PoolEntry::Long { bytes } = *self else {
			bail!("pool entry not `Long`: {self:?}");
		};
		Ok(bytes)
	}

	fn as_float(&self) -> Result<f32> {
		let PoolEntry::Float { bytes } = *self else {
			bail!("pool entry not `Float`: {self:?}");
		};
		Ok(f32::from_bits(bytes))
	}

	fn as_double(&self) -> Result<f64> {
		let PoolEntry::Double { bytes } = *self else {
			bail!("pool entry not `Double`: {self:?}");
		};
		Ok(f64::from_bits(bytes))
	}

	fn as_method_handle(&self, pool: &PoolRead) -> Result<Handle> {
		let PoolEntry::MethodHandle { reference_kind, reference_index } = *self else {
			bail!("pool entry not `MethodHandle`: {self:?}");
		};

		let handle = match reference_kind {
			method_handle_reference::GET_FIELD => Handle::GetField(pool.get_field_ref(reference_index)?),
			method_handle_reference::GET_STATIC => Handle::GetStatic(pool.get_field_ref(reference_index)?),
			method_handle_reference::PUT_FIELD => Handle::PutField(pool.get_field_ref(reference_index)?),
			method_handle_reference::PUT_STATIC => Handle::PutStatic(pool.get_field_ref(reference_index)?),

			//TODO?: maybe we should check the names? (on read and write, or just on write?)
			// (in the next 5: for new_INVOKE_SPECIAL: method name must be `<init>`, otherwise it MUST not be that)
			method_handle_reference::INVOKE_VIRTUAL => Handle::InvokeVirtual(pool.get_method_ref(reference_index)?),
			method_handle_reference::INVOKE_STATIC => {
				let (method_ref, is_interface) = pool.get_method_ref_or_interface_method_ref(reference_index)?;
				Handle::InvokeStatic(method_ref, is_interface)
			},
			method_handle_reference::INVOKE_SPECIAL => {
				let (method_ref, is_interface) = pool.get_method_ref_or_interface_method_ref(reference_index)?;
				Handle::InvokeSpecial(method_ref, is_interface)
			},
			method_handle_reference::NEW_INVOKE_SPECIAL => Handle::NewInvokeSpecial(pool.get_method_ref(reference_index)?),
			method_handle_reference::INVOKE_INTERFACE => Handle::InvokeInterface(pool.get_interface_method_ref(reference_index)?),
			tag => bail!("unknown `reference_kind` {tag} for `MethodHandle` pool entry"),
		};

		Ok(handle)
	}

	fn as_method_type(&self, pool: &PoolRead) -> Result<MethodDescriptor> {
		let PoolEntry::MethodType { descriptor_index } = *self else {
			bail!("pool entry not `MethodType`: {self:?}");
		};
		Ok(MethodDescriptor::from(pool.get_utf8(descriptor_index).context("while getting method type")?))
	}

	fn as_dynamic(&self, pool: &PoolRead, bootstrap_methods: &Option<Vec<BootstrapMethodRead>>) -> Result<ConstantDynamic> {
		let PoolEntry::Dynamic { bootstrap_method_attribute_index, name_and_type_index } = *self else {
			bail!("pool entry not `Dynamic`: {self:?}");
		};

		let (name, descriptor) = pool.get_name_and_type(name_and_type_index)?;

		let Some(bootstrap_methods_) = bootstrap_methods.as_ref() else {
			bail!("cannot load `Dynamic` pool entry, as there's no `BootstrapMethods` attribute")
		};
		let Some(method) = bootstrap_methods_.get(bootstrap_method_attribute_index as usize) else {
			bail!("cannot load `Dynamic` pool entry, as there's no bootstrap method at index {}", bootstrap_method_attribute_index);
		};
		let handle = method.handle.clone();
		let arguments = {
			let mut vec = Vec::with_capacity(method.arguments.len());
			for &argument in &method.arguments {
				let value = pool.get_loadable(argument, bootstrap_methods)
					.with_context(|| anyhow!("while argument for `Dynamic` at index {bootstrap_method_attribute_index:?}: {name:?} {descriptor:?} {handle:?}"))?;
				vec.push(value); // TODO: recursion
			}
			vec
		};

		Ok(ConstantDynamic { name, descriptor, handle, arguments })
	}

	fn as_invoke_dynamic(&self, pool: &PoolRead, bootstrap_methods: &Option<Vec<BootstrapMethodRead>>) -> Result<InvokeDynamic> {
		let PoolEntry::InvokeDynamic { bootstrap_method_attribute_index, name_and_type_index } = *self else {
			bail!("pool entry not `InvokeDynamic`: {self:?}");
		};

		let (name, descriptor) = pool.get_name_and_type(name_and_type_index)?;

		let Some(bootstrap_methods_) = bootstrap_methods.as_ref() else {
			bail!("cannot load `InvokeDynamic` pool entry, as there's no `BootstrapMethods` attribute");
		};
		let Some(method) = bootstrap_methods_.get(bootstrap_method_attribute_index as usize) else {
			bail!("cannot load `InvokeDynamic` pool entry, as there's no bootstrap method at index {}", bootstrap_method_attribute_index);
		};
		let handle = method.handle.clone();
		let arguments = {
			let mut vec = Vec::with_capacity(method.arguments.len());
			for &argument in &method.arguments {
				let value = pool.get_loadable(argument, bootstrap_methods)
					.with_context(|| anyhow!("while argument for `InvokeDynamic` at index {bootstrap_method_attribute_index:?}: {name:?} {descriptor:?} {handle:?}"))?;
				vec.push(value); // TODO: recursion
			}
			vec
		};

		Ok(InvokeDynamic { name, descriptor, handle, arguments })
	}

	fn as_loadable(&self, pool: &PoolRead, bootstrap_methods: &Option<Vec<BootstrapMethodRead>>) -> Result<Loadable> {
		match self {
			PoolEntry::Integer { .. } => Ok(Loadable::Integer(self.as_integer()?)),
			PoolEntry::Float { .. } => Ok(Loadable::Float(self.as_float()?)),
			PoolEntry::Long { .. } => Ok(Loadable::Long(self.as_long()?)),
			PoolEntry::Double { .. } => Ok(Loadable::Double(self.as_double()?)),
			PoolEntry::Class { .. } => Ok(Loadable::Class(self.as_class(pool)?)),
			PoolEntry::String { .. } => Ok(Loadable::String(self.as_string(pool)?)),
			PoolEntry::MethodHandle { .. } => Ok(Loadable::MethodHandle(self.as_method_handle(pool)?)),
			PoolEntry::MethodType { .. } => Ok(Loadable::MethodType(self.as_method_type(pool)?)),
			PoolEntry::Dynamic { .. } => Ok(Loadable::Dynamic(self.as_dynamic(pool, bootstrap_methods)?)),
			_ => bail!("pool entry is not loadable: {self:?}"),
		}
	}

	fn as_constant_value(&self, pool: &PoolRead) -> Result<ConstantValue> {
		match self {
			PoolEntry::Integer { .. } => Ok(ConstantValue::Integer(self.as_integer()?)),
			PoolEntry::Float { .. } => Ok(ConstantValue::Float(self.as_float()?)),
			PoolEntry::Long { .. } => Ok(ConstantValue::Long(self.as_long()?)),
			PoolEntry::Double { .. } => Ok(ConstantValue::Double(self.as_double()?)),
			PoolEntry::String { .. } => Ok(ConstantValue::String(self.as_string(pool)?)),
			_ => bail!("pool entry may not be used in a `ConstantValue` attribute: {self:?}"),
		}
	}
}

pub(crate) struct PoolRead {
	/// We store a [`None`] for the zero index, as well as for the upper indices of [`PoolEntry::Double`] and [`PoolEntry::Long`].
	inner: Vec<Option<PoolEntry>>,
}

impl PoolRead {
	/// Reads the constant pool from the specified reader. The first thing read is an `u16` specifying the size of the constant pool.
	pub(crate) fn read(reader: &mut impl ClassRead) -> Result<PoolRead> {
		let mut pool = vec![None];

		let constant_pool_count = reader.read_u16_as_usize()?;
		while pool.len() < constant_pool_count {
			match reader.read_u8()? {
				pool::UTF8 => {
					let length = reader.read_u16_as_usize()?;
					let vec = reader.read_u8_vec(length)?;
					let string = jstring::from_vec_to_string(vec)?;
					let entry = PoolEntry::Utf8 { string };
					pool.push(Some(entry));
				},
				pool::INTEGER => {
					let bytes = reader.read_i32()?;
					let entry = PoolEntry::Integer { bytes };
					pool.push(Some(entry));
				},
				pool::FLOAT => {
					let bytes = reader.read_u32()?;
					let entry = PoolEntry::Float { bytes };
					pool.push(Some(entry));
				},
				pool::LONG => {
					let bytes = reader.read_i64()?;
					let entry = PoolEntry::Long { bytes };
					pool.push(Some(entry));
					pool.push(None); // long and double take up two pool slots
				},
				pool::DOUBLE => {
					let bytes = reader.read_u64()?;
					let entry = PoolEntry::Double { bytes };
					pool.push(Some(entry));
					pool.push(None); // long and double take up two pool slots
				},
				pool::CLASS => {
					let name_index = reader.read_u16()?;
					let entry = PoolEntry::Class { name_index };
					pool.push(Some(entry));
				},
				pool::STRING => {
					let string_index = reader.read_u16()?;
					let entry = PoolEntry::String { string_index };
					pool.push(Some(entry));
				},
				pool::FIELD_REF => {
					let class_index = reader.read_u16()?;
					let name_and_type_index = reader.read_u16()?;
					let entry = PoolEntry::FieldRef { class_index, name_and_type_index };
					pool.push(Some(entry));
				},
				pool::METHOD_REF => {
					let class_index = reader.read_u16()?;
					let name_and_type_index = reader.read_u16()?;
					let entry = PoolEntry::MethodRef { class_index, name_and_type_index };
					pool.push(Some(entry));
				},
				pool::INTERFACE_METHOD_REF => {
					let class_index = reader.read_u16()?;
					let name_and_type_index = reader.read_u16()?;
					let entry = PoolEntry::InterfaceMethodRef { class_index, name_and_type_index };
					pool.push(Some(entry));
				},
				pool::NAME_AND_TYPE => {
					let name_index = reader.read_u16()?;
					let descriptor_index = reader.read_u16()?;
					let entry = PoolEntry::NameAndType { name_index, descriptor_index };
					pool.push(Some(entry));
				},
				pool::METHOD_HANDLE => {
					let reference_kind = reader.read_u8()?;
					let reference_index = reader.read_u16()?;
					let entry = PoolEntry::MethodHandle { reference_kind, reference_index };
					pool.push(Some(entry));
				},
				pool::METHOD_TYPE => {
					let descriptor_index = reader.read_u16()?;
					let entry = PoolEntry::MethodType { descriptor_index };
					pool.push(Some(entry));
				},
				pool::DYNAMIC => {
					let bootstrap_method_attribute_index = reader.read_u16()?;
					let name_and_type_index = reader.read_u16()?;
					let entry = PoolEntry::Dynamic { bootstrap_method_attribute_index, name_and_type_index };
					pool.push(Some(entry));
				},
				pool::INVOKE_DYNAMIC => {
					let bootstrap_method_attribute_index = reader.read_u16()?;
					let name_and_type_index = reader.read_u16()?;
					let entry = PoolEntry::InvokeDynamic { bootstrap_method_attribute_index, name_and_type_index };
					pool.push(Some(entry));
				},
				pool::MODULE => {
					let name_index = reader.read_u16()?;
					let entry = PoolEntry::Module { name_index };
					pool.push(Some(entry));
				},
				pool::PACKAGE => {
					let name_index = reader.read_u16()?;
					let entry = PoolEntry::Package { name_index };
					pool.push(Some(entry));
				},
				tag => bail!("unknown constant pool tag {tag} at pool index {}", pool.len()),
			};
		}

		Ok(PoolRead { inner: pool })
	}

	fn get(&self, index: u16) -> Result<&PoolEntry> {
		if let Some(Some(entry)) = self.inner.get(index as usize) {
			Ok(entry)
		} else {
			bail!("pool entry at index {index:?} is not there: either index too large or the upper half of long or double");
		}
	}

	/// Returns [`None`] if `index` is zero, otherwise returns [`Some`] of the result of the function `f`.
	pub(crate) fn get_optional<'a, T: 'a>(&'a self, index: u16, f: impl Fn(&'a PoolRead, u16) -> Result<T>) -> Result<Option<T>> {
		if index == 0 {
			Ok(None)
		} else {
			Ok(Some(f(self, index)?))
		}
	}

	pub(crate) fn get_utf8(&self, index: u16) -> Result<String> {
		self.get(index)?.as_utf8().pool_context(index).cloned()
	}

	pub(crate) fn get_utf8_ref(&self, index: u16) -> Result<&String> {
		self.get(index)?.as_utf8().pool_context(index)
	}

	pub(crate) fn get_class(&self, index: u16) -> Result<ClassName> {
		self.get(index)?.as_class(self).pool_context(index)
	}

	pub(crate) fn get_package(&self, index: u16) -> Result<PackageName> {
		self.get(index)?.as_package(self).pool_context(index)
	}

	pub(crate) fn get_module(&self, index: u16) -> Result<ModuleName> {
		self.get(index)?.as_module(self).pool_context(index)
	}

	pub(crate) fn get_name_and_type<A: From<String>, B: From<String>>(&self, index: u16) -> Result<(A, B)> {
		self.get(index)?.as_name_and_type(self).pool_context(index).map(|(a, b)| (A::from(a.clone()), B::from(b.clone())))
	}

	fn get_name_and_type_ref(&self, index: u16) -> Result<(&String, &String)> {
		self.get(index)?.as_name_and_type(self).pool_context(index)
	}

	pub(crate) fn get_field_ref(&self, index: u16) -> Result<FieldRef> {
		self.get(index)?.as_field_ref(self).pool_context(index)
	}

	pub(crate) fn get_method_ref(&self, index: u16) -> Result<MethodRef> {
		self.get(index)?.as_method_ref(self).pool_context(index)
	}

	pub(crate) fn get_interface_method_ref(&self, index: u16) -> Result<MethodRef> {
		self.get(index)?.as_interface_method_ref(self).pool_context(index)
	}

	/// `true` índicates it was an [`PoolEntry::InterfaceMethodRef`], `false` that it was a [`PoolEntry::MethodRef`]
	pub(crate) fn get_method_ref_or_interface_method_ref(&self, index: u16) -> Result<(MethodRef, bool)> {
		self.get(index)?.as_method_ref_or_interface_method_ref(self).pool_context(index)
	}

	pub(crate) fn get_integer(&self, index: u16) -> Result<i32> {
		self.get(index)?.as_integer().pool_context(index)
	}
	pub(crate) fn get_integer_as_byte(&self, index: u16) -> Result<i8> {
		let integer = self.get_integer(index)?;
		Ok(integer as i8)
	}
	pub(crate) fn get_integer_as_char(&self, index: u16) -> Result<u16> {
		let integer = self.get_integer(index)?;
		Ok(integer as u16)
	}
	pub(crate) fn get_integer_as_short(&self, index: u16) -> Result<i16> {
		let integer = self.get_integer(index)?;
		Ok(integer as i16)
	}
	pub(crate) fn get_integer_as_boolean(&self, index: u16) -> Result<bool> {
		Ok(self.get_integer(index)? != 0)
	}
	pub(crate) fn get_double(&self, index: u16) -> Result<f64> {
		self.get(index)?.as_double().pool_context(index)
	}
	pub(crate) fn get_float(&self, index: u16) -> Result<f32> {
		self.get(index)?.as_float().pool_context(index)
	}
	pub(crate) fn get_long(&self, index: u16) -> Result<i64> {
		self.get(index)?.as_long().pool_context(index)
	}

	/// Gets a loadable constant pool entry.
	///
	/// Loadable entries are: [`PoolEntry::Integer`], [`PoolEntry::Float`], [`PoolEntry::Long`], [`PoolEntry::Double`], [`PoolEntry::Class`],
	/// [`PoolEntry::String`], [`PoolEntry::MethodHandle`], [`PoolEntry::MethodType`] and [`PoolEntry::Dynamic`].
	///
	/// These are collected in the [`Loadable`] type.
	pub(crate) fn get_loadable(&self, index: u16, bootstrap_methods: &Option<Vec<BootstrapMethodRead>>) -> Result<Loadable> {
		self.get(index)?.as_loadable(self, bootstrap_methods).pool_context(index)
	}

	pub(crate) fn get_constant_value(&self, index: u16) -> Result<ConstantValue> {
		self.get(index)?.as_constant_value(self).pool_context(index)
	}

	pub(crate) fn get_method_handle(&self, index: u16) -> Result<Handle> {
		self.get(index)?.as_method_handle(self).pool_context(index)
	}

	pub(crate) fn get_invoke_dynamic(&self, index: u16, bootstrap_methods: &Option<Vec<BootstrapMethodRead>>) -> Result<InvokeDynamic> {
		self.get(index)?.as_invoke_dynamic(self, bootstrap_methods).pool_context(index)
	}
}

/// Tiny helper trait for adding pool indices to errors.
trait PoolContext {
	fn pool_context(self, index: u16) -> Self;
}
impl<T> PoolContext for Result<T> {
	fn pool_context(self, index: u16) -> Self {
		self.with_context(|| anyhow!("while getting pool index {index}"))
	}
}

impl Debug for PoolRead {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut d = f.debug_map();

		for (i, entry) in self.inner.iter().enumerate() {
			if i == 0 { continue; }
			if let Some(entry) = entry {
				let dbg = || -> Result<_> { Ok(match entry {
					PoolEntry::Class { .. } => {
						format!("Class: {}", entry.as_class(self)?)
					},
					&PoolEntry::FieldRef { class_index, name_and_type_index } => {
						let class = self.get_class(class_index)?;
						let (name, descriptor) = self.get_name_and_type_ref(name_and_type_index)?;

						format!("FieldRef: {class}.{name}:{descriptor}")
					},
					&PoolEntry::MethodRef { class_index, name_and_type_index } => {
						let class = self.get_class(class_index)?;
						let (name, descriptor) = self.get_name_and_type_ref(name_and_type_index)?;

						format!("MethodRef: {class}.{name}:{descriptor}")
					},
					&PoolEntry::InterfaceMethodRef { class_index, name_and_type_index } => {
						let class = self.get_class(class_index)?;
						let (name, descriptor) = self.get_name_and_type_ref(name_and_type_index)?;

						format!("InterfaceMethodRef: {class}.{name}:{descriptor}")
					},
					&PoolEntry::String { string_index } => {
						format!("String: {}", self.get_utf8_ref(string_index)?)
					},
					PoolEntry::Integer { .. } => {
						format!("Integer: {}", entry.as_integer()?)
					},
					PoolEntry::Float { .. } => {
						format!("Float: {}", entry.as_float()?)
					},
					PoolEntry::Long { .. } => {
						format!("Long: {}", entry.as_long()?)
					},
					PoolEntry::Double { .. } => {
						format!("Double: {}", entry.as_double()?)
					},
					PoolEntry::NameAndType { .. } => {
						let (name, descriptor) = entry.as_name_and_type(self)?;
						format!("NameAndType: {name}:{descriptor}")
					},
					PoolEntry::Utf8 { ref string } => {
						format!("Utf8: {string}")
					},
					PoolEntry::MethodHandle { reference_index, reference_kind } => {
						format!("MethodHandle: reference index: {reference_index}, reference kind: {reference_kind}")
					},
					PoolEntry::MethodType { descriptor_index } => {
						format!("MethodType: descriptor index: {descriptor_index}")
					},
					PoolEntry::Dynamic { name_and_type_index, bootstrap_method_attribute_index } => {
						format!("Dynamic: boostrap method attribute index: {bootstrap_method_attribute_index}, name and type index: {name_and_type_index}")
					},
					PoolEntry::InvokeDynamic { name_and_type_index, bootstrap_method_attribute_index } => {
						format!("InvokeDynamic: bootstrap method attribute index: {bootstrap_method_attribute_index}, name and type index: {name_and_type_index}")
					},
					PoolEntry::Module { .. } => {
						format!("Module: {}", entry.as_module(self)?.as_str())
					},
					PoolEntry::Package { .. } => {
						format!("Package: {}", entry.as_package(self)?.as_str())
					},
				}) };
				match dbg() {
					Ok(dbg) => d.entry(&i, &dbg),
					Err(e) => d.entry(&i, &e),
				};
			}
		}

		d.finish()?;
		Ok(())
	}
}

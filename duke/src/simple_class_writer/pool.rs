use std::collections::hash_map::Entry;
use std::collections::HashMap;
use anyhow::{anyhow, Context, Result};
use crate::class_constants::pool;
use crate::{ClassWrite, jstring};
use crate::class_constants::pool::method_handle_reference;
use crate::tree::class::ClassName;
use crate::tree::field::{ConstantValue, FieldRef};
use crate::tree::method::{MethodDescriptor, MethodRef};
use crate::tree::method::code::{ConstantDynamic, Handle, InvokeDynamic, Loadable};
use crate::tree::module::{ModuleName, PackageName};

/// A small helper struct for writing the bootstrap methods attribute. Represents a bootstrap method, but with arguments as pool indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BootstrapMethodWrite<'a> {
	pub(crate) handle: &'a Handle,
	/// The arguments to the boostrap method. We store the raw constant pool indices here.
	///
	/// Each index is created from a call to [`PoolWrite::put_loadable`].
	pub(crate) arguments: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PoolEntry<'a> {
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
	Utf8 { string: &'a str },
	MethodHandle { reference_kind: u8, reference_index: u16 },
	MethodType { descriptor_index: u16 },
	Dynamic { bootstrap_method_attribute_index: u16, name_and_type_index: u16 },
	InvokeDynamic { bootstrap_method_attribute_index: u16, name_and_type_index: u16 },
	Module { name_index: u16 },
	Package { name_index: u16 },
}

impl PoolEntry<'_> {
	fn from_utf8(string: &str) -> PoolEntry {
		PoolEntry::Utf8 { string }
	}

	fn from_string<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b str) -> Result<Self> {
		Ok(PoolEntry::String { string_index: pool.put_utf8(value)? })
	}

	fn from_class<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b ClassName) -> Result<Self> {
		Ok(PoolEntry::Class { name_index: pool.put_utf8(value.as_str())? })
	}

	fn from_name_and_type<'a, 'b: 'a, 'c: 'a>(pool: &mut PoolWrite<'a>, name: &'b str, descriptor: &'c str) -> Result<Self> {
		Ok(PoolEntry::NameAndType {
			name_index: pool.put_utf8(name)?,
			descriptor_index: pool.put_utf8(descriptor)?,
		})
	}

	fn from_field_ref<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b FieldRef) -> Result<Self> {
		Ok(PoolEntry::FieldRef {
			class_index: pool.put_class(&value.class)?,
			name_and_type_index: pool.put_name_and_type(&value.name, &value.desc)?,
		})
	}

	fn from_method_ref<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b MethodRef) -> Result<Self> {
		Ok(PoolEntry::MethodRef {
			class_index: pool.put_class(&value.class)?,
			name_and_type_index: pool.put_name_and_type(&value.name, &value.desc)?,
		})
	}

	fn from_interface_method_ref<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b MethodRef) -> Result<Self> {
		Ok(PoolEntry::InterfaceMethodRef {
			class_index: pool.put_class(&value.class)?,
			name_and_type_index: pool.put_name_and_type(&value.name, &value.desc)?,
		})
	}

	/// `true` índicates it's an [`PoolEntry::InterfaceMethodRef`], `false` that it's a [`PoolEntry::MethodRef`]
	fn from_method_ref_or_interface_method_ref<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: (&'b MethodRef, bool)) -> Result<Self> {
		let (method, is_interface) = value;
		let class_index = pool.put_class(&method.class)?;
		let name_and_type_index = pool.put_name_and_type(&method.name, &method.desc)?;
		if is_interface {
			Ok(PoolEntry::InterfaceMethodRef { class_index, name_and_type_index })
		} else {
			Ok(PoolEntry::MethodRef { class_index, name_and_type_index })
		}
	}

	fn from_package<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b PackageName) -> Result<Self> {
		Ok(PoolEntry::Package { name_index: pool.put_utf8(value.as_str())? })
	}

	fn from_module<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b ModuleName) -> Result<Self> {
		Ok(PoolEntry::Module { name_index: pool.put_utf8(value.as_str())? })
	}

	fn from_integer(value: i32) -> Self {
		PoolEntry::Integer { bytes: value }
	}

	fn from_long(value: i64) -> Self {
		PoolEntry::Long { bytes: value }
	}

	fn from_float(value: f32) -> Self {
		PoolEntry::Float { bytes: value.to_bits() }
	}

	fn from_double(value: f64) -> Self {
		PoolEntry::Double { bytes: value.to_bits() }
	}

	fn from_method_handle<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b Handle) -> Result<Self> {
		let (reference_kind, reference_index) = match value {
			Handle::GetField(field) => (method_handle_reference::GET_FIELD, pool.put_field_ref(field)?),
			Handle::GetStatic(field) => (method_handle_reference::GET_STATIC, pool.put_field_ref(field)?),
			Handle::PutField(field) => (method_handle_reference::PUT_FIELD, pool.put_field_ref(field)?),
			Handle::PutStatic(field) => (method_handle_reference::PUT_STATIC, pool.put_field_ref(field)?),
			Handle::InvokeVirtual(method) => (method_handle_reference::INVOKE_VIRTUAL, pool.put_method_ref(method)?),
			&Handle::InvokeStatic(ref method, interface) => (method_handle_reference::INVOKE_STATIC, pool.put_method_ref_or_interface_method_ref((method, interface))?),
			&Handle::InvokeSpecial(ref method, interface) => (method_handle_reference::INVOKE_SPECIAL, pool.put_method_ref_or_interface_method_ref((method, interface))?),
			Handle::NewInvokeSpecial(method) => (method_handle_reference::NEW_INVOKE_SPECIAL, pool.put_method_ref(method)?),
			Handle::InvokeInterface(method) => (method_handle_reference::INVOKE_INTERFACE, pool.put_interface_method_ref(method)?)
		};
		Ok(PoolEntry::MethodHandle { reference_kind, reference_index })
	}

	fn from_method_type<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b MethodDescriptor) -> Result<Self> {
		Ok(PoolEntry::MethodType { descriptor_index: pool.put_utf8(value.as_str())? })
	}

	fn from_dynamic<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b ConstantDynamic) -> Result<Self> {
		let name_and_type_index = pool.put_name_and_type(&value.name, &value.descriptor)?;
		let bootstrap_method_attribute_index = pool.put_bootstrap_method(&value.handle, &value.arguments)?;

		Ok(PoolEntry::Dynamic { bootstrap_method_attribute_index, name_and_type_index })
	}

	fn from_invoke_dynamic<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b InvokeDynamic) -> Result<Self> {
		let name_and_type_index = pool.put_name_and_type(&value.name, &value.descriptor)?;
		let bootstrap_method_attribute_index = pool.put_bootstrap_method(&value.handle, &value.arguments)?;

		Ok(PoolEntry::InvokeDynamic { bootstrap_method_attribute_index, name_and_type_index })
	}

	fn from_loadable<'a, 'b: 'a>(pool: &mut PoolWrite<'a>,value: &'b Loadable) -> Result<Self> {
		match *value {
			Loadable::Integer(value) => Ok(PoolEntry::from_integer(value)),
			Loadable::Float(value) => Ok(PoolEntry::from_float(value)),
			Loadable::Long(value) => Ok(PoolEntry::from_long(value)),
			Loadable::Double(value) => Ok(PoolEntry::from_double(value)),
			Loadable::Class(ref value) => PoolEntry::from_class(pool, value),
			Loadable::String(ref value) => PoolEntry::from_string(pool, value),
			Loadable::MethodHandle(ref value) => PoolEntry::from_method_handle(pool, value),
			Loadable::MethodType(ref value) => PoolEntry::from_method_type(pool, value),
			Loadable::Dynamic(ref value) => PoolEntry::from_dynamic(pool, value), }
	}

	fn from_constant_value<'a, 'b: 'a>(pool: &mut PoolWrite<'a>, value: &'b ConstantValue) -> Result<Self> {
		match *value {
			ConstantValue::Integer(value) => Ok(PoolEntry::from_integer(value)),
			ConstantValue::Float(value) => Ok(PoolEntry::from_float(value)),
			ConstantValue::Long(value) => Ok(PoolEntry::from_long(value)),
			ConstantValue::Double(value) => Ok(PoolEntry::from_double(value)),
			ConstantValue::String(ref value) => Ok(PoolEntry::from_string(pool, value)?),
		}
	}
}

#[derive(Debug)]
pub(crate) struct PoolWrite<'a> {
	/// The value written as `constant_pool_count` in the class file.
	///
	/// We start at `1` and increment this twice for [`PoolEntry::Double`] and [`PoolEntry::Long`].
	count: u16,
	/// All actually used pool entries. Note that there can be pool entries with indices larger than what you'd expect from the position in this vec,
	/// since we skip indices for the zeroth entry and for the upper indices of long and double.
	inner: Vec<PoolEntry<'a>>,
	/// A [`HashMap`] to check if we already have such an item. Maps an [`PoolEntry`] to the corresponding index.
	map: HashMap<PoolEntry<'a>, u16>,

	/// For writing the bootstrap methods attribute.
	pub(crate) bootstrap_methods: Option<(Vec<BootstrapMethodWrite<'a>>, HashMap<BootstrapMethodWrite<'a>, u16>)>,
}

impl PoolWrite<'_> {
	/// Creates a pool in a ready-to-write state. This means that the element at index zero is occupied.
	/// We never write this item when writing out the pool to a writer.
	pub(crate) fn new<'a>() -> PoolWrite<'a> {
		PoolWrite {
			count: 1, // first index given out is 1
			inner: Vec::new(),
			map: HashMap::new(),

			bootstrap_methods: None,
		}
	}

	/// Writes the constant pool to the specified writer. The first thing written is an `u16` specifying the size of the constant pool.
	pub(crate) fn write(self, writer: &mut impl ClassWrite) -> Result<()> {
		writer.write_u16(self.count)?;

		for entry in self.inner {
			match entry {
				PoolEntry::Utf8 { string } => {
					writer.write_u8(pool::UTF8)?;
					let vec = jstring::from_string_to_vec(string);
					writer.write_usize_as_u16(vec.len()).context("failed to write length of string")?;
					writer.write_u8_slice(&vec)?;
				},
				PoolEntry::Integer { bytes} => {
					writer.write_u8(pool::INTEGER)?;
					writer.write_i32(bytes)?;
				},
				PoolEntry::Float { bytes } => {
					writer.write_u8(pool::FLOAT)?;
					writer.write_u32(bytes)?;
				},
				PoolEntry::Long { bytes } => {
					writer.write_u8(pool::LONG)?;
					writer.write_i64(bytes)?;
				},
				PoolEntry::Double { bytes } => {
					writer.write_u8(pool::DOUBLE)?;
					writer.write_u64(bytes)?;
				},
				PoolEntry::Class { name_index } => {
					writer.write_u8(pool::CLASS)?;
					writer.write_u16(name_index)?;
				},
				PoolEntry::String { string_index } => {
					writer.write_u8(pool::STRING)?;
					writer.write_u16(string_index)?;
				},
				PoolEntry::FieldRef { class_index, name_and_type_index } => {
					writer.write_u8(pool::FIELD_REF)?;
					writer.write_u16(class_index)?;
					writer.write_u16(name_and_type_index)?;
				},
				PoolEntry::MethodRef { class_index, name_and_type_index } => {
					writer.write_u8(pool::METHOD_REF)?;
					writer.write_u16(class_index)?;
					writer.write_u16(name_and_type_index)?;
				},
				PoolEntry::InterfaceMethodRef { class_index, name_and_type_index } => {
					writer.write_u8(pool::INTERFACE_METHOD_REF)?;
					writer.write_u16(class_index)?;
					writer.write_u16(name_and_type_index)?;
				},
				PoolEntry::NameAndType { name_index, descriptor_index } => {
					writer.write_u8(pool::NAME_AND_TYPE)?;
					writer.write_u16(name_index)?;
					writer.write_u16(descriptor_index)?;
				},
				PoolEntry::MethodHandle { reference_kind, reference_index } => {
					writer.write_u8(pool::METHOD_HANDLE)?;
					writer.write_u8(reference_kind)?;
					writer.write_u16(reference_index)?;
				},
				PoolEntry::MethodType { descriptor_index } => {
					writer.write_u8(pool::METHOD_TYPE)?;
					writer.write_u16(descriptor_index)?;
				},
				PoolEntry::Dynamic { bootstrap_method_attribute_index, name_and_type_index } => {
					writer.write_u8(pool::DYNAMIC)?;
					writer.write_u16(bootstrap_method_attribute_index)?;
					writer.write_u16(name_and_type_index)?;
				},
				PoolEntry::InvokeDynamic { bootstrap_method_attribute_index, name_and_type_index } => {
					writer.write_u8(pool::INVOKE_DYNAMIC)?;
					writer.write_u16(bootstrap_method_attribute_index)?;
					writer.write_u16(name_and_type_index)?;
				},
				PoolEntry::Module { name_index } => {
					writer.write_u8(pool::MODULE)?;
					writer.write_u16(name_index)?;
				},
				PoolEntry::Package { name_index } => {
					writer.write_u8(pool::PACKAGE)?;
					writer.write_u16(name_index)?;
				},
			}
		}

		Ok(())
	}
}

impl<'a> PoolWrite<'a> {
	fn put<'b: 'a>(&mut self, entry: PoolEntry<'b>) -> Result<u16> {
		match self.map.entry(entry) {
			Entry::Occupied(entry) => Ok(*entry.get()),
			Entry::Vacant(entry) => {
				let index = self.count;

				let inc = if matches!(entry.key(), PoolEntry::Long { .. } | PoolEntry::Double { .. }) {
					2 // long and double take up two pool slots
				} else {
					1
				};
				self.count = self.count.checked_add(inc)
					.with_context(|| anyhow!("pool count overflowed while adding pool entry {:?} to pool at index {}", entry.key(), index))?;

				self.inner.push(entry.key().clone());
				entry.insert(index);

				Ok(index)
			},
		}
	}

	/// Puts an entry into the `BootstrapMethods` attribute.
	///
	/// Returns an index from inside that attribute.
	fn put_bootstrap_method<'b: 'a, 'c: 'a>(&mut self, handle: &'b Handle, arguments: &'c Vec<Loadable>) -> Result<u16> {
		let mut vec = Vec::with_capacity(arguments.len());
		for argument in arguments {
			vec.push(self.put_loadable(argument)?);
		}
		let arguments = vec;

		let entry = BootstrapMethodWrite { handle, arguments };

		let (vec, map) = self.bootstrap_methods.get_or_insert_with(|| (Vec::new(), HashMap::new()));

		match map.entry(entry) {
			Entry::Occupied(entry) => Ok(*entry.get()),
			Entry::Vacant(entry) => {
				let index = vec.len().try_into()
					.with_context(|| anyhow!("bootstrap methods attribute count overflowed while adding bootstrap method {:?}", entry.key()))?;

				vec.push(entry.key().clone());
				entry.insert(index);

				Ok(index)
			},
		}
	}

	/// Returns zero if the value is [`None`], otherwise returns the result of the function `f` called on the value of [`Some`].
	pub(crate) fn put_optional<'b, T: ?Sized>(&mut self, value: Option<&'b T>, f: impl Fn(&mut PoolWrite<'a>, &'b T) -> Result<u16>) -> Result<u16> {
		if let Some(value) = value {
			f(self, value)
		} else {
			Ok(0)
		}
	}

	pub(crate) fn put_utf8<'b: 'a>(&mut self, value: &'b str) -> Result<u16> {
		self.put(PoolEntry::from_utf8(value))
	}

	pub(crate) fn put_class<'b: 'a>(&mut self, value: &'b ClassName) -> Result<u16> {
		let entry = PoolEntry::from_class(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_package<'b: 'a>(&mut self, value: &'b PackageName) -> Result<u16> {
		let entry = PoolEntry::from_package(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_module<'b: 'a>(&mut self, value: &'b ModuleName) -> Result<u16> {
		let entry = PoolEntry::from_module(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_name_and_type<'t, 'u, T, U>(&mut self, t: &'t T, u: &'u U) -> Result<u16>
		where
			't: 'a,
			'u: 'a,
			T: AsRef<str>,
			U: AsRef<str>,
	{
		let entry = PoolEntry::from_name_and_type(self, t.as_ref(), u.as_ref())?;
		self.put(entry)
	}

	pub(crate) fn put_field_ref<'b: 'a>(&mut self, value: &'b FieldRef) -> Result<u16> {
		let entry = PoolEntry::from_field_ref(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_method_ref<'b: 'a>(&mut self, value: &'b MethodRef) -> Result<u16> {
		let entry = PoolEntry::from_method_ref(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_interface_method_ref<'b: 'a>(&mut self, value: &'b MethodRef) -> Result<u16> {
		let entry = PoolEntry::from_interface_method_ref(self, value)?;
		self.put(entry)
	}

	/// `true` índicates it's an [`PoolEntry::InterfaceMethodRef`], `false` that it's a [`PoolEntry::MethodRef`]
	pub(crate) fn put_method_ref_or_interface_method_ref<'b: 'a>(&mut self, value: (&'b MethodRef, bool)) -> Result<u16> {
		let entry = PoolEntry::from_method_ref_or_interface_method_ref(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_integer(&mut self, value: i32) -> Result<u16> {
		self.put(PoolEntry::from_integer(value))
	}
	pub(crate) fn put_byte_as_integer(&mut self, value: i8) -> Result<u16> {
		self.put_integer(value as i32)
	}
	pub(crate) fn put_char_as_integer(&mut self, value: u16) -> Result<u16> {
		self.put_integer(value as i32)
	}
	pub(crate) fn put_short_as_integer(&mut self, value: i16) -> Result<u16> {
		self.put_integer(value as i32)
	}
	pub(crate) fn put_boolean_as_integer(&mut self, value: bool) -> Result<u16> {
		self.put(PoolEntry::from_integer(i32::from(value)))
	}
	pub(crate) fn put_double(&mut self, value: f64) -> Result<u16> {
		self.put(PoolEntry::from_double(value))
	}
	pub(crate) fn put_float(&mut self, value: f32) -> Result<u16> {
		self.put(PoolEntry::from_float(value))
	}
	pub(crate) fn put_long(&mut self, value: i64) -> Result<u16> {
		self.put(PoolEntry::from_long(value))
	}

	/// Stores a loadable constant pool entry.
	///
	/// Loadable entries are: [`PoolEntry::Integer`], [`PoolEntry::Float`], [`PoolEntry::Long`], [`PoolEntry::Double`], [`PoolEntry::Class`],
	/// [`PoolEntry::String`], [`PoolEntry::MethodHandle`], [`PoolEntry::MethodType`] and [`PoolEntry::Dynamic`].
	///
	/// These are collected in the [`Loadable`] type.
	pub(crate) fn put_loadable<'b: 'a>(&mut self, value: &'b Loadable) -> Result<u16> {
		let entry = PoolEntry::from_loadable(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_constant_value<'b: 'a>(&mut self, value: &'b ConstantValue) -> Result<u16> {
		let entry = PoolEntry::from_constant_value(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_method_handle<'b: 'a>(&mut self, value: &'b Handle) -> Result<u16> {
		let entry = PoolEntry::from_method_handle(self, value)?;
		self.put(entry)
	}

	pub(crate) fn put_invoke_dynamic<'b: 'a>(&mut self, value: &'b InvokeDynamic) -> Result<u16> {
		let entry = PoolEntry::from_invoke_dynamic(self, value)?;
		self.put(entry)
	}
}

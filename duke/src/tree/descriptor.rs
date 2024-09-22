use std::iter::Peekable;
use anyhow::{anyhow, bail, Context, Result};
use java_string::{Chars, JavaCodePoint, JavaStr, JavaString};
use crate::macros::make_string_str_like;
use crate::tree::class::{ClassName, ClassNameSlice};
use crate::tree::field::{FieldDescriptor, FieldDescriptorSlice};
use crate::tree::method::{MethodDescriptor, MethodDescriptorSlice};

/// Represents a type.
///
/// In case of an array, use the [`Type::Array`] variant.
///
/// ```
/// use duke::tree::descriptor::{ArrayType, Type};
///
/// // the type of a java `int`
/// let int_type = Type::I;
///
/// // the type of a java `int[][]`
/// let int_array_type = Type::Array(2, ArrayType::I);
///
/// assert_ne!(int_type, int_array_type);
/// ```
///
/// Note: you should never construct the [`Type::Array`] variant with a dimension
/// of zero, as the [`Eq`] and [`PartialEq`] implementations don't respect that:
/// ```
/// use duke::tree::descriptor::{ArrayType, Type};
/// let double_type = Type::D;
/// let double_array_zero_type = Type::Array(0, ArrayType::D); // a "D" with zero "[" in front of it
/// assert_ne!(double_type, double_array_zero_type);
/// ```
///
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Type {
	/// A `byte`. In rust, this is a `i8`.
	B,
	/// A `char`.
	C,
	/// A `double`. In rust, this is a `f64`.
	D,
	/// A `float`. In rust, this is a `f32`.
	F,
	/// An `int`. In rust, this is a `i32`.
	I,
	/// A `long`. In rust, this is a `i64`.
	J,
	/// A `short`. In rust, this is a `i16`.
	S,
	/// A `boolean`. In rust, this is a `bool`.
	Z,
	/// An instance of the class specified by [`ClassName`].
	Object(ClassName),
	/// An array type, represented by the dimension and the inner [`ArrayType`].
	Array(u8, ArrayType),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ArrayType {
	B,
	C,
	D,
	F,
	I,
	J,
	S,
	Z,
	Object(ClassName),
}

// The grammar for descriptors is:
//   FieldDescriptor:
//     FieldType
//
//   MethodDescriptor:
//     "(" FieldType* ")" ReturnDescriptor
//
//   ReturnDescriptor:
//     FieldType | "V"
//
//   FieldType:
//     "B" | "C" | "D" | "F" | "I" | "J" | "S" | "Z" |
//     "L" ClassName ";" |
//     "[" FieldType
fn read_field_type(chars: &mut Peekable<Chars>) -> Result<Type> {
	const B: JavaCodePoint = JavaCodePoint::from_char('B');
	const C: JavaCodePoint = JavaCodePoint::from_char('C');
	const D: JavaCodePoint = JavaCodePoint::from_char('D');
	const F: JavaCodePoint = JavaCodePoint::from_char('F');
	const I: JavaCodePoint = JavaCodePoint::from_char('I');
	const J: JavaCodePoint = JavaCodePoint::from_char('J');
	const L: JavaCodePoint = JavaCodePoint::from_char('L');
	const S: JavaCodePoint = JavaCodePoint::from_char('S');
	const Z: JavaCodePoint = JavaCodePoint::from_char('Z');

	let mut array_dimension = 0;
	while chars.next_if_eq(&'[').is_some() {
		array_dimension += 1;
	}
	// TODO: only valid if array_dimension is fitting into u8 and not overflowing (the addition above)

	if array_dimension == 0 {
		let char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor"))?;
		let descriptor = match char {
			B => Type::B,
			C => Type::C,
			D => Type::D,
			F => Type::F,
			I => Type::I,
			J => Type::J,
			S => Type::S,
			Z => Type::Z,
			L => {
				let mut s = JavaString::new();

				let mut char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor"))?;
				while char != ';' {
					s.push_java(char);

					char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor"))?;
				}

				let class_name = unsafe { ClassName::from_inner_unchecked(s) };
				Type::Object(class_name)
			},
			x => {
				bail!("unexpected char {x:?} in descriptor");
			}
		};

		Ok(descriptor)
	} else {
		let char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor"))?;
		let descriptor = match char {
			B => Type::Array(array_dimension, ArrayType::B),
			C => Type::Array(array_dimension, ArrayType::C),
			D => Type::Array(array_dimension, ArrayType::D),
			F => Type::Array(array_dimension, ArrayType::F),
			I => Type::Array(array_dimension, ArrayType::I),
			J => Type::Array(array_dimension, ArrayType::J),
			S => Type::Array(array_dimension, ArrayType::S),
			Z => Type::Array(array_dimension, ArrayType::Z),
			L => {
				let mut s = JavaString::new();

				let mut char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor"))?;
				while char != ';' {
					s.push_java(char);

					char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor"))?;
				}

				let class_name = unsafe { ClassName::from_inner_unchecked(s) };
				Type::Array(array_dimension, ArrayType::Object(class_name))
			},
			x => {
				bail!("unexpected char {x:?} in descriptor");
			}
		};

		Ok(descriptor)
	}
}

fn write_field_type(t: &Type, string: &mut JavaString) {
	match t {
		Type::B => string.push('B'),
		Type::C => string.push('C'),
		Type::D => string.push('D'),
		Type::F => string.push('F'),
		Type::I => string.push('I'),
		Type::J => string.push('J'),
		Type::S => string.push('S'),
		Type::Z => string.push('Z'),
		Type::Object(class_name) => {
			assert!(!class_name.as_inner().starts_with('['));
			string.push('L');
			string.push_java_str(class_name.as_inner());
			string.push(';');
		},
		Type::Array(array_dimension, array_type) => {
			for _ in 0..*array_dimension {
				string.push('[');
			}
			match array_type {
				ArrayType::B => string.push('B'),
				ArrayType::C => string.push('C'),
				ArrayType::D => string.push('D'),
				ArrayType::F => string.push('F'),
				ArrayType::I => string.push('I'),
				ArrayType::J => string.push('J'),
				ArrayType::S => string.push('S'),
				ArrayType::Z => string.push('Z'),
				ArrayType::Object(class_name) => {
					assert!(!class_name.as_inner().starts_with('['));
					string.push('L');
					string.push_java_str(class_name.as_inner());
					string.push(';');
				},
			}
		},
	}
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedFieldDescriptor(pub Type);

impl FieldDescriptorSlice {
	/// Attempts to parse a field descriptor.
	///
	/// A field descriptor is defined by the [grammar](https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html#jvms-4.3.2) in the
	/// Java Virtual Machine Specification.
	///
	/// The inverse of this function is [`ParsedFieldDescriptor::write`].
	///
	/// # Examples
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use duke::tree::class::ClassName;
	/// use duke::tree::descriptor::{ArrayType, ParsedFieldDescriptor, Type};
	/// use duke::tree::field::FieldDescriptorSlice;
	///
	/// assert_eq!(
	///     unsafe { FieldDescriptorSlice::from_inner_unchecked("I".into()) }.parse().unwrap(),
	///     ParsedFieldDescriptor(Type::I)
	/// );
	/// assert_eq!(
	///     unsafe { FieldDescriptorSlice::from_inner_unchecked("Ljava/lang/Object;".into()) }.parse().unwrap(),
	///     ParsedFieldDescriptor(Type::Object(ClassName::JAVA_LANG_OBJECT.to_owned()))
	/// );
	/// assert_eq!(
	///     unsafe { FieldDescriptorSlice::from_inner_unchecked("[[[D".into()) }.parse().unwrap(),
	///     ParsedFieldDescriptor(Type::Array(3, ArrayType::D))
	/// );
	///
	/// let double_array = unsafe { FieldDescriptorSlice::from_inner_unchecked("[[[D".into()) };
	/// assert_eq!(double_array, double_array.parse().unwrap().write());
	/// ```
	pub fn parse(&self) -> Result<ParsedFieldDescriptor> {
		let mut chars = self.as_inner().chars().peekable();

		let descriptor = read_field_type(&mut chars)
			.with_context(|| anyhow!("failed to read field descriptor {self:?}"))?;

		if chars.peek().is_some() {
			bail!("expected end of field descriptor {self:?}, got {:?} remaining", JavaString::from_iter(chars));
		}

		Ok(ParsedFieldDescriptor(descriptor))
	}
}

impl ParsedFieldDescriptor {
	/// Writes a field descriptor.
	///
	/// The inverse of this function is [`FieldDescriptorSlice::parse`].
	///
	/// # Examples
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use duke::tree::class::ClassName;
	/// use duke::tree::descriptor::{ArrayType, ParsedFieldDescriptor, Type};
	/// use duke::tree::field::FieldDescriptorSlice;
	///
	/// assert_eq!(
	///     ParsedFieldDescriptor(Type::I).write(),
	///     unsafe { FieldDescriptorSlice::from_inner_unchecked("I".into()) },
	/// );
	/// assert_eq!(
	///     ParsedFieldDescriptor(Type::Object(ClassName::JAVA_LANG_OBJECT.to_owned())).write(),
	///     unsafe { FieldDescriptorSlice::from_inner_unchecked("Ljava/lang/Object;".into()) },
	/// );
	/// assert_eq!(
	///     ParsedFieldDescriptor(Type::Array(3, ArrayType::D)).write(),
	///     unsafe { FieldDescriptorSlice::from_inner_unchecked("[[[D".into()) },
	/// );
	///
	/// let double_array = ParsedFieldDescriptor(Type::Array(3, ArrayType::D));
	/// assert_eq!(double_array, double_array.write().parse().unwrap());
	/// ```
	pub fn write(&self) -> FieldDescriptor {
		let mut s = JavaString::new();
		write_field_type(&self.0, &mut s);
		unsafe { FieldDescriptor::from_inner_unchecked(s) }
	}
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedMethodDescriptor {
	pub parameter_descriptors: Vec<Type>,
	pub return_descriptor: Option<Type>,
}

impl MethodDescriptorSlice {
	// TODO: same quality of doc as above
	pub fn parse(&self) -> Result<ParsedMethodDescriptor> {
		let mut chars = self.as_inner().chars().peekable();

		if chars.next_if_eq(&'(').is_none() {
			bail!("method descriptor {self:?} doesn't start with '('");
		}

		let mut parameter_descriptors = Vec::new();
		loop {
			if chars.next_if_eq(&')').is_some() {
				break;
			}

			let descriptor = read_field_type(&mut chars)
				.with_context(|| anyhow!("failed to read parameter descriptor of {self:?}"))?;
			parameter_descriptors.push(descriptor);
		}

		let return_descriptor = if chars.next_if_eq(&'V').is_some() {
			None
		} else {
			let descriptor = read_field_type(&mut chars)
				.with_context(|| anyhow!("failed to read return descriptor of {self:?}"))?;

			Some(descriptor)
		};

		if chars.peek().is_some() {
			bail!("expected end of method descriptor {self:?}, got {} remaining", JavaString::from_iter(chars));
		}

		Ok(ParsedMethodDescriptor {
			parameter_descriptors,
			return_descriptor,
		})
	}

	/// Returns the argument size + 1 (for the implicit `this`).
	/// Double and longs count 2 instead of 1.
	///
	/// Does not look at the return descriptor.
	pub(crate) fn get_arguments_size(&self) -> Result<u8> {
		let mut chars = self.as_inner().chars().peekable();

		if chars.next_if_eq(&'(').is_none() {
			bail!("method descriptor {self:?} doesn't start with '('");
		}

		let mut size = 1; // implicit `this` argument
		loop {
			if chars.next_if_eq(&')').is_some() {
				break;
			} else if chars.next_if(|&x| x == 'D' || x == 'J').is_some() {
				size += 2;
			} else {
				while chars.next_if_eq(&'[').is_some() { };

				let char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor {self:?}"))?;

				if char == 'L' {
					let mut char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor {self:?}"))?;
					while char != ';' {
						char = chars.next().ok_or_else(|| anyhow!("unexpected abrupt ending of descriptor {self:?}"))?;
					}
				}

				size += 1;
			}
		}

		Ok(size)
	}
}

impl ParsedMethodDescriptor {
	pub fn write(&self) -> MethodDescriptor {
		let mut s = JavaString::new();
		s.push('(');
		for parameter_descriptor in &self.parameter_descriptors {
			write_field_type(parameter_descriptor, &mut s);
		}
		s.push(')');
		if let Some(return_descriptor) = &self.return_descriptor {
			write_field_type(return_descriptor, &mut s);
		} else {
			s.push('V');
		}
		unsafe { MethodDescriptor::from_inner_unchecked(s) }
	}
}

make_string_str_like!(
	pub ReturnDescriptor(JavaString);
	pub ReturnDescriptorSlice(JavaStr);
	is_valid(s) = Ok(()); // TODO: impl
);

// TODO: rethink if we really want these examples with that amount of unsafe/JavaStr
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedReturnDescriptor(pub Option<Type>);

impl ReturnDescriptorSlice {
	/// Attempts to parse a return descriptor.
	///
	// TODO: check
	/// A return descriptor is defined by the [grammar](https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html#jvms-4.3.2) in the
	/// Java Virtual Machine Specification.
	///
	/// The inverse of this function is [`ParsedReturnDescriptor::write`].
	///
	/// # Examples
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use duke::tree::class::ClassName;
	/// use duke::tree::descriptor::{ArrayType, ParsedReturnDescriptor, ReturnDescriptorSlice, Type};
	///
	/// assert_eq!(
	///     unsafe { ReturnDescriptorSlice::from_inner_unchecked("I".into()) }.parse().unwrap(),
	///     ParsedReturnDescriptor(Some(Type::I))
	/// );
	/// assert_eq!(
	///     unsafe { ReturnDescriptorSlice::from_inner_unchecked("V".into()) }.parse().unwrap(),
	///     ParsedReturnDescriptor(None)
	/// );
	/// assert_eq!(
	///     unsafe { ReturnDescriptorSlice::from_inner_unchecked("Ljava/lang/Object;".into()) }.parse().unwrap(),
	///     ParsedReturnDescriptor(Some(Type::Object(ClassName::JAVA_LANG_OBJECT.to_owned())))
	/// );
	/// assert_eq!(
	///     unsafe { ReturnDescriptorSlice::from_inner_unchecked("[[[D".into()) }.parse().unwrap(),
	///     ParsedReturnDescriptor(Some(Type::Array(3, ArrayType::D)))
	/// );
	///
	/// let double_array = unsafe { ReturnDescriptorSlice::from_inner_unchecked("[[[D".into()) };
	/// assert_eq!(double_array, double_array.parse().unwrap().write());
	/// ```
	pub fn parse(&self) -> Result<ParsedReturnDescriptor> {
		let mut chars = self.as_inner().chars().peekable();

		let return_descriptor = if chars.next_if_eq(&'V').is_some() {
			None
		} else {
			let descriptor = read_field_type(&mut chars)
				.with_context(|| anyhow!("failed to read return descriptor {self:?}"))?;

			Some(descriptor)
		};

		if chars.peek().is_some() {
			bail!("expected end of return descriptor {self:?}, got {} remaining", JavaString::from_iter(chars));
		}

		Ok(ParsedReturnDescriptor(return_descriptor))
	}
}

impl ParsedReturnDescriptor {
	/// Writes a return descriptor.
	///
	/// The inverse of this function is [`ReturnDescriptorSlice::parse`].
	///
	/// # Examples
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use duke::tree::class::ClassName;
	/// use duke::tree::descriptor::{ArrayType, ParsedReturnDescriptor, Type};
	///
	/// assert_eq!(
	///     ParsedReturnDescriptor(Some(Type::I)).write().as_inner(),
 	///     "I"
	/// );
	/// assert_eq!(
	///     ParsedReturnDescriptor(None).write().as_inner(),
	///     "V"
	/// );
	/// assert_eq!(
	///     ParsedReturnDescriptor(Some(Type::Object(ClassName::JAVA_LANG_OBJECT.to_owned())))
	///         .write().as_inner(),
	///     "Ljava/lang/Object;"
	/// );
	/// assert_eq!(
	///     ParsedReturnDescriptor(Some(Type::Array(3, ArrayType::D))).write().as_inner(),
	///     "[[[D"
	/// );
	///
	/// let double_array = ParsedReturnDescriptor(Some(Type::Array(3, ArrayType::D)));
	/// assert_eq!(double_array, double_array.write().parse().unwrap());
	/// ```
	pub fn write(&self) -> ReturnDescriptor {
		let s = if let Some(return_descriptor) = &self.0 {
			let mut s = JavaString::new();
			write_field_type(return_descriptor, &mut s);
			s
		} else {
			JavaString::from("V")
		};
		unsafe { ReturnDescriptor::from_inner_unchecked(s) }
	}
}

impl From<FieldDescriptor> for ReturnDescriptor {
	/// Converts a field descriptor into a return descriptor.
	///
	/// Field descriptors are a subset of return descriptors.
	///
	/// The only value not represented by a field descriptor is `V`.
	fn from(value: FieldDescriptor) -> Self {
		let inner = value.into_inner();
		// SAFETY: `FieldDescriptor`s are a subset of `ReturnDescriptor`s
		unsafe { ReturnDescriptor::from_inner_unchecked(inner) }
	}
}

// TODO: restructure all things? split the tree:: files into one for each struct "type"...?
//  (so we end up with like tree::field::descriptor containing field descriptor stuff, and so on)
//  (tho possibly do it the other way around... not sure yet!)
impl FieldDescriptor {
	/// Creates a field descriptor of the class name given.
	///
	/// This is equivalent to something like `"L" + class_name + ";"`, but performs more checks:
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use duke::tree::class::ClassName;
	/// use duke::tree::field::FieldDescriptor;
	/// let a = FieldDescriptor::from("Ljava/lang/Object;");
	/// let b = FieldDescriptor::from_class(ClassName::JAVA_LANG_OBJECT);
	/// assert_eq!(a, b);
	// TODO: test cases, also one that fails, with array class name?
	/// ```
	pub fn from_class(class_name: &ClassNameSlice) -> FieldDescriptor {
		let class_name = class_name.as_inner();
		assert!(!class_name.starts_with('['));
		// TODO: remove this? more generally: decide about array classes
		//  (this includes searching for "assert!" and "["...)

		if class_name.starts_with('[') {
			// for array classes, the class name is just a descriptor already
			let desc = class_name.to_owned();
			// SAFETY: An array class name is a valid field descriptor.
			unsafe { FieldDescriptor::from_inner_unchecked(desc) }
		} else {
			// otherwise, build a descriptor by L...;-ing the class name
			let desc = JavaString::with_capacity(2 + class_name.len())
				+ "L" + class_name + ";";

			// SAFETY: `desc` is valid by construction.
			unsafe { FieldDescriptor::from_inner_unchecked(desc) }
		}
	}
}

#[cfg(test)]
mod testing {
	use pretty_assertions::assert_eq;
	use anyhow::Result;
	use java_string::JavaStr;
	use crate::tree::class::ClassNameSlice;
	use crate::tree::descriptor::{ParsedFieldDescriptor, ParsedMethodDescriptor, ParsedReturnDescriptor, ReturnDescriptorSlice, Type};
	use crate::tree::field::FieldDescriptorSlice;
	use crate::tree::method::MethodDescriptorSlice;

	const JAVA_LANG_THREAD: &ClassNameSlice = unsafe { ClassNameSlice::from_inner_unchecked(JavaStr::from_str("java/lang/Thread")) };
	const JAVA_LANG_OBJECT: &ClassNameSlice = unsafe { ClassNameSlice::from_inner_unchecked(JavaStr::from_str("java/lang/Object")) };

	#[test]
	fn field_parse() -> Result<()> {
		assert_eq!(
			unsafe { FieldDescriptorSlice::from_inner_unchecked("I".into()) }.parse()?,
			ParsedFieldDescriptor(Type::I),
		);
		assert_eq!(
			ParsedFieldDescriptor(Type::I).write().as_inner(),
			"I",
		);

		assert_eq!(
			unsafe { FieldDescriptorSlice::from_inner_unchecked("D".into()) }.parse()?,
			ParsedFieldDescriptor(Type::D),
		);
		assert_eq!(
			ParsedFieldDescriptor(Type::D).write().as_inner(),
			"D",
		);

		assert_eq!(
			unsafe { FieldDescriptorSlice::from_inner_unchecked("Ljava/lang/Thread;".into()) }.parse()?,
			ParsedFieldDescriptor(Type::Object(JAVA_LANG_THREAD.to_owned())),
		);
		assert_eq!(
			ParsedFieldDescriptor(Type::Object(JAVA_LANG_THREAD.to_owned()))
				.write().as_inner(),
			"Ljava/lang/Thread;",
		);

		assert_eq!(
			unsafe { FieldDescriptorSlice::from_inner_unchecked("Ljava/lang/Object;".into()) }.parse()?,
			ParsedFieldDescriptor(Type::Object(JAVA_LANG_OBJECT.to_owned())),
		);
		assert_eq!(
			ParsedFieldDescriptor(Type::Object(JAVA_LANG_OBJECT.to_owned()))
				.write().as_inner(),
			"Ljava/lang/Object;",
		);

		Ok(())
	}

	#[test]
	fn field_parse_err() -> Result<()> {
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("V".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("(".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked(")".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("()".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("[V".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("()V".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("(D)I".into()) }.parse().is_err());
		assert!(unsafe { FieldDescriptorSlice::from_inner_unchecked("L;DV".into()) }.parse().is_err());
		Ok(())
	}

	#[test]
	fn method_parse() -> Result<()> {
		assert_eq!(
			unsafe { MethodDescriptorSlice::from_inner_unchecked("(IDLjava/lang/Thread;)Ljava/lang/Object;".into()) }.parse()?,
			ParsedMethodDescriptor {
				parameter_descriptors: vec![
					Type::I,
					Type::D,
					Type::Object(JAVA_LANG_THREAD.to_owned()),
				],
				return_descriptor: Some(Type::Object(JAVA_LANG_OBJECT.to_owned()))
			},
		);
		assert_eq!(
			ParsedMethodDescriptor {
				parameter_descriptors: vec![
					Type::I,
					Type::D,
					Type::Object(JAVA_LANG_THREAD.to_owned()),
				],
				return_descriptor: Some(Type::Object(JAVA_LANG_OBJECT.to_owned()))
			}.write().as_inner(),
			"(IDLjava/lang/Thread;)Ljava/lang/Object;",
		);

		assert_eq!(
			unsafe { MethodDescriptorSlice::from_inner_unchecked("(Ljava/lang/Thread;Ljava/lang/Object;)V".into()) }.parse()?,
			ParsedMethodDescriptor {
				parameter_descriptors: vec![
					Type::Object(JAVA_LANG_THREAD.to_owned()),
					Type::Object(JAVA_LANG_OBJECT.to_owned()),
				],
				return_descriptor: None,
			},
		);
		assert_eq!(
			ParsedMethodDescriptor {
				parameter_descriptors: vec![
					Type::Object(JAVA_LANG_THREAD.to_owned()),
					Type::Object(JAVA_LANG_OBJECT.to_owned()),
				],
				return_descriptor: None,
			}.write().as_inner(),
			"(Ljava/lang/Thread;Ljava/lang/Object;)V",
		);

		Ok(())
	}

	#[test]
	fn method_parse_err() -> Result<()> {
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(D".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(V".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("()".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(I)".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(V)D".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(D)[".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(D)[V".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("[(D)V".into()) }.parse().is_err());
		assert!(unsafe { MethodDescriptorSlice::from_inner_unchecked("(L;;)V".into()) }.parse().is_err());
		Ok(())
	}

	#[test]
	fn method_get_arguments_size() -> Result<()> {
		impl MethodDescriptorSlice {
			fn from_str_(s: &str) -> &MethodDescriptorSlice {
				unsafe { Self::from_inner_unchecked(s.into()) }
			}
		}

		assert_eq!(MethodDescriptorSlice::from_str_("(IDLjava/lang/Thread;)Ljava/lang/Object;").get_arguments_size()?, 1 + 1 + 2 + 1);
		assert_eq!(MethodDescriptorSlice::from_str_("(Ljava/lang/Thread;Ljava/lang/Object;)V").get_arguments_size()?, 1 + 1 + 1);
		assert_eq!(MethodDescriptorSlice::from_str_("(BCDFIJLjava/lang/Thread;SZ)Ljava/lang/Object;").get_arguments_size()?, 1 + 1 + 1 + 2 + 1 + 1 + 2 + 1 + 1 + 1);
		assert_eq!(MethodDescriptorSlice::from_str_("(DDD)V").get_arguments_size()?, 1 + 2 + 2 + 2);
		assert_eq!(MethodDescriptorSlice::from_str_("(JJJ)V").get_arguments_size()?, 1 + 2 + 2 + 2);
		assert_eq!(MethodDescriptorSlice::from_str_("(D)V").get_arguments_size()?, 1 + 2);
		assert_eq!(MethodDescriptorSlice::from_str_("(I)V").get_arguments_size()?, 1 + 1);
		assert_eq!(MethodDescriptorSlice::from_str_("()V").get_arguments_size()?, 1);
		assert_eq!(MethodDescriptorSlice::from_str_("()J").get_arguments_size()?, 1);
		assert_eq!(MethodDescriptorSlice::from_str_("()D").get_arguments_size()?, 1);
		Ok(())
	}

	// TODO: method_get_arguments_size_err

	#[test]
	fn return_parse() -> Result<()> {
		assert_eq!(
			unsafe { ReturnDescriptorSlice::from_inner_unchecked("I".into()) }.parse()?,
			ParsedReturnDescriptor(Some(Type::I)),
		);
		assert_eq!(
			ParsedReturnDescriptor(Some(Type::I)).write().as_inner(),
			"I",
		);

		assert_eq!(
			unsafe { ReturnDescriptorSlice::from_inner_unchecked("V".into()) }.parse()?,
			ParsedReturnDescriptor(None),
		);
		assert_eq!(
			ParsedReturnDescriptor(None).write().as_inner(),
			"V",
		);

		assert_eq!(
			unsafe { ReturnDescriptorSlice::from_inner_unchecked("D".into()) }.parse()?,
			ParsedReturnDescriptor(Some(Type::D)),
		);
		assert_eq!(
			ParsedReturnDescriptor(Some(Type::D)).write().as_inner(),
			"D",
		);

		assert_eq!(
			unsafe { ReturnDescriptorSlice::from_inner_unchecked("Ljava/lang/Thread;".into()) }.parse()?,
			ParsedReturnDescriptor(Some(Type::Object(JAVA_LANG_THREAD.to_owned()))),
		);
		assert_eq!(
			ParsedReturnDescriptor(Some(Type::Object(JAVA_LANG_THREAD.to_owned())))
				.write().as_inner(),
			"Ljava/lang/Thread;",
		);

		assert_eq!(
			unsafe { ReturnDescriptorSlice::from_inner_unchecked("Ljava/lang/Object;".into()) }.parse()?,
			ParsedReturnDescriptor(Some(Type::Object(JAVA_LANG_OBJECT.to_owned()))),
		);
		assert_eq!(
			ParsedReturnDescriptor(Some(Type::Object(JAVA_LANG_OBJECT.to_owned())))
				.write().as_inner(),
			"Ljava/lang/Object;",
		);

		Ok(())
	}

	#[test]
	fn return_parse_err() -> Result<()> {
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked("".into()) }.parse().is_err());
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked("(".into()) }.parse().is_err());
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked(")".into()) }.parse().is_err());
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked("()".into()) }.parse().is_err());
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked("[V".into()) }.parse().is_err());
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked("()V".into()) }.parse().is_err());
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked("(D)I".into()) }.parse().is_err());
		assert!(unsafe { ReturnDescriptorSlice::from_inner_unchecked("L;DV".into()) }.parse().is_err());
		Ok(())
	}

}
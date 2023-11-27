use std::fmt::{Debug, Formatter};
use anyhow::{bail, Error};

#[derive(Clone)]
enum BaseType {
	B, C, D, F, I, J, S, Z, L(String),
}

impl From<&BaseType> for String {
	fn from(value: &BaseType) -> Self {
		match value {
			BaseType::B => "B".into(),
			BaseType::C => "C".into(),
			BaseType::D => "D".into(),
			BaseType::F => "F".into(),
			BaseType::I => "I".into(),
			BaseType::J => "J".into(),
			BaseType::S => "S".into(),
			BaseType::Z => "Z".into(),
			BaseType::L(name) => format!("L{};", name),
		}
	}
}

#[derive(Clone)]
pub(crate) struct FieldDescriptor {
	dimension: usize,
	base: BaseType,
}

impl TryFrom<&str> for FieldDescriptor {
	type Error = Error;
	fn try_from(value: &str) -> Result<Self, Self::Error> {
		todo!()
		//Ok(FieldDescriptor(value))
	}
}

impl From<&FieldDescriptor> for String {
	fn from(value: &FieldDescriptor) -> Self {
		format!("{}{}", "[".repeat(value.dimension), String::from(&value.base))
	}
}

impl Debug for FieldDescriptor {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&String::from(self)) // use our from and to string impl here
	}
}

#[derive(Clone)]
pub(crate) struct MethodDescriptor {
	args: Vec<FieldDescriptor>,
	ret: Option<FieldDescriptor>, // None is `void`, and nicely the rest fits well from the field!
}

impl TryFrom<&str> for MethodDescriptor {
	type Error = Error;
	fn try_from(value: &str) -> Result<Self, Self::Error> {
		todo!()
		//Ok(MethodDescriptor(value))
	}
}

impl From<&MethodDescriptor> for String {
	fn from(value: &MethodDescriptor) -> Self {
		let args: String = value.args.iter()
			.map(|x| String::from(x))
			.collect();
		let ret = match &value.ret {
			Some(base) => String::from(base),
			None => String::from("V"),
		};
		format!("({args}){ret}")
	}
}

impl Debug for MethodDescriptor {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&String::from(self)) // use our from and to string impl here
	}
}

#[cfg(test)]
mod testing {
	use crate::tree::descriptor::{BaseType, FieldDescriptor, MethodDescriptor};

	// TODO: write tests

	#[test]
	fn method_test_to_str() {
		let m = MethodDescriptor {
			args: vec![
				FieldDescriptor {
					dimension: 1,
					base: BaseType::B,
				},
				FieldDescriptor {
					dimension: 1,
					base: BaseType::C,
				},
			],
			ret: None,
		};

		assert_eq!(String::from(&m), String::from("([B[C)V"));
	}
}

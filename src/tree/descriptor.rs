use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use anyhow::{anyhow, bail, Context, Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldDescriptor {
	dimension: usize,
	base: BaseType,
}

impl FieldDescriptor {
	fn from_iter(iter: &mut Peekable<impl Iterator<Item=char>>) -> Result<FieldDescriptor> {
		let mut dimension = 0;
		while iter.next_if(|&x| x == '[').is_some() {
			dimension += 1;
		}

		let ch = iter.next()
			.with_context(|| anyhow!("Descriptor ends suddenly"))?;

		let base = match ch {
			'B' => BaseType::B,
			'C' => BaseType::C,
			'D' => BaseType::D,
			'F' => BaseType::F,
			'I' => BaseType::I,
			'J' => BaseType::J,
			'S' => BaseType::S,
			'Z' => BaseType::Z,
			'L' => {
				let mut s = String::new();
				while let Some(ch) = iter.next_if(|&x| x != ';') {
					s.push(ch);
				}
				if Some(';') != iter.next() {
					bail!("Expected semicolon to terminate class name {s:?} in descriptor");
				}
				BaseType::L(s)
			},
			x => {
				bail!("Unexpected base type {x:?} in descriptor");
			}
		};

		Ok(FieldDescriptor { dimension, base })
	}
}

impl TryFrom<String> for FieldDescriptor {
	type Error = Error;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		Self::try_from(value.as_str())
	}
}

impl TryFrom<&str> for FieldDescriptor {
	type Error = Error;
	fn try_from(value: &str) -> Result<Self, Self::Error> {
		let mut iter = value.chars().peekable();

		let descriptor = FieldDescriptor::from_iter(&mut iter)
			.with_context(|| anyhow!("Failed to parse field descriptor {value:?}"))?;

		if iter.next().is_some() {
			bail!("Field descriptor doesn't end: {value:?}");
		}

		Ok(descriptor)
	}
}

impl From<&FieldDescriptor> for String {
	fn from(value: &FieldDescriptor) -> Self {
		format!("{}{}", "[".repeat(value.dimension), String::from(&value.base))
	}
}

impl Debug for FieldDescriptor {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", String::from(self))
	}
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodDescriptor {
	args: Vec<FieldDescriptor>,
	ret: Option<FieldDescriptor>, // None is `void`, and nicely the rest fits well from the field!
}

impl MethodDescriptor {
	pub(crate) fn len(&self) -> usize {
		self.args.len()
	}
}

impl TryFrom<String> for MethodDescriptor {
	type Error = Error;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		Self::try_from(value.as_str())
	}
}

impl TryFrom<&str> for MethodDescriptor {
	type Error = Error;
	fn try_from(value: &str) -> Result<Self, Self::Error> {
		let mut iter = value.chars()
			.peekable();

		if Some('(') != iter.next() {
			bail!("Method descriptor must start with opening parenthesis: {value:?}");
		}

		let mut args = Vec::new();

		while iter.peek().is_some_and(|&x| x != ')') {
			let field_descriptor = FieldDescriptor::from_iter(&mut iter)
				.with_context(|| anyhow!("Failed to parse method parameter descriptor: {value:?}"))?;

			args.push(field_descriptor);
		}

		if Some(')') != iter.next() {
			bail!("Method descriptor must contain a closing parenthesis: {value:?}");
		}

		let ret = if Some(&'V') == iter.peek() {
			iter.next(); // take the `V`
			None
		} else {
			let ret = FieldDescriptor::from_iter(&mut iter)
				.with_context(|| anyhow!("Failed to parse method return descriptor: {value:?}"))?;

			Some(ret)
		};

		if iter.next().is_some() {
			bail!("Method descriptor doesn't end: {value:?}");
		}

		Ok(MethodDescriptor { args, ret })
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
		write!(f, "{:?}", String::from(self))
	}
}

#[cfg(test)]
mod testing {
	use crate::tree::descriptor::{BaseType, FieldDescriptor, MethodDescriptor};

	#[test]
	fn method_to_str() {
		assert_eq!(
			String::from(&MethodDescriptor {
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
			}),
			String::from("([B[C)V")
		);
		assert_eq!(
			String::from(&MethodDescriptor {
				args: vec![
					FieldDescriptor {
						dimension: 4,
						base: BaseType::L(String::from("a/fun/class/Name")),
					},
				],
				ret: Some(FieldDescriptor {
					dimension: 6,
					base: BaseType::Z,
				}),
			}),
			String::from("([[[[La/fun/class/Name;)[[[[[[Z")
		);
		assert_eq!(
			String::from(&MethodDescriptor {
				args: vec![],
				ret: None,
			}),
			String::from("()V")
		);
		assert_eq!(
			String::from(&MethodDescriptor {
				args: vec![
					FieldDescriptor {
						dimension: 0,
						base: BaseType::B,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::C,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::D,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::F,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::I,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::J,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::L(String::from("a")),
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::S,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::Z,
					},
				],
				ret: None,
			}),
			String::from("(BCDFIJLa;SZ)V")
		);
	}

	#[test]
	fn method_from_str() {
		assert_eq!(
			MethodDescriptor::try_from("([[[I[[[La/b;La;Lb;)[I").unwrap(),
			MethodDescriptor {
				args: vec![
					FieldDescriptor {
						dimension: 3,
						base: BaseType::I,
					},
					FieldDescriptor {
						dimension: 3,
						base: BaseType::L(String::from("a/b")),
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::L(String::from("a")),
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::L(String::from("b")),
					}
				],
				ret: Some(FieldDescriptor {
					dimension: 1,
					base: BaseType::I,
				}),
			}
		);
		assert_eq!(
			MethodDescriptor::try_from("()V").unwrap(),
			MethodDescriptor {
				args: vec![],
				ret: None,
			}
		);
		assert_eq!(
			MethodDescriptor::try_from("(BCDFIJLa;SZ)V").unwrap(),
			MethodDescriptor {
				args: vec![
					FieldDescriptor {
						dimension: 0,
						base: BaseType::B,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::C,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::D,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::F,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::I,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::J,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::L(String::from("a")),
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::S,
					},
					FieldDescriptor {
						dimension: 0,
						base: BaseType::Z,
					},
				],
				ret: None,
			}
		);
	}
}

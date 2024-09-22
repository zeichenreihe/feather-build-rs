use java_string::JavaString;

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
	pub name: JavaString,
	pub bytes: Vec<u8>,
}
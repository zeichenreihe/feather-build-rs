use anyhow::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClassName(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldDescriptor(String); // TODO: see method desc todo!

impl TryFrom<String> for FieldDescriptor {
	type Error = Error;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		Ok(FieldDescriptor(value))
	}
}

impl From<FieldDescriptor> for String {
	fn from(value: FieldDescriptor) -> Self {
		value.0
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldName(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodDescriptor(String); // TODO: switch to more complex representation that allow us to check all args / return type; + compare lengths!

impl TryFrom<String> for MethodDescriptor {
	type Error = Error;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		Ok(MethodDescriptor(value))
	}
}

impl From<MethodDescriptor> for String {
	fn from(value: MethodDescriptor) -> Self {
		value.0
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodName(pub(crate) String);


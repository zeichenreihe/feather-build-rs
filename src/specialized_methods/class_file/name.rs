#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClassName(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldDescriptor(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldName(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodDescriptor(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MethodName(pub(crate) String);


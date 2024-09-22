use anyhow::Result;
use java_string::JavaString;
use crate::class_reader::pool::PoolRead;
use crate::tree::attribute::Attribute;

pub trait UnknownAttributeVisitor: Sized {
	fn read(name: JavaString, bytes: Vec<u8>, pool: &PoolRead) -> Result<Self>;

	/// Note that because we don't pass the pool as well, it might be impossible to parse the
	/// attribute. Therefore, you can return `None` to indicate that.
	fn from_attribute(attribute: Attribute) -> Result<Option<Self>>;
}
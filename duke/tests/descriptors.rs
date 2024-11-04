use anyhow::Result;
use java_string::JavaStr;
use duke::tree::descriptor::ReturnDescriptor;
use duke::tree::field::FieldDescriptor;
use duke::tree::method::MethodDescriptor;

// TODO: tests for array dimension >= 256

#[test]
fn valid_field_descriptors() -> Result<()> {
	let valid_field_descriptors = [
		"B",
		"C",
		"D",
		"F",
		"I",
		"J",
		"Ljava/lang/Object;",
		"Lorg/example/MyClassName;",
		"S",
		"Z",
		"[[[D",
	];

	for i in valid_field_descriptors {
		assert!(
			FieldDescriptor::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid field desc", i
		);
	}

	Ok(())
}

#[test]
fn invalid_field_descriptors() -> Result<()> {
	let invalid_field_descriptors = [
		"",
		"V",
		"(",
		")",
		"()",
		"[V",
		"L;",
		"()V",
		"foo",
		"(D)I",
		"L;DV",
	];

	for i in invalid_field_descriptors {
		assert!(
			!FieldDescriptor::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid field desc", i
		);
	}

	Ok(())
}

#[test]
fn valid_method_descriptors() -> Result<()> {
	let valid_method_descriptors = [
		"()V",
		"(D)I",
		"(Ljava/lang/Object;)Ljava/lang/Object;",
	];

	for i in valid_method_descriptors {
		assert!(
			MethodDescriptor::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid method desc", i
		);
	}

	Ok(())
}

#[test]
fn invalid_method_descriptors() -> Result<()> {
	let invalid_method_descriptors = [
		"B",
		"C",
		"D",
		"F",
		"I",
		"J",
		"Ljava/lang/Object;",
		"Lorg/example/MyClassName;",
		"S",
		"Z",
		"[[[D",
		"",
		"V",
		"(",
		")",
		"()",
		"[V",
		"L;",
		"foo",
		"L;DV",
		"(L;)V",
	];

	for i in invalid_method_descriptors {
		assert!(
			!MethodDescriptor::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid method desc", i
		);
	}

	Ok(())
}

#[test]
fn valid_return_descriptors() -> Result<()> {
	let valid_return_descriptors = [
		"B",
		"C",
		"D",
		"F",
		"I",
		"J",
		"Ljava/lang/Object;",
		"Lorg/example/MyClassName;",
		"S",
		"V",
		"Z",
		"[[[D",
	];

	for i in valid_return_descriptors {
		assert!(
			ReturnDescriptor::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid return desc", i
		);
	}

	Ok(())
}

#[test]
fn invalid_return_descriptors() -> Result<()> {
	let invalid_return_descriptors = [
		"",
		"(",
		")",
		"()",
		"[V",
		"L;",
		"()V",
		"foo",
		"(D)I",
		"L;DV",
	];

	for i in invalid_return_descriptors {
		assert!(
			!ReturnDescriptor::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid return desc", i
		);
	}

	Ok(())
}




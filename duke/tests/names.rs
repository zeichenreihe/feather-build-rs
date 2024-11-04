use anyhow::Result;
use java_string::JavaStr;
use duke::tree::class::{ArrClassName, ClassName, ObjClassName};
use duke::tree::field::FieldName;
use duke::tree::method::MethodName;

// TODO: tests for array dimension >= 256

#[test]
fn valid_class_names() -> Result<()> {
	let valid_class_names = [
		"foo",
		"foo$bar",
		"java/lang/Object",
		"org/example/MyClassName",
		"[[[D",
		"[[Ljava/lang/Integer;",
	];

	for i in valid_class_names {
		assert!(
			ClassName::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid class name", i
		);
	}

	Ok(())
}

#[test]
fn invalid_class_names() -> Result<()> {
	let invalid_class_names = [
		"",
		".",
		"/",
		";",
		"[",
		"a/",
		"/a",
		"[V",
		"L;",
		"//a",
		"a//",
		"a.b",
		"a;b",
		"a[b",
		"L;DV",
		"a//a",
	];

	for i in invalid_class_names {
		assert!(
			!ClassName::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid class name", i
		);
	}

	Ok(())
}

#[test]
fn valid_arr_class_names() -> Result<()> {
	let valid_arr_class_names = [
		"[[[D",
		"[[Ljava/lang/Integer;",
	];

	for i in valid_arr_class_names {
		assert!(
			ArrClassName::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid array class name name", i
		);
	}

	Ok(())
}

#[test]
fn invalid_arr_class_names() -> Result<()> {
	let invalid_arr_class_names = [
		"",
		"B",
		"C",
		"I",
		"J",
		"Z",
		"(",
		")",
		"()",
		"[V",
		"L;",
		"()V",
		"foo",
		"foo$bar",
		"(D)I",
		"L;DV",
		"Ljava/lang/Object;",
		"Lorg/example/MyClassName;",
	];

	for i in invalid_arr_class_names {
		assert!(
			!ArrClassName::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid array class name", i
		);
	}

	Ok(())
}

#[test]
fn valid_obj_class_names() -> Result<()> {
	let valid_obj_class_names = [
		"1234", // yes numbers are allowed at the start, only the java language denies it
		"---",
		"foo",
		"foo$bar",
		"java/lang/Object",
		"org/example/MyClassName",
	];

	for i in valid_obj_class_names {
		assert!(
			ObjClassName::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid object class name", i
		);
	}

	Ok(())
}

#[test]
fn invalid_obj_class_names() -> Result<()> {
	let invalid_obj_class_names = [
		"",
		"[V",
		"L;",
		"L;DV",
		"[[[D",
		"[[Ljava/lang/Integer;",
	];

	for i in invalid_obj_class_names {
		assert!(
			!ObjClassName::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid object class name", i
		);
	}

	Ok(())
}

#[test]
fn valid_field_names() -> Result<()> {
	let valid_field_names = [
		"foo",
		"bar",
		"L<foo>",
		"---",
		"1234",
		"do",
		"while",
	];
	
	for i in valid_field_names {
		assert!(
			FieldName::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid field name", i
		);
	}
	
	Ok(())
}

#[test]
fn invalid_field_names() -> Result<()> {
	let invalid_field_names = [
		"",
		".",
		";",
		"[",
		"/",
	];

	for i in invalid_field_names {
		assert!(
			!FieldName::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid field name", i
		);
	}

	Ok(())
}

#[test]
fn valid_method_names() -> Result<()> {
	let valid_method_names = [
		"foo",
		"<init>",
		"<clinit>",
		"123",
		"---",
		"bar",
		"$foo$",
	];

	for i in valid_method_names {
		assert!(
			MethodName::is_valid(JavaStr::from_str(i)),
			"{:?} is a valid method name", i
		);
	}

	Ok(())
}

#[test]
fn invalid_method_names() -> Result<()> {
	let invalid_method_names = [
		"",
		"<foo>",
		"<clinit",
		"clinit>",
		".",
		";",
		"[",
		"/",
		"<",
		">",
	];

	for i in invalid_method_names {
		assert!(
			!MethodName::is_valid(JavaStr::from_str(i)),
			"{:?} is an invalid method name", i
		);
	}

	Ok(())
}




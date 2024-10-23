pub mod class;
pub mod field;
pub mod method;
pub mod attribute;
pub mod version;
pub mod module;
pub mod annotation;
pub mod descriptor;
pub mod record;
pub mod type_annotation;

mod names {
	use java_string::{JavaCodePoint, JavaStr};

	const DOT: JavaCodePoint = JavaCodePoint::from_char('.');
	const SEMICOLON: JavaCodePoint = JavaCodePoint::from_char(';');
	const OPENING_BRACKET: JavaCodePoint = JavaCodePoint::from_char('[');
	const SLASH: JavaCodePoint = JavaCodePoint::from_char('/');
	const LESS_THAN: JavaCodePoint = JavaCodePoint::from_char('<');
	const GREATER_THAN: JavaCodePoint = JavaCodePoint::from_char('>');

	/// Checks if a class name is valid according to JVMS 4.2.1 (also accepting array class names).
	pub(super) fn is_valid_class_name(x: &JavaStr) -> bool {
		if x.starts_with('[') {
			// TODO: max 255 [ are allowed
			// TODO: must be a field desc
			true
		} else {
			// a list of identifiers split by /
			// each identifier must be an unqualified name
			x.split('/').all(is_valid_unqualified_name)
			// TODO: explicitly note that "" is not valid (tests?!)
		}
	}

	/// Checks if a class name is a valid array class name according to JVMS 4.2.1
	pub(super) fn is_valid_arr_class_name(x: &JavaStr) -> bool {
		if x.starts_with('[') {
			// TODO: max 255 [ are allowed
			// TODO: must be a field desc
			true
		} else {
			false
		}
	}

	/// Checks if a class name is a valid object class name according to JVMS 4.2.1
	///
	/// Doesn't accept array class names.
	pub(super) fn is_valid_obj_class_name(x: &JavaStr) -> bool {
		// doesn't start with [
		// a list of identifiers split by /
		// each identifier must be an unqualified name
		!x.starts_with('[') && x.split('/').all(is_valid_unqualified_name)
	}

	/// Checks if a name is an unqualified name according to JVMS 4.2.2
	///
	/// This is used for field names, formal parameter names, local variable names.
	pub(super) fn is_valid_unqualified_name(x: &JavaStr) -> bool {
		// must contain at least one unicode codepoint
		!x.is_empty() &&
			// must not contain any of . ; [ /
			x.chars().all(|c| !matches!(c, DOT | SEMICOLON | OPENING_BRACKET | SLASH))

	}

	/// Checks if a method name is valid according to JVMS 4.2.2
	pub(super) fn is_valid_method_name(x: &JavaStr) -> bool {
		// either one of the special names or an unqualified name with special < > restriction
		x == "<init>" || x == "<clinit>" || (
			// must contain at least one unicode codepoint
			!x.is_empty() &&
				x.chars().all(|c| !matches!(c, DOT | SEMICOLON | OPENING_BRACKET | SLASH | LESS_THAN | GREATER_THAN))
		)
	}

	// TODO: 4.2.3 module and package names

	#[cfg(test)]
	mod testing {
		use crate::tree::names::*;

		// TODO: arr and obj class name tests

		#[test]
		fn class_names() {
			assert!(is_valid_class_name(JavaStr::from_str("java/lang/Object")));
			assert!(is_valid_class_name(JavaStr::from_str("java/lang/Thread")));
			assert!(is_valid_class_name(JavaStr::from_str("[[[D")));
			assert!(is_valid_class_name(JavaStr::from_str("An$Inner$Class")));

			assert!(!is_valid_class_name(JavaStr::from_str(""))); // it may come as a surprise, but an empty class name is not valid
			assert!(!is_valid_class_name(JavaStr::from_str("/")));
			assert!(!is_valid_class_name(JavaStr::from_str("/a")));
			assert!(!is_valid_class_name(JavaStr::from_str("a/")));
			assert!(!is_valid_class_name(JavaStr::from_str("//a")));
			assert!(!is_valid_class_name(JavaStr::from_str("a//")));
			assert!(!is_valid_class_name(JavaStr::from_str("a.b")));
			assert!(!is_valid_class_name(JavaStr::from_str("a;b")));
			assert!(!is_valid_class_name(JavaStr::from_str("a[b")));
		}

		#[test]
		fn unqualified_names() {
			assert!(is_valid_unqualified_name(JavaStr::from_str("foo")));
			assert!(is_valid_unqualified_name(JavaStr::from_str("bar")));
			assert!(is_valid_unqualified_name(JavaStr::from_str("FOO")));
			assert!(is_valid_unqualified_name(JavaStr::from_str("1234567"))); // yes numbers are valid here, but not in java source code
			assert!(is_valid_unqualified_name(JavaStr::from_str("---")));
			assert!(is_valid_unqualified_name(JavaStr::from_str("a$name")));

			assert!(!is_valid_unqualified_name(JavaStr::from_str("")));
			assert!(!is_valid_unqualified_name(JavaStr::from_str(".")));
			assert!(!is_valid_unqualified_name(JavaStr::from_str(";")));
			assert!(!is_valid_unqualified_name(JavaStr::from_str("[")));
			assert!(!is_valid_unqualified_name(JavaStr::from_str("/")));
		}

		#[test]
		fn method_names() {
			assert!(is_valid_method_name(JavaStr::from_str("foo")));
			assert!(is_valid_method_name(JavaStr::from_str("bar")));
			assert!(is_valid_method_name(JavaStr::from_str("FOO")));
			assert!(is_valid_method_name(JavaStr::from_str("1234567"))); // yes numbers are valid here, but not in java source code
			assert!(is_valid_method_name(JavaStr::from_str("---")));
			assert!(is_valid_method_name(JavaStr::from_str("a$name")));

			assert!(!is_valid_method_name(JavaStr::from_str("")));
			assert!(!is_valid_method_name(JavaStr::from_str(".")));
			assert!(!is_valid_method_name(JavaStr::from_str(";")));
			assert!(!is_valid_method_name(JavaStr::from_str("[")));
			assert!(!is_valid_method_name(JavaStr::from_str("/")));
			assert!(!is_valid_method_name(JavaStr::from_str("<NotClinit>")));
			assert!(!is_valid_method_name(JavaStr::from_str("<>")));
			assert!(!is_valid_method_name(JavaStr::from_str("<")));
			assert!(!is_valid_method_name(JavaStr::from_str(">")));
		}
	}
}
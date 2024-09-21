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
	/// Checks if a class name is valid according to JVMS 4.2.1 (also accepting array class names).
	pub(super) fn is_valid_class_name(x: &str) -> bool {
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

	/// Checks if a name is an unqualified name according to JVMS 4.2.2
	///
	/// This is used for field names, formal parameter names, local variable names.
	pub(super) fn is_valid_unqualified_name(x: &str) -> bool {
		// must contain at least one unicode codepoint
		!x.is_empty() &&
			// must not contain any of . ; [
			// /
			x.chars().all(|c| !matches!(c, '.' | ';' | '[' | '/'))

	}

	/// Checks if a method name is valid according to JVMS 4.2.2
	pub(super) fn is_valid_method_name(x: &str) -> bool {
		// either one of the special names or an unqualified name with special < > restriction
		x == "<init>" || x == "<clinit>" || (
			// must contain at least one unicode codepoint
			!x.is_empty() &&
				x.chars().all(|c| !matches!(c, '.' | ';' | '[' | '/' | '<' | '>'))
		)
	}

	// TODO: 4.2.3 module and package names

	#[cfg(test)]
	mod testing {
		use crate::tree::names::*;

		#[test]
		fn class_names() {
			assert!(is_valid_class_name("java/lang/Object"));
			assert!(is_valid_class_name("java/lang/Thread"));
			assert!(is_valid_class_name("[[[D"));
			assert!(is_valid_class_name("An$Inner$Class"));

			assert!(!is_valid_class_name("")); // it may come as a surprise, but an empty class name is not valid
			assert!(!is_valid_class_name("/"));
			assert!(!is_valid_class_name("/a"));
			assert!(!is_valid_class_name("a/"));
			assert!(!is_valid_class_name("//a"));
			assert!(!is_valid_class_name("a//"));
			assert!(!is_valid_class_name("a.b"));
			assert!(!is_valid_class_name("a;b"));
			assert!(!is_valid_class_name("a[b"));
		}

		#[test]
		fn unqualified_names() {
			assert!(is_valid_unqualified_name("foo"));
			assert!(is_valid_unqualified_name("bar"));
			assert!(is_valid_unqualified_name("FOO"));
			assert!(is_valid_unqualified_name("1234567")); // yes numbers are valid here, but not in java source code
			assert!(is_valid_unqualified_name("---"));
			assert!(is_valid_unqualified_name("a$name"));

			assert!(!is_valid_unqualified_name(""));
			assert!(!is_valid_unqualified_name("."));
			assert!(!is_valid_unqualified_name(";"));
			assert!(!is_valid_unqualified_name("["));
			assert!(!is_valid_unqualified_name("/"));
		}

		#[test]
		fn method_names() {
			assert!(is_valid_method_name("foo"));
			assert!(is_valid_method_name("bar"));
			assert!(is_valid_method_name("FOO"));
			assert!(is_valid_method_name("1234567")); // yes numbers are valid here, but not in java source code
			assert!(is_valid_method_name("---"));
			assert!(is_valid_method_name("a$name"));

			assert!(!is_valid_method_name(""));
			assert!(!is_valid_method_name("."));
			assert!(!is_valid_method_name(";"));
			assert!(!is_valid_method_name("["));
			assert!(!is_valid_method_name("/"));
			assert!(!is_valid_method_name("<NotClinit>"));
			assert!(!is_valid_method_name("<>"));
			assert!(!is_valid_method_name("<"));
			assert!(!is_valid_method_name(">"));
		}
	}
}
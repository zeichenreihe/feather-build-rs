
// TODO: always keep in sync with the "normal" name checking stuff

/// Checks if a class name is valid according to JVMS 4.2.1 (also accepting array class names).
pub(crate) fn is_valid_class_name(x: String) -> Result<(), String> {
	if x.starts_with('[') {
		// TODO: max 255 [ are allowed
		// TODO: must be a field desc
		Ok(())
	} else {
		// a list of identifiers split by /
		// each identifier must be an unqualified name
		for unqualified_name in x.split('/') {
			// must contain at least one unicode codepoint
			if unqualified_name.is_empty() {
				return Err(format!("invalid class name: empty segment {unqualified_name:?} (split at `/`) not allowed"));
			}
			// must not contain any of . ; [ /
			if unqualified_name.contains(['.', ';', '[', '/']) {
				return Err(format!("invalid class name: segment {unqualified_name:?} (split at `/`) contains on of `.`, `;` or `[`, which are not allowed"));
			}
		}
		Ok(())
	}
}

/// Checks if a class name is a valid array class name according to JVMS 4.2.1
pub(crate) fn is_valid_arr_class_name(x: String) -> Result<(), String> {
	if x.starts_with('[') {
		// TODO: max 255 [ are allowed
		// TODO: must be a field desc
		Ok(())
	} else {
		Err("invalid array class name: must start with `[`".to_owned())
	}
}

/// Checks if a class name is a valid object class name according to JVMS 4.2.1
///
/// Doesn't accept array class names.
pub(crate) fn is_valid_obj_class_name(x: String) -> Result<(), String> {
	// doesn't start with [
	if !x.starts_with('[') {
		// a list of identifiers split by /
		// each identifier must be an unqualified name
		for unqualified_name in x.split('/') {
			// must contain at least one unicode codepoint
			if unqualified_name.is_empty() {
				return Err(format!("invalid object class name: empty segment {unqualified_name:?} (split at `/`) not allowed"));
			}
			// must not contain any of . ; [ /
			if unqualified_name.contains(['.', ';', '[', '/']) {
				return Err(format!("invalid object class name: segment {unqualified_name:?} (split at `/`) contains on of `.`, `;` or `[`, which are not allowed"));
			}
		}
		Ok(())
	} else {
		Err("invalid object class name: must not start with `[`".to_owned())
	}
}

/// Checks if a name is an unqualified name according to JVMS 4.2.2
///
/// This is used for field names, formal parameter names, local variable names.
pub(crate) fn is_valid_unqualified_name(x: String, usage: &str) -> Result<(), String> {
	// must contain at least one unicode codepoint
	if x.is_empty() {
		return Err(format!("invalid {usage} name: empty name not allowed"));
	}
	// must not contain any of . ; [ /
	if x.contains(['.', ';', '[', '/']) {
		return Err(format!("invalid {usage} name: must not contain `.`, `;`, `[` or `/`"));
	}
	Ok(())
}

/// Checks if a method name is valid according to JVMS 4.2.2
pub(crate) fn is_valid_method_name(x: String) -> Result<(), String> {
	// either one of the special names
	if x == "<init>" || x == "<clinit>" {
		Ok(())
	} else {
		// or an unqualified name with special < > restriction

		// must contain at least one unicode codepoint
		if x.is_empty() {
			return Err("invalid method name: empty name not allowed".to_string());
		}
		// must not contain any of . ; [ / < >
		if x.contains(['.', ';', '[', '/', '<', '>']) {
			return Err("invalid method name: must not contain `.`, `;`, `[`, `/`, `<` or `>`".to_string());
		}
		Ok(())
	}
}

// TODO: 4.2.3 module and package names
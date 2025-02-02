//! Crate for compile time checked strings of class names and the like.
// TODO: docs here (with comments testing out working / not working)

use proc_macro::TokenStream;
use quote::quote;
use syn::__private::TokenStream2;
use syn::parse_macro_input;
use crate::names::{is_valid_arr_class_name, is_valid_class_name, is_valid_method_name, is_valid_obj_class_name, is_valid_unqualified_name};

mod names;

fn check(tokens: TokenStream, checker: impl Fn(String) -> Result<(), String>, path: TokenStream2) -> TokenStream {
	let input: syn::LitStr = parse_macro_input!(tokens);

	match checker(input.value()) {
		Ok(()) => quote!{{
			let string = java_string::JavaStr::from_str(#input);
			// SAFETY: We checked that `string` is valid for `#path` because of the `checker` call on `#input` above.
			unsafe { #path::from_inner_unchecked(string) }
		}},
		Err(msg) => syn::Error::new(input.span(), msg).into_compile_error(),
	}.into()
}



#[proc_macro]
pub fn class_name(tokens: TokenStream) -> TokenStream {
	check(tokens, is_valid_class_name, quote!{duke::tree::class::ClassNameSlice})
}

#[proc_macro]
pub fn arr_class_name(tokens: TokenStream) -> TokenStream {
	check(tokens, is_valid_arr_class_name, quote!{duke::tree::class::ArrClassNameSlice})
}

/// A compile-time checked object class name
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use duke_macros::obj_class_name;
/// let a = obj_class_name!("java/lang/Object");
/// assert_eq!(a.as_inner(), "java/lang/Object");
///
/// obj_class_name!("foo");
/// obj_class_name!("foo$bar");
/// obj_class_name!("org/example/MyClassName");
/// obj_class_name!("123456");
/// ```
///
/// Array class names are not valid object class names (these won't compile):
/// ```compile_fail
/// duke_macros::obj_class_name!("[[[D");
/// ```
/// ```compile_fail
/// duke_macros::obj_class_name!("[[Ljava/lang/Integer;");
/// ```
/// Other invalid object class names:
/// ```compile_fail
/// duke_macros::obj_class_name!("");
/// ```
/// The characters `.`, `;`, `[` and `/` are not allowed:
/// ```compile_fail
/// duke_macros::obj_class_name!(".");
/// ```
/// ```compile_fail
/// # use duke_macros::obj_class_name;
/// duke_macros::obj_class_name!(";");
/// ```
/// ```compile_fail
/// duke_macros::obj_class_name!("[");
/// ```
/// ```compile_fail
/// duke_macros::obj_class_name!("/");
/// ```
/// Empty segments are invalid:
/// ```compile_fail
/// duke_macros::obj_class_name!("a/");
/// ```
/// ```compile_fail
/// duke_macros::obj_class_name!("/a");
/// ```
/// ```compile_fail
/// duke_macros::obj_class_name!("a//a");
/// ```
#[proc_macro]
pub fn obj_class_name(tokens: TokenStream) -> TokenStream {
	check(tokens, is_valid_obj_class_name, quote!{duke::tree::class::ObjClassNameSlice})
}

#[proc_macro]
pub fn field(tokens: TokenStream) -> TokenStream {
	check(tokens, |x| is_valid_unqualified_name(x, "field"), quote!{duke::tree::field::FieldNameSlice})
}

#[proc_macro]
pub fn method(tokens: TokenStream) -> TokenStream {
	check(tokens, is_valid_method_name, quote!{duke::tree::method::MethodNameSlice})
}

#[proc_macro]
pub fn parameter(tokens: TokenStream) -> TokenStream {
	check(tokens, |x| is_valid_unqualified_name(x, "parameter"), quote!{duke::tree::method::ParameterNameSlice})
}

#[proc_macro]
pub fn local_variable(tokens: TokenStream) -> TokenStream {
	check(tokens, |x| is_valid_unqualified_name(x, "local variable"), quote!{duke::tree::method::code::LocalVariableNameSlice})
}





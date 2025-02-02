use duke_macros::arr_class_name as foo;

#[test]
fn bar() {
	let a = foo!("test class name");
	dbg!(a);
	let b = foo!(";test class name");
	dbg!(b);
}
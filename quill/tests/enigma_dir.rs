use std::path::Path;
use anyhow::Result;
use duke_macros::obj_class_name;
use quill::tree::names::Namespaces;

#[test]
fn read_dir() -> Result<()> {
	let path = Path::new("tests/enigma_dir");

	let namespaces = Namespaces::try_from([
		"first".to_owned(),
		"second".to_owned(),
	])?;

	let mappings = quill::enigma_dir::read::<()>(&path, namespaces)?;

	//dbg!(&mappings);

	let namespace_a = mappings.get_namespace("first")?;
	let namespace_b = mappings.get_namespace("second")?;

	assert_eq!(mappings.classes.len(), 17);

	let a = mappings.classes.get(obj_class_name!("org/example/foo/bar/ComplexClass")).unwrap();
	assert_eq!(a.javadoc.as_ref().unwrap().0, "It shouldn't matter where comments are\nhow are multiple comments handled?");
	assert_eq!(a.info.names[namespace_a].as_deref(), Some(obj_class_name!("org/example/foo/bar/ComplexClass")));
	assert_eq!(a.info.names[namespace_b].as_deref(), Some(obj_class_name!("com/example/second/foo/bar/ComplexClass")));

	let b = mappings.classes.get(obj_class_name!("org/example/foo/bar/ComplexClass$Foo1$Foo2$Foo3$Foo4$Foo5$Foo6$Foo7$Foo8$Foo9$Foo10$Foo11$Foo12")).unwrap();
	assert_eq!(b.javadoc.as_ref().unwrap().0, "deep nesting");
	assert_eq!(b.info.names[namespace_a].as_deref(), Some(obj_class_name!("org/example/foo/bar/ComplexClass$Foo1$Foo2$Foo3$Foo4$Foo5$Foo6$Foo7$Foo8$Foo9$Foo10$Foo11$Foo12")));
	assert_eq!(b.info.names[namespace_b].as_deref(), Some(obj_class_name!("com/example/second/foo/bar/ComplexClass$Foo1Second$Foo2Second$Foo3Second$Foo4Second$Foo5Second$Foo6Second$Foo7Second$Foo8Second$Foo9Second$Foo10Second$Foo11Second$Foo12Second")));

	Ok(())
}
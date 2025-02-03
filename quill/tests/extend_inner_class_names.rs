
use anyhow::Result;
use pretty_assertions::assert_eq;
use quill::tree::mappings::Mappings;

#[test]
fn extend_inner_class_names() -> Result<()> {
	let input = include_str!("extend_inner_class_names_input.tiny");
	let expected = include_str!("extend_inner_class_names_output.tiny");

	let input: Mappings<2, ()> = quill::tiny_v2::read(input.as_bytes())?;

	let output = input.extend_inner_class_names("namespaceB")?;

	let actual = quill::tiny_v2::write_string(&output)?;

	assert_eq!(actual, expected, "left: actual, right: expected");

	Ok(())
}
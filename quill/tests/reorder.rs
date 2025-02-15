
use anyhow::Result;
use pretty_assertions::assert_eq;

#[test]
fn reorder() -> Result<()> {
	let input = include_str!("reorder_input.tiny");
	let expected = include_str!("reorder_output.tiny");

	let input = quill::tiny_v2::read::<2, ()>(input.as_bytes())?;

	let output = input.reorder::<()>(["namespaceB", "namespaceA"])?;

	let actual = quill::tiny_v2::write_string(&output)?;

	assert_eq!(actual, expected, "left: actual, right: expected");

	Ok(())

}
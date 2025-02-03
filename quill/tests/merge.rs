
use anyhow::Result;
use pretty_assertions::assert_eq;
use quill::tree::mappings::Mappings;

#[test]
fn merge() -> Result<()> {
	let input_a = include_str!("merge_input_a.tiny");
	let input_b = include_str!("merge_input_b.tiny");
	let expected = include_str!("merge_output.tiny");

	struct A; struct B; struct C;

	let input_a = quill::tiny_v2::read::<2, (A, B)>(input_a.as_bytes())?;
	let input_b = quill::tiny_v2::read::<2, (A, C)>(input_b.as_bytes())?;

	let output = Mappings::merge(&input_a, &input_b)?;

	let actual = quill::tiny_v2::write_string(&output)?;

	assert_eq!(actual, expected, "left: actual, right: expected");

	Ok(())
}
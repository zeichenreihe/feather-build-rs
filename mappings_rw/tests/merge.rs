
use anyhow::Result;
use pretty_assertions::assert_eq;
use mappings_rw::tree::mappings::Mappings;

#[test]
fn merge() -> Result<()> {
	let input_a = include_str!("merge_input_a.tiny");
	let input_b = include_str!("merge_input_b.tiny");
	let expected = include_str!("merge_output.tiny");

	let input_a = mappings_rw::tiny_v2::read(input_a.as_bytes())?;
	let input_b = mappings_rw::tiny_v2::read(input_b.as_bytes())?;

	let output = Mappings::merge(&input_a, &input_b)?;

	let actual = mappings_rw::tiny_v2::write_string(&output)?;

	assert_eq!(actual, expected, "left: actual, right: expected");

	Ok(())
}
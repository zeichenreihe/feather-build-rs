
use anyhow::Result;
use pretty_assertions::assert_eq;
use mappings_rw::tree::mappings::Mappings;

#[test]
fn remove_dummy() -> Result<()> {
	let input = include_str!("remove_dummy_input.tiny");
	let expected = include_str!("remove_dummy_output.tiny");

	let input: Mappings<2> = mappings_rw::tiny_v2::read(input.as_bytes())?;

	let output = input.remove_dummy("namespaceB")?;

	let actual = mappings_rw::tiny_v2::write_string(&output)?;

	assert_eq!(actual, expected, "left: actual, right: expected");

	Ok(())
}
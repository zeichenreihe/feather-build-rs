//! Methods for converting the string format used in the Java Virtual Machine Specification to and from
//! rust strings.
//!
//! The Java Virtual Machine Specification's string format is using a 2x3-format and storing `\0`
//! using two bytes.
//!
//! See <https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html#jvms-4.4.7> for the complete specification of
//! the string format used in the Java Virtual Machine Specification.

use std::borrow::Cow;
use anyhow::{anyhow, Context, Result};
use java_string::{JavaStr, JavaString};

/// Takes in a vec of data, tries to read it into a [`JavaString`].
pub(crate) fn from_vec_to_string(vec: Vec<u8>) -> Result<JavaString> {
	JavaString::from_modified_utf8(vec)
		.with_context(|| anyhow!("invalid java utf8 contents"))
}

/// Takes in a string and writes it out into a vec.
pub(crate) fn from_string_to_vec(string: &JavaStr) -> Cow<[u8]> {
	string.to_modified_utf8()
}

#[cfg(test)]
mod testing {
	use anyhow::Result;
	use java_string::JavaStr;
	use pretty_assertions::assert_eq;
	use crate::jstring::{from_string_to_vec, from_vec_to_string};

	fn round_trip_str(raw: &[u8], string: &str) -> Result<()> {
		let str = JavaStr::from_str(string);
		round_trip_java_str(raw, str)
	}
	fn round_trip_java_str(raw: &[u8], str: &JavaStr) -> Result<()> {
		assert_eq!(from_string_to_vec(str), raw);
		assert_eq!(from_vec_to_string(raw.to_owned())?, str);
		Ok(())
	}
	fn round_trip_owned_java_string(raw: Vec<u8>, string: String) -> Result<()> {
		round_trip_str(&raw, &string)
	}

	#[test]
	fn zero() -> Result<()> {
		round_trip_str(&[0b1100_0000, 0b1000_0000, 0b1100_0000, 0b1000_0000, 0b1100_0000, 0b1000_0000], "\0\0\0")
	}

	#[test]
	fn one_byte() -> Result<()> {
		round_trip_owned_java_string((0x0001..=0x007f).collect(), ('\u{0001}'..='\u{007f}').collect())?;
		round_trip_owned_java_string((0x0001..=0x007f).rev().collect(), ('\u{0001}'..='\u{007f}').rev().collect())
	}

	#[test]
	fn two_bytes() -> Result<()> {
		let vec = &[
			0b1100_0000, 0b1000_0000,
			0b1100_0010, 0b1000_0000,
			0b1100_1111, 0b1000_1010,
			0b1101_0011, 0b1011_1110,
			0b1101_1110, 0b1011_1010,
			0b1101_0110, 0b1011_1110,
			0b1101_1111, 0b1011_1111,
		];
		let str = JavaStr::from_str("\u{0000}\u{0080}\u{03ca}\u{04fe}\u{07ba}\u{05be}\u{07ff}");
		round_trip_java_str(vec, str)?;

		let vec = &[
			0b1101_1111, 0b1011_1111,
			0b1101_0110, 0b1011_1110,
			0b1101_1110, 0b1011_1010,
			0b1101_0011, 0b1011_1110,
			0b1100_1111, 0b1000_1010,
			0b1100_0010, 0b1000_0000,
			0b1100_0000, 0b1000_0000,
		];
		let str = JavaStr::from_str("\u{07ff}\u{05be}\u{07ba}\u{04fe}\u{03ca}\u{0080}\u{0000}");
		round_trip_java_str(vec, str)
	}

	#[test]
	fn three_bytes() -> Result<()> {
		let vec = &[
			0b1110_0000, 0b1010_0000, 0b1000_0000,
			0b1110_0001, 0b1000_1000, 0b1011_0100,
			0b1110_0100, 0b1000_1100, 0b1010_0001,
			0b1110_0111, 0b1010_0010, 0b1001_1101,
			0b1110_1100, 0b1010_1011, 0b1011_1110,
			0b1110_1011, 0b1010_1010, 0b1011_1110,
			0b1110_1111, 0b1011_1111, 0b1011_1111,
		];
		let str = JavaStr::from_str("\u{0800}\u{1234}\u{4321}\u{789d}\u{cafe}\u{babe}\u{ffff}");
		round_trip_java_str(vec, str)?;

		let vec = &[
			0b1110_1111, 0b1011_1111, 0b1011_1111,
			0b1110_1011, 0b1010_1010, 0b1011_1110,
			0b1110_1100, 0b1010_1011, 0b1011_1110,
			0b1110_0111, 0b1010_0010, 0b1001_1101,
			0b1110_0100, 0b1000_1100, 0b1010_0001,
			0b1110_0001, 0b1000_1000, 0b1011_0100,
			0b1110_0000, 0b1010_0000, 0b1000_0000,
		];
		let str = JavaStr::from_str("\u{ffff}\u{babe}\u{cafe}\u{789d}\u{4321}\u{1234}\u{0800}");
		round_trip_java_str(vec, str)
	}

	#[test]
	fn six_bytes() -> Result<()> {
		let vec = &[
			0b1110_1101, 0b1010_0000, 0b1000_0000, 0b1110_1101, 0b1011_0000, 0b1000_0000,
			0b1110_1101, 0b1010_0000, 0b1000_1000, 0b1110_1101, 0b1011_1101, 0b1000_0101,
			0b1110_1101, 0b1010_0100, 0b1001_0000, 0b1110_1101, 0b1011_1100, 0b1010_0001,
			0b1110_1101, 0b1010_0101, 0b1001_1110, 0b1110_1101, 0b1011_0010, 0b1001_1101,
			0b1110_1101, 0b1010_1011, 0b1010_1011, 0b1110_1101, 0b1011_1111, 0b1010_1011,
			0b1110_1101, 0b1010_1001, 0b1010_1111, 0b1110_1101, 0b1011_1001, 0b1010_0110,
			0b1110_1101, 0b1010_1111, 0b1011_1111, 0b1110_1101, 0b1011_1111, 0b1011_1111,
		];
		let str = JavaStr::from_str("\u{010000}\u{012345}\u{054321}\u{06789d}\u{0cafeb}\u{0abe66}\u{10ffff}");
		round_trip_java_str(vec, str)?;

		let vec = &[
			0b1110_1101, 0b1010_1111, 0b1011_1111, 0b1110_1101, 0b1011_1111, 0b1011_1111,
			0b1110_1101, 0b1010_1001, 0b1010_1111, 0b1110_1101, 0b1011_1001, 0b1010_0110,
			0b1110_1101, 0b1010_1011, 0b1010_1011, 0b1110_1101, 0b1011_1111, 0b1010_1011,
			0b1110_1101, 0b1010_0101, 0b1001_1110, 0b1110_1101, 0b1011_0010, 0b1001_1101,
			0b1110_1101, 0b1010_0100, 0b1001_0000, 0b1110_1101, 0b1011_1100, 0b1010_0001,
			0b1110_1101, 0b1010_0000, 0b1000_1000, 0b1110_1101, 0b1011_1101, 0b1000_0101,
			0b1110_1101, 0b1010_0000, 0b1000_0000, 0b1110_1101, 0b1011_0000, 0b1000_0000,
		];
		let str = JavaStr::from_str("\u{10ffff}\u{0abe66}\u{0cafeb}\u{06789d}\u{054321}\u{012345}\u{010000}");
		round_trip_java_str(vec, str)
	}

	#[test]
	fn unmatched_surrogate() -> Result<()> {
		let vec = vec![ 0b1110_1101, 0b1010_0000, 0b1000_0000 ];
		assert_eq!(from_string_to_vec(&from_vec_to_string(vec.clone())?), vec);
		let vec = vec![ 0b1110_1101, 0b1010_1010, 0b1011_1111 ];
		assert_eq!(from_string_to_vec(&from_vec_to_string(vec.clone())?), vec);
		let vec = vec![ 0b1110_1101, 0b1010_1111, 0b1011_1111 ];
		assert_eq!(from_string_to_vec(&from_vec_to_string(vec.clone())?), vec);
		let vec = vec![ 0b1110_1101, 0b1011_0000, 0b1000_0000 ];
		assert_eq!(from_string_to_vec(&from_vec_to_string(vec.clone())?), vec);
		let vec = vec![ 0b1110_1101, 0b1011_0101, 0b1010_1010 ];
		assert_eq!(from_string_to_vec(&from_vec_to_string(vec.clone())?), vec);
		let vec = vec![ 0b1110_1101, 0b1011_1111, 0b1011_1111 ];
		assert_eq!(from_string_to_vec(&from_vec_to_string(vec.clone())?), vec);

		Ok(())
	}
}
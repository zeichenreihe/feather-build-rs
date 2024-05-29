//! Methods for converting the string format used in the Java Virtual Machine Specification to and from
//! rust strings.
//!
//! The Java Virtual Machine Specification's string format is using a 2x3-format and storing `\0`
//! using two bytes.
//!
//! See <https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html#jvms-4.4.7> for the complete specification of
//! the string format used in the Java Virtual Machine Specification.

use anyhow::{bail, Context, Result};

/// Takes in a vec of data, tries to read it into a normal [`String`].
pub(crate) fn from_vec_to_string(vec: Vec<u8>) -> Result<String> {
	let mut iter = vec.into_iter();

	let mut s = String::new();

	while let Some(x) = iter.next() {
		let ch = if x >> 7 == 0 {
			// one byte
			x as u32
		} else if x >> 5 == 0b110 {
			// two byte
			let y = iter.next().context("unexpected end for second of two byte")?;

			if y >> 6 != 0b10 {
				bail!("got wrong second value for two byte unicode encoding: {x:#b} and {y:#b}");
			}

			((x as u32 & 0x1f) << 6) + (y as u32 & 0x3f)
		} else if x /* u */ == 0b1110_1101 {
			// six byte
			let u = x;
			let v = iter.next().context("unexpected end for second of six byte")?;
			let w = iter.next().context("unexpected end for third of six byte")?;
			let x = iter.next().context("unexpected end for fourth of six byte")?;
			let y = iter.next().context("unexpected end for fifth of six byte")?;
			let z = iter.next().context("unexpected end for sixth of six byte")?;

			if v >> 4 != 0b1010 {
				bail!("got wrong second value for six byte unicode encoding: {u:#b}, {v:#b}, {w:#b}, {x:#b}, {y:#b} and {z:#b}");
			}
			if w >> 6 != 0b10 {
				bail!("got wrong third value for six byte unicode encoding: {u:#b}, {v:#b}, {w:#b}, {x:#b}, {y:#b} and {z:#b}");
			}
			if x != 0b1110_1101 {
				bail!("got wrong fourth value for six byte unicode encoding: {u:#b}, {v:#b}, {w:#b}, {x:#b}, {y:#b} and {z:#b}");
			}

			0x10000 + ((v as u32 & 0x0f) << 16) + ((w as u32 & 0x3f) << 10) +
				((y as u32 & 0x0f) << 6) + (z as u32 & 0x3f)
		} else if x >> 4 == 0b1110 {
			// three byte
			let y = iter.next().context("unexpected end for second of three byte")?;
			let z = iter.next().context("unexpected end for third of three byte")?;

			if y >> 6 != 0b10 {
				bail!("got wrong second value for three byte unicode encoding: {x:#b}, {y:#b} and {z:#b}");
			}
			if z >> 6 != 0b10 {
				bail!("got wrong third value for three byte unicode encoding: {x:#b}, {y:#b} and {z:#b}");
			}

			((x as u32 & 0xf) << 12) + ((y as u32 & 0x3f) << 6) + (z & 0x3f) as u32
		} else {
			bail!("illegal byte in unicode string: {x:#b}")
		};

		let ch = char::from_u32(ch)
			.unwrap_or(char::REPLACEMENT_CHARACTER);

		s.push(ch);
	}

	Ok(s)
}

/// Takes in a string and writes it out into a vec.
///
/// As rust strings are always valid UTF-8, this operation cannot fail.
pub(crate) fn from_string_to_vec(string: &str) -> Vec<u8> {
	string.chars()
		.flat_map(|ch| {
			match u32::from(ch) {
				n @ 0x0001..=0x007F => vec![n as u8],
				n @ 0x0000 |
				n @ 0x0080..=0x07FF => vec![
					0b1100_0000 | ((0b0000_0111_1100_0000 & n) >> 6) as u8,
					0b1000_0000 |  (0b0000_0000_0011_1111 & n)       as u8
				],
				n @ 0x0800..=0xFFFF => vec![
					0b1110_0000 | ((0b1111_0000_0000_0000 & n) >> 12) as u8,
					0b1000_0000 | ((0b0000_1111_1100_0000 & n) >>  6) as u8,
					0b1000_0000 |  (0b0000_0000_0011_1111 & n)        as u8,
				],
				n @ 0x0001_0000.. => vec![
					0b1110_1101,
					0b1010_0000 | (((0b0001_1111_0000_0000_0000_0000 & n) >> 16) as u8 - 1),
					0b1000_0000 |  ((0b0000_0000_1111_1100_0000_0000 & n) >> 10) as u8,
					0b1110_1101,
					0b1011_0000 |  ((0b0000_0000_0000_0011_1100_0000 & n) >>  6) as u8,
					0b1000_0000 |   (0b0000_0000_0000_0000_0011_1111 & n)        as u8,
				],
			}
		})
		.collect()
}

#[cfg(test)]
mod testing {
	use pretty_assertions::assert_eq;
	use crate::jstring::{from_string_to_vec, from_vec_to_string};

	#[test]
	fn zero() {
		let vec = vec![0b1100_0000, 0b1000_0000, 0b1100_0000, 0b1000_0000, 0b1100_0000, 0b1000_0000];
		let str = "\0\0\0";

		assert_eq!(from_string_to_vec(str), vec);
		assert_eq!(from_vec_to_string(vec).unwrap(), str);
	}

	#[test]
	fn one_byte() {
		let vec: Vec<u8> = (0x0001..=0x007f).collect();
		let string: String = ('\u{0001}'..='\u{007f}').collect();

		assert_eq!(from_string_to_vec(&string), vec);
		assert_eq!(from_vec_to_string(vec).unwrap(), string);

		let vec: Vec<u8> = (0x0001..=0x007f).rev().collect();
		let string: String = ('\u{0001}'..='\u{007f}').rev().collect();

		assert_eq!(from_string_to_vec(&string), vec);
		assert_eq!(from_vec_to_string(vec).unwrap(), string);
	}

	#[test]
	fn two_bytes() {
		let vec = vec![
			0b1100_0000, 0b1000_0000,
			0b1100_0010, 0b1000_0000,
			0b1100_1111, 0b1000_1010,
			0b1101_0011, 0b1011_1110,
			0b1101_1110, 0b1011_1010,
			0b1101_0110, 0b1011_1110,
			0b1101_1111, 0b1011_1111,
		];
		let str = "\u{0000}\u{0080}\u{03ca}\u{04fe}\u{07ba}\u{05be}\u{07ff}";

		assert_eq!(from_string_to_vec(str), vec);
		assert_eq!(from_vec_to_string(vec.clone()).unwrap(), str);

		let vec = vec![
			0b1101_1111, 0b1011_1111,
			0b1101_0110, 0b1011_1110,
			0b1101_1110, 0b1011_1010,
			0b1101_0011, 0b1011_1110,
			0b1100_1111, 0b1000_1010,
			0b1100_0010, 0b1000_0000,
			0b1100_0000, 0b1000_0000,
		];
		let str = "\u{07ff}\u{05be}\u{07ba}\u{04fe}\u{03ca}\u{0080}\u{0000}";

		assert_eq!(from_string_to_vec(str), vec);
		assert_eq!(from_vec_to_string(vec.clone()).unwrap(), str);
	}

	#[test]
	fn three_bytes() {
		let vec = vec![
			0b1110_0000, 0b1010_0000, 0b1000_0000,
			0b1110_0001, 0b1000_1000, 0b1011_0100,
			0b1110_0100, 0b1000_1100, 0b1010_0001,
			0b1110_0111, 0b1010_0010, 0b1001_1101,
			0b1110_1100, 0b1010_1011, 0b1011_1110,
			0b1110_1011, 0b1010_1010, 0b1011_1110,
			0b1110_1111, 0b1011_1111, 0b1011_1111,
		];
		let str = "\u{0800}\u{1234}\u{4321}\u{789d}\u{cafe}\u{babe}\u{ffff}";

		assert_eq!(from_string_to_vec(str), vec);
		assert_eq!(from_vec_to_string(vec.clone()).unwrap(), str);

		let vec = vec![
			0b1110_1111, 0b1011_1111, 0b1011_1111,
			0b1110_1011, 0b1010_1010, 0b1011_1110,
			0b1110_1100, 0b1010_1011, 0b1011_1110,
			0b1110_0111, 0b1010_0010, 0b1001_1101,
			0b1110_0100, 0b1000_1100, 0b1010_0001,
			0b1110_0001, 0b1000_1000, 0b1011_0100,
			0b1110_0000, 0b1010_0000, 0b1000_0000,
		];
		let str = "\u{ffff}\u{babe}\u{cafe}\u{789d}\u{4321}\u{1234}\u{0800}";

		assert_eq!(from_string_to_vec(str), vec);
		assert_eq!(from_vec_to_string(vec.clone()).unwrap(), str);
	}

	#[test]
	fn six_bytes() {
		let vec = vec![
			0b1110_1101, 0b1010_0000, 0b1000_0000, 0b1110_1101, 0b1011_0000, 0b1000_0000,
			0b1110_1101, 0b1010_0000, 0b1000_1000, 0b1110_1101, 0b1011_1101, 0b1000_0101,
			0b1110_1101, 0b1010_0100, 0b1001_0000, 0b1110_1101, 0b1011_1100, 0b1010_0001,
			0b1110_1101, 0b1010_0101, 0b1001_1110, 0b1110_1101, 0b1011_0010, 0b1001_1101,
			0b1110_1101, 0b1010_1011, 0b1010_1011, 0b1110_1101, 0b1011_1111, 0b1010_1011,
			0b1110_1101, 0b1010_1001, 0b1010_1111, 0b1110_1101, 0b1011_1001, 0b1010_0110,
			0b1110_1101, 0b1010_1111, 0b1011_1111, 0b1110_1101, 0b1011_1111, 0b1011_1111,
		];
		let str = "\u{010000}\u{012345}\u{054321}\u{06789d}\u{0cafeb}\u{0abe66}\u{10ffff}";

		assert_eq!(from_vec_to_string(vec.clone()).unwrap(), str);
		assert_eq!(from_string_to_vec(str), vec);

		let vec = vec![
			0b1110_1101, 0b1010_1111, 0b1011_1111, 0b1110_1101, 0b1011_1111, 0b1011_1111,
			0b1110_1101, 0b1010_1001, 0b1010_1111, 0b1110_1101, 0b1011_1001, 0b1010_0110,
			0b1110_1101, 0b1010_1011, 0b1010_1011, 0b1110_1101, 0b1011_1111, 0b1010_1011,
			0b1110_1101, 0b1010_0101, 0b1001_1110, 0b1110_1101, 0b1011_0010, 0b1001_1101,
			0b1110_1101, 0b1010_0100, 0b1001_0000, 0b1110_1101, 0b1011_1100, 0b1010_0001,
			0b1110_1101, 0b1010_0000, 0b1000_1000, 0b1110_1101, 0b1011_1101, 0b1000_0101,
			0b1110_1101, 0b1010_0000, 0b1000_0000, 0b1110_1101, 0b1011_0000, 0b1000_0000,
		];
		let str = "\u{10ffff}\u{0abe66}\u{0cafeb}\u{06789d}\u{054321}\u{012345}\u{010000}";

		assert_eq!(from_vec_to_string(vec.clone()).unwrap(), str);
		assert_eq!(from_string_to_vec(str), vec);
	}

	// TODO: create a vec/string pari where x would fall into that 6 byte case but actually should go into that 3 byte one (and the other way around)
}
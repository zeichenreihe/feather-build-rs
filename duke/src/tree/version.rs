use std::cmp::Ordering;

/// Represents a class file version.
///
/// Use the associated constants (like [`Version::V1_1`]) if you want that version.
///
/// Take a look at [the list of class file versions](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.1-200-B.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
	pub(crate) major: u16,
	pub(crate) minor: u16,
}

impl Version {
	#[allow(unused)]
	pub const V1_1: Version = Version::new(45, 3);
	#[allow(unused)]
	pub const V1_2: Version = Version::new(46, 0);
	#[allow(unused)]
	pub const V1_3: Version = Version::new(47, 0);
	#[allow(unused)]
	pub const V1_4: Version = Version::new(48, 0);
	#[allow(unused)]
	pub const V1_5: Version = Version::new(49, 0);
	#[allow(unused)]
	pub const V1_6: Version = Version::new(50, 0);
	#[allow(unused)]
	pub const V1_7: Version = Version::new(51, 0);
	#[allow(unused)]
	pub const V1_8: Version = Version::new(52, 0);
	#[allow(unused)]
	pub const V9: Version = Version::new(53, 0);
	#[allow(unused)]
	pub const V10: Version = Version::new(54, 0);
	#[allow(unused)]
	pub const V11: Version = Version::new(55, 0);
	#[allow(unused)]
	pub const V12: Version = Version::new(56, 0);
	#[allow(unused)]
	pub const V13: Version = Version::new(57, 0);
	#[allow(unused)]
	pub const V14: Version = Version::new(58, 0);
	#[allow(unused)]
	pub const V15: Version = Version::new(59, 0);
	#[allow(unused)]
	pub const V16: Version = Version::new(60, 0);
	#[allow(unused)]
	pub const V17: Version = Version::new(61, 0);
	#[allow(unused)]
	pub const V18: Version = Version::new(62, 0);
	#[allow(unused)]
	pub const V19: Version = Version::new(63, 0);
	#[allow(unused)]
	pub const V20: Version = Version::new(64, 0);
	#[allow(unused)]
	pub const V21: Version = Version::new(65, 0);
	#[allow(unused)]
	pub const V22: Version = Version::new(66, 0);
	#[allow(unused)]
	pub const V23: Version = Version::new(67, 0);

	pub(crate) const fn new(major: u16, minor: u16) -> Version {
		Version { major, minor }
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> Ordering {
		self.major.cmp(&other.major)
			.then_with(|| self.minor.cmp(&other.minor))
	}
}

#[cfg(test)]
mod testing {
	use crate::tree::version::Version;

	#[test]
	fn test_cmp() {
		assert!(Version::V21 < Version::V22);
		assert!(Version::V21 < Version::V23);
		assert!(Version::V21 <= Version::V21);
		assert!(Version::V21 >= Version::V20);
		assert!(Version::V21 >= Version::V10);

		assert!(Version::V21 < Version::new(65, 1));
		assert!(Version::V22 > Version::new(65, 1));
		assert!(Version::new(65, 2) > Version::new(65, 1));
	}
}


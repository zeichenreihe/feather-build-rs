use pretty_assertions::assert_eq;
use raw_class_file::{AttributeInfo, ClassFile, CpInfo, FieldInfo, flags, MethodInfo};

#[test]
fn test() {
	let class = ClassFile {
		minor_version: 0,
		major_version: 52,
		constant_pool: vec![
			CpInfo::Utf8 { bytes: b"ThisClass".to_vec() },
			CpInfo::Class { name_index: 1 },
			CpInfo::Utf8 { bytes: b"ThatClass".to_vec() },
			CpInfo::Class { name_index: 3 },
			CpInfo::Utf8 { bytes: b"ThisInterface".to_vec() },
			CpInfo::Class { name_index: 5 },
			CpInfo::Utf8 { bytes: b"ThatInterface".to_vec() },
			CpInfo::Class { name_index: 7 },
			CpInfo::Utf8 { bytes: b"thisField".to_vec() },
			CpInfo::Utf8 { bytes: b"I".to_vec() },
			CpInfo::Utf8 { bytes: b"ConstantValue".to_vec() },
			CpInfo::Integer { bytes: 42 },
			CpInfo::Utf8 { bytes: b"thatField".to_vec() },
			CpInfo::Utf8 { bytes: b"F".to_vec() },
			CpInfo::Float { bytes: 42.3456f32.to_bits() },
			CpInfo::Utf8 { bytes: b"thisMethod".to_vec() },
			CpInfo::Utf8 { bytes: b"()I".to_vec() },
			CpInfo::Utf8 { bytes: b"thatMethod".to_vec() },
			CpInfo::Utf8 { bytes: b"()F".to_vec() },
		],
		access_flags: 0,
		this_class: 2,
		super_class: 4,
		interfaces: vec![6, 8],
		fields: vec![
			FieldInfo {
				access_flags: 0,
				name_index: 9,
				descriptor_index: 10,
				attributes: vec![
					AttributeInfo::ConstantValue {
						attribute_name_index: 11,
						constantvalue_index: 12,
					}
				],
			},
			FieldInfo {
				access_flags: 0,
				name_index: 13,
				descriptor_index: 14,
				attributes: vec![
					AttributeInfo::ConstantValue {
						attribute_name_index: 11,
						constantvalue_index: 15,
					}
				],
			},
		],
		methods: vec![
			MethodInfo {
				access_flags: flags::ACC_ABSTRACT,
				name_index: 16,
				descriptor_index: 17,
				attributes: vec![],
			},
			MethodInfo {
				access_flags: flags::ACC_ABSTRACT,
				name_index: 18,
				descriptor_index: 19,
				attributes: vec![],
			},
		],
		attributes: vec![],
	};

	let bytes = class.to_bytes();

	// Uncomment if you update the class on disk
	//std::io::Write::write_all(&mut std::fs::File::create("tests/simple_expected.class").unwrap(), &bytes).unwrap();

	let expected = include_bytes!("simple_expected.class");

	assert_eq!(bytes.as_slice(), expected.as_slice());

	let mut cursor = std::io::Cursor::new(bytes);
	let read = ClassFile::read(&mut cursor).unwrap();

	assert_eq!(class, read);

}
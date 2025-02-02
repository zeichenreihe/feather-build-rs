
use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use java_string::{JavaStr, JavaString};
use pretty_assertions::assert_eq;
use quill::remapper::{ARemapper, BRemapper, JarSuperProv};
use quill::tree::mappings::Mappings;

fn o<T>(s: &str) -> Result<T>
	where T: TryFrom<JavaString, Error=anyhow::Error>
{
	JavaString::from(s.to_owned()).try_into()
}

fn b<T: ?Sized>(s: &str) -> Result<&T>
	where for<'a> &'a T: TryFrom<&'a JavaStr, Error=anyhow::Error>
{
	<&JavaStr>::from(s).try_into()
}

#[test]
fn remap() -> Result<()> {
	let input_a = include_str!("remap_input.tiny");

	let input_a: Mappings<2> = quill::tiny_v2::read(input_a.as_bytes())?;


	// TODO: use duke-macros
	let super_classes_provider = JarSuperProv { super_classes: IndexMap::from([
		(o("classS1")?, IndexSet::from([
			o("classS2")?,
			o("classS3")?,
			o("classS4")?,
		])),
		(o("classS2")?, IndexSet::from([
			o("classS5")?,
		])),
		(o("classS3")?, IndexSet::from([
			o("classS5")?,
		])),
		(o("classS4")?, IndexSet::from([
			o("classS5")?,
		])),
		(o("classS5")?, IndexSet::from([
			o("java/lang/Object")?,
		])),
	]) };

	let from = input_a.get_namespace("namespaceA")?;
	let to = input_a.get_namespace("namespaceB")?;
	let remapper = input_a.remapper_b(from, to, &super_classes_provider)?;

	let class = |class: &'static str| -> Result<JavaString> {
		let a: JavaString = remapper.map_class(b(class)?).map(From::from)?;
		let b: JavaString = remapper.map_class_any(b(class)?).map(From::from)?;
		assert_eq!(a, b);
		Ok(a)
	};
	let class_arr = |class: &'static str| -> Result<JavaString> {
		remapper.map_class_any(b(class)?).map(From::from)
	};
	let field = |class: &'static str, field: &'static str, descriptor: &'static str| -> Result<(JavaString, JavaString, JavaString)> {
		let class = b(class)?;
		let field_name = b(field)?;
		let field_desc = b(descriptor)?;

		let class_new = remapper.map_class(class)?;
		let field_new = remapper.map_field(class, field_name, field_desc)?;

		Ok((class_new.into(), field_new.name.into(), field_new.desc.into()))
	};
	let method = |class: &'static str, method: &'static str, descriptor: &'static str| -> Result<(JavaString, JavaString, JavaString)> {
		let class = b(class)?;
		let method_name = b(method)?;
		let method_desc = b(descriptor)?;

		let class_new = remapper.map_class(class)?;
		let method_new = remapper.map_method(class, method_name, method_desc)?;

		Ok((class_new.into(), method_new.name.into(), method_new.desc.into()))
	};

	assert_eq!(class("classA1")?, "classB1");
	assert_eq!(class("classA2")?, "classB2");
	assert_eq!(class("classA2$innerA1")?, "classB2$innerB1");
	assert_eq!(class("classA3")?, "classB3");
	assert_eq!(class("classA4L")?, "classB4L");

	assert_eq!(class_arr("[LclassA1;")?, "[LclassB1;");
	assert_eq!(class_arr("[[LclassA2;")?, "[[LclassB2;");
	assert_eq!(class_arr("[[[LclassA2$innerA1;")?, "[[[LclassB2$innerB1;");
	assert_eq!(class_arr("[I")?, "[I");
	assert_eq!(class_arr("[[[D")?, "[[[D");

	assert_eq!(field("classA1", "field1A1", "I")?,
		("classB1".into(), "field1B1".into(), "I".into()));
	assert_eq!(field("classA1", "field1A2", "Ljava/lang/Object;")?,
		("classB1".into(), "field1B2".into(), "Ljava/lang/Object;".into()));
	assert_eq!(field("classA1", "field1A3", "LclassA1;")?,
		("classB1".into(), "field1B3".into(), "LclassB1;".into()));
	assert_eq!(field("classA1", "field1A4", "LclassA2$innerA1;")?,
		("classB1".into(), "field1B4".into(), "LclassB2$innerB1;".into()));
	assert_eq!(field("classA1", "field1A1", "[I")?,
		("classB1".into(), "field1B1".into(), "[I".into()));
	assert_eq!(field("classA1", "field1A2", "[Ljava/lang/Object;")?,
		("classB1".into(), "field1B2".into(), "[Ljava/lang/Object;".into()));
	assert_eq!(field("classA1", "field1A3", "[LclassA1;")?,
		("classB1".into(), "field1B3".into(), "[LclassB1;".into()));
	assert_eq!(field("classA1", "field1A4", "[LclassA2$innerA1;")?,
		("classB1".into(), "field1B4".into(), "[LclassB2$innerB1;".into()));
	assert_eq!(field("classA1", "field1A1", "[[[[I")?,
		("classB1".into(), "field1B1".into(), "[[[[I".into()));
	assert_eq!(field("classA1", "field1A2", "[[[[Ljava/lang/Object;")?,
		("classB1".into(), "field1B2".into(), "[[[[Ljava/lang/Object;".into()));
	assert_eq!(field("classA1", "field1A3", "[[[[LclassA1;")?,
		("classB1".into(), "field1B3".into(), "[[[[LclassB1;".into()));
	assert_eq!(field("classA1", "field1A4", "[[[[LclassA2$innerA1;")?,
		("classB1".into(), "field1B4".into(), "[[[[LclassB2$innerB1;".into()));

	assert_eq!(method("classA2", "method2A1", "()V")?,
		("classB2".into(), "method2B1".into(), "()V".into()));
	assert_eq!(method("classA2", "method2A2", "(I)I")?,
		("classB2".into(), "method2B2".into(), "(I)I".into()));
	assert_eq!(method("classA2", "method2A3", "(Ljava/lang/Integer;)Ljava/lang/Object;")?,
		("classB2".into(), "method2B3".into(), "(Ljava/lang/Integer;)Ljava/lang/Object;".into()));

	assert_eq!(method("classA2$innerA1", "<init>", "()V")?,
		("classB2$innerB1".into(), "<init>".into(), "()V".into()));

	assert_eq!(method("classA3", "method3A1", "(BCDFJSZ)V")?,
		("classB3".into(), "method3B1".into(), "(BCDFJSZ)V".into()));
	assert_eq!(method("classA3", "method3A2", "(LclassA1;LclassA2$innerA1;LclassA2;)LclassA3;")?,
		("classB3".into(), "method3B2".into(), "(LclassB1;LclassB2$innerB1;LclassB2;)LclassB3;".into()));
	assert_eq!(method("classA3", "method3A2", "([LclassA1;[LclassA2$innerA1;[LclassA2;)[LclassA3;")?,
		("classB3".into(), "method3B2".into(), "([LclassB1;[LclassB2$innerB1;[LclassB2;)[LclassB3;".into()));
	assert_eq!(method("classA3", "method3A2", "([LclassA2$innerA1;LclassA2$innerA1;[[[LclassA2;)[[[LclassA3;")?,
		("classB3".into(), "method3B2".into(), "([LclassB2$innerB1;LclassB2$innerB1;[[[LclassB2;)[[[LclassB3;".into()));
	assert_eq!(method("classA3", "method3A2", "([LclassA1;[[[LclassA2$innerA1;LclassA2;)[[[LclassA2$innerA1;")?,
		("classB3".into(), "method3B2".into(), "([LclassB1;[[[LclassB2$innerB1;LclassB2;)[[[LclassB2$innerB1;".into()));
	assert_eq!(method("classA3", "method3A3", "([B[C[D[F[J[S[Z)I")?,
		("classB3".into(), "method3B3".into(), "([B[C[D[F[J[S[Z)I".into()));
	assert_eq!(method("classA3", "method3A3", "([[B[[C[[D[[F[[J[[S[[Z)[[I")?,
		("classB3".into(), "method3B3".into(), "([[B[[C[[D[[F[[J[[S[[Z)[[I".into()));

	assert_eq!(field("classA4L", "field4A1", "LclassA4L;")?,
		("classB4L".into(), "field4B1".into(), "LclassB4L;".into()));
	assert_eq!(method("classA4L", "method4A1", "(LclassA4L;)LclassA4L;")?,
		("classB4L".into(), "method4B1".into(), "(LclassB4L;)LclassB4L;".into()));

	// Tests for super classes:
	assert_eq!(class("classS1")?, "classS1_");
	assert_eq!(class("classS2")?, "classS2_");
	assert_eq!(class("classS3")?, "classS3_");
	assert_eq!(class("classS4")?, "classS4_");
	assert_eq!(class("classS5")?, "classS5_");

	assert_eq!(field("classS1", "fieldFromS1", "I")?, ("classS1_".into(), "fieldFromS1_".into(), "I".into()));
	assert_eq!(field("classS1", "fieldFromS2", "I")?, ("classS1_".into(), "fieldFromS2_".into(), "I".into()));
	assert_eq!(field("classS1", "fieldFromS3", "I")?, ("classS1_".into(), "fieldFromS3_".into(), "I".into()));
	assert_eq!(field("classS1", "fieldFromS4", "I")?, ("classS1_".into(), "fieldFromS4_".into(), "I".into()));
	assert_eq!(field("classS1", "fieldFromS5", "I")?, ("classS1_".into(), "fieldFromS5_".into(), "I".into()));
	assert_eq!(field("classS2", "fieldFromS2", "I")?, ("classS2_".into(), "fieldFromS2_".into(), "I".into()));
	assert_eq!(field("classS2", "fieldFromS5", "I")?, ("classS2_".into(), "fieldFromS5_".into(), "I".into()));
	assert_eq!(field("classS3", "fieldFromS3", "I")?, ("classS3_".into(), "fieldFromS3_".into(), "I".into()));
	assert_eq!(field("classS3", "fieldFromS5", "I")?, ("classS3_".into(), "fieldFromS5_".into(), "I".into()));
	assert_eq!(field("classS4", "fieldFromS4", "I")?, ("classS4_".into(), "fieldFromS4_".into(), "I".into()));
	assert_eq!(field("classS4", "fieldFromS5", "I")?, ("classS4_".into(), "fieldFromS5_".into(), "I".into()));
	assert_eq!(field("classS5", "fieldFromS5", "I")?, ("classS5_".into(), "fieldFromS5_".into(), "I".into()));

	assert_eq!(method("classS1", "methodFromS1", "(I)I")?, ("classS1_".into(), "methodFromS1_".into(), "(I)I".into()));
	assert_eq!(method("classS1", "methodFromS2", "(I)I")?, ("classS1_".into(), "methodFromS2_".into(), "(I)I".into()));
	assert_eq!(method("classS1", "methodFromS3", "(I)I")?, ("classS1_".into(), "methodFromS3_".into(), "(I)I".into()));
	assert_eq!(method("classS1", "methodFromS4", "(I)I")?, ("classS1_".into(), "methodFromS4_".into(), "(I)I".into()));
	assert_eq!(method("classS1", "methodFromS5", "(I)I")?, ("classS1_".into(), "methodFromS5_".into(), "(I)I".into()));
	assert_eq!(method("classS2", "methodFromS2", "(I)I")?, ("classS2_".into(), "methodFromS2_".into(), "(I)I".into()));
	assert_eq!(method("classS2", "methodFromS5", "(I)I")?, ("classS2_".into(), "methodFromS5_".into(), "(I)I".into()));
	assert_eq!(method("classS3", "methodFromS3", "(I)I")?, ("classS3_".into(), "methodFromS3_".into(), "(I)I".into()));
	assert_eq!(method("classS3", "methodFromS5", "(I)I")?, ("classS3_".into(), "methodFromS5_".into(), "(I)I".into()));
	assert_eq!(method("classS4", "methodFromS4", "(I)I")?, ("classS4_".into(), "methodFromS4_".into(), "(I)I".into()));
	assert_eq!(method("classS4", "methodFromS5", "(I)I")?, ("classS4_".into(), "methodFromS5_".into(), "(I)I".into()));
	assert_eq!(method("classS5", "methodFromS5", "(I)I")?, ("classS5_".into(), "methodFromS5_".into(), "(I)I".into()));

	// TODO: another test method: also test if failures are there

	Ok(())
}
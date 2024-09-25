
use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use java_string::JavaString;
use pretty_assertions::assert_eq;
use duke::tree::class::{ClassName, ClassNameSlice};
use duke::tree::field::{FieldDescriptorSlice, FieldNameSlice};
use duke::tree::method::{MethodDescriptorSlice, MethodNameSlice};
use quill::remapper::{ARemapper, BRemapper, JarSuperProv};
use quill::tree::mappings::Mappings;

#[test]
fn remap() -> Result<()> {
	let input_a = include_str!("remap_input.tiny");

	let input_a: Mappings<2> = quill::tiny_v2::read(input_a.as_bytes())?;

	let super_classes_provider = JarSuperProv { super_classes: IndexMap::from([
		// SAFETY: is a valid class name
		(unsafe { ClassName::from_inner_unchecked("classS1".to_owned().into()) }, IndexSet::from([
			// SAFETY: is a valid class name
			unsafe { ClassName::from_inner_unchecked("classS2".to_owned().into()) },
			// SAFETY: is a valid class name
			unsafe { ClassName::from_inner_unchecked("classS3".to_owned().into()) },
			// SAFETY: is a valid class name
			unsafe { ClassName::from_inner_unchecked("classS4".to_owned().into()) },
		])),
		// SAFETY: is a valid class name
		(unsafe { ClassName::from_inner_unchecked("classS2".to_owned().into()) }, IndexSet::from([
			// SAFETY: is a valid class name
			unsafe { ClassName::from_inner_unchecked("classS5".to_owned().into()) },
		])),
		// SAFETY: is a valid class name
		(unsafe { ClassName::from_inner_unchecked("classS3".to_owned().into()) }, IndexSet::from([
			// SAFETY: is a valid class name
			unsafe { ClassName::from_inner_unchecked("classS5".to_owned().into()) },
		])),
		// SAFETY: is a valid class name
		(unsafe { ClassName::from_inner_unchecked("classS4".to_owned().into()) }, IndexSet::from([
			// SAFETY: is a valid class name
			unsafe { ClassName::from_inner_unchecked("classS5".to_owned().into()) },
		])),
		// SAFETY: is a valid class name
		(unsafe { ClassName::from_inner_unchecked("classS5".to_owned().into()) }, IndexSet::from([
			// SAFETY: is a valid class name
			unsafe { ClassName::from_inner_unchecked("java/lang/Object".to_owned().into()) },
		])),
	]) };

	let from = input_a.get_namespace("namespaceA")?;
	let to = input_a.get_namespace("namespaceB")?;
	let remapper = input_a.remapper_b(from, to, &super_classes_provider)?;

	let class = |class: &'static str| -> Result<JavaString> {
		// SAFETY: below are only valid class names
		let class = unsafe { ClassNameSlice::from_inner_unchecked(class.into()) };

		let class_new = remapper.map_class(class)?;

		Ok(class_new.into())
	};
	let field = |class: &'static str, field: &'static str, descriptor: &'static str| -> Result<(JavaString, JavaString, JavaString)> {
		// SAFETY: below are only valid class names
		let class = unsafe { ClassNameSlice::from_inner_unchecked(class.into()) };
		// SAFETY: below are only valid field names
		let field_name = unsafe { FieldNameSlice::from_inner_unchecked(field.into()) };
		// SAFETY: below are only valid field descs
		let field_desc = unsafe { FieldDescriptorSlice::from_inner_unchecked(descriptor.into()) };

		let class_new = remapper.map_class(class)?;
		let field_new = remapper.map_field(class, field_name, field_desc)?;

		Ok((class_new.into(), field_new.name.into(), field_new.desc.into()))
	};
	let method = |class: &'static str, method: &'static str, descriptor: &'static str| -> Result<(JavaString, JavaString, JavaString)> {
		// SAFETY: below are only valid class names
		let class = unsafe { ClassNameSlice::from_inner_unchecked(class.into()) };
		// SAFETY: below are only valid method names
		let method_name = unsafe { MethodNameSlice::from_inner_unchecked(method.into()) };
		// SAFETY: below are only valid method descs
		let method_desc = unsafe { MethodDescriptorSlice::from_inner_unchecked(descriptor.into()) };

		let class_new = remapper.map_class(class)?;
		let method_new = remapper.map_method(class, method_name, method_desc)?;

		Ok((class_new.into(), method_new.name.into(), method_new.desc.into()))
	};

	assert_eq!(class("classA1")?, "classB1");
	assert_eq!(class("classA2")?, "classB2");
	assert_eq!(class("classA2$innerA1")?, "classB2$innerB1");
	assert_eq!(class("classA3")?, "classB3");
	assert_eq!(class("classA4L")?, "classB4L");

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
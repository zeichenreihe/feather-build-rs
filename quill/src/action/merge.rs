use std::fmt::Debug;
use anyhow::{bail, Context, Result};
use java_string::JavaStr;
use crate::tree::names::{Names, Namespaces};
use crate::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, MappingInfo, Mappings, MethodMapping, MethodNowodeMapping, ParameterMapping, ParameterNowodeMapping};
use crate::tree::NodeJavadocInfo;
use super::diff_mappings::diff_and_merge::*;

fn merge_javadoc<Target, Javadoc>(ab: Combination<&Target>) -> Result<Option<Javadoc>>
	where
		Target: NodeJavadocInfo<Option<Javadoc>>,
		Javadoc: Clone + Debug + PartialEq,
{
	Ok(match ab.map(NodeJavadocInfo::get_node_javadoc_info) {
		Combination::A(a) => a.clone(),
		Combination::B(b) => b.clone(),
		Combination::AB(a, b) => match (a, b) {
			(None, None) => None,
			(None, Some(b)) => Some(b.clone()),
			(Some(a), None) => Some(a.clone()),
			(Some(a), Some(b)) if a == b => Some(a.clone()),
			(Some(a), Some(b)) => bail!("cannot merge: both left {a:?} and right {b:?} are given"),
		},
	})
}

fn merge_namespaces(a: &Namespaces<2>, b: &Namespaces<2>) -> Result<Namespaces<3>> {
	let a: &[String; 2] = a.into();
	let b: &[String; 2] = b.into();

	if a[0] != b[0] {
		bail!("cannot merge namespaces {a:?} and {b:?}: first namespaces don't match up");
	}

	let [a0, a1] = a.clone();
	let [_, b1] = b.clone();

	[a0, a1, b1].try_into()
}

fn merge_names<Name>(ab: Combination<&Names<2, Name>>) -> Result<Names<3, Name>>
	where
		Name: Debug + Clone + PartialEq + AsRef<JavaStr>,
{
	match ab.map(|names| <&[Option<Name>; 2]>::from(names).clone()) {
		Combination::A([a0, a1]) => [a0, a1, None],
		Combination::B([b0, b1]) => [b0, None, b1],
		Combination::AB(a, b) if a[0] != b[0] => bail!("cannot merge {a:?} and {b:?}: the first names must match up"),
		Combination::AB([a0, a1], [_, b1]) => [a0, a1, b1],
	}
		.try_into()
}

fn merge_equal<T>(ab: Combination<&T>) -> Result<T>
	where
		T: Clone + PartialEq + Debug,
{
	Ok(match ab {
		Combination::A(a) => a,
		Combination::B(b) => b,
		Combination::AB(a, b) if a != b => bail!("cannot merge {a:?} and {b:?}: expected them to be equal, but they are not equal"),
		Combination::AB(a, _) => a,
	}.clone())
}

impl Mappings<2> {
	// TODO: docs
	pub fn merge(a: &Mappings<2>, b: &Mappings<2>) -> Result<Mappings<3>> {
		let ab = Combination::AB(a, b);
		Ok(Mappings {
			info: MappingInfo {
				namespaces: merge_namespaces(&a.info.namespaces, &b.info.namespaces).context("failed to merge namespaces")?,
			},
			classes: zip_map_combination(
				ab.map(|x| &x.classes),
				|ab| Ok(ClassNowodeMapping {
					info: ClassMapping {
						names: merge_names(ab.map(|x| &x.info.names)).context("cannot merge class names")?,
					},
					fields: zip_map_combination(
						ab.map(|x| &x.fields),
						|ab| Ok(FieldNowodeMapping {
							info: FieldMapping {
								desc: merge_equal(ab.map(|x| &x.info.desc)).context("cannot merge field descriptors")?,
								names: merge_names(ab.map(|x| &x.info.names)).context("cannot merge field names")?,
							},
							javadoc: merge_javadoc(ab).context("cannot merge field javadoc")?,
						})
					)?,
					methods: zip_map_combination(
						ab.map(|x| &x.methods),
						|ab| Ok(MethodNowodeMapping {
							info: MethodMapping {
								desc: merge_equal(ab.map(|x| &x.info.desc)).context("cannot merge method descriptors")?,
								names: merge_names(ab.map(|x| &x.info.names)).context("cannot merge method names")?,
							},
							parameters: zip_map_combination(
								ab.map(|x| &x.parameters),
								|ab| Ok(ParameterNowodeMapping {
									info: ParameterMapping {
										index: merge_equal(ab.map(|x| &x.info.index)).context("cannot merge parameter indices")?,
										names: merge_names(ab.map(|x| &x.info.names)).context("cannot merge parameter names")?,
									},
									javadoc: merge_javadoc(ab).context("cannot merge parameter javadoc")?,
								})
							)?,
							javadoc: merge_javadoc(ab).context("cannot merge method javadoc")?,
						})
					)?,
					javadoc: merge_javadoc(ab).context("cannot merge class javadoc")?,
				})
			)?,
			javadoc: merge_javadoc(ab).context("cannot merge mappings javadoc")?,
		})
	}
}

#[cfg(test)]
mod testing {
	// TODO: consider testing internals
}
//! A module containing methods to read and write mappings in the "Tiny v2" format.
//!
//! # Reading
//! You can read a `.tiny` file using the [`read_file`] method, by passing a path.
//! If you already have a [`Read`]er, you can use the [`read`] method.
//!
//! It's recommended to check that the namespaces are indeed the ones expected.
//! See [`Namespaces::check_that`] for more info.
//!
//! # Writing
//! For writing `.tiny` files, there are the [`write`][fn@write] as well as the [`write_vec`] and [`write_string`] methods.
//!
//! Note that there's also a [`write_zip_file`] method, tho most likely it's not what you actually want.
//!
//! Note that all writing sorts the tiny files.

use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::ZipWriter;
use class_file::tree::class::ClassName;
use class_file::tree::field::FieldName;
use class_file::tree::method::{MethodName, ParameterName};
use crate::tiny_v2_line::{Line, WithMoreIdentIter};
use crate::tree::mappings::{ClassMapping, FieldMapping, JavadocMapping, MappingInfo, MethodMapping, ParameterMapping, ClassNowodeMapping, FieldNowodeMapping, Mappings, MethodNowodeMapping, ParameterNowodeMapping};
use crate::tree::names::{Names, Namespaces};
use crate::tree::NodeInfo;

/// Reads a `.tiny` file (tiny v2), by opening the file given by the path.
///
/// It's recommended to check that the namespaces are indeed the ones expected.
/// See [`Namespaces::check_that`] for more info.
///
/// ```
/// use std::path::Path;
/// use mappings_rw::tree::mappings::Mappings;
///
/// let path = Path::new("tests/remap_input.tiny");
/// let mappings: Mappings<2> = mappings_rw::tiny_v2::read_file(path).unwrap();
///
/// mappings.info.namespaces.check_that(["namespaceA", "namespaceB"]).unwrap();
/// ```
pub fn read_file<const N: usize>(path: impl AsRef<Path> + Debug) -> Result<Mappings<N>> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("failed to read mappings file {path:?}"))
}

#[allow(clippy::tabs_in_doc_comments)]
/// Reads the tiny v2 format, from the given reader.
///
/// It's recommended to check that the namespaces are indeed the ones expected.
/// See [`Namespaces::check_that`] for more info.
///
/// ```
/// use mappings_rw::tree::mappings::Mappings;
/// let string = "\
/// tiny	2	0	namespaceA	namespaceB	namespaceC
/// c	A	B	C
/// 	f	LA;	a	b	c
/// 	m	(LA;)V	a	b	c
/// ";
///
/// let reader = &mut string.as_bytes();
/// let mappings: Mappings<3> = mappings_rw::tiny_v2::read(reader).unwrap();
///
/// mappings.info.namespaces.check_that(["namespaceA", "namespaceB", "namespaceC"]).unwrap();
/// ```
pub fn read<const N: usize>(reader: impl Read) -> Result<Mappings<N>> {
	if N < 2 {
		bail!("must read at least two namespaces, {N} is less than that");
	}

	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| -> Result<Line> {
			Line::new(line_number + 1, line?)
		})
		.peekable();

	let mut header = lines.next().context("no header")??;

	if header.first_field != "tiny" || header.next()? != "2" || header.next()? != "0" {
		bail!("header version isn't tiny v2.0");
	}

	let namespaces = header.list()?.into();

	let mut mappings = Mappings::new(MappingInfo { namespaces });

	let mut iter = WithMoreIdentIter::new(&mut lines);
	while let Some(line) = iter.next().transpose()? {
		if line.first_field == "c" {
			let names = line.list()?.map(ClassName::from).into();

			let mapping = ClassMapping { names };

			let mut class: ClassNowodeMapping<N> = ClassNowodeMapping::new(mapping);

			let mut iter = iter.next_level();
			while let Some(mut line) = iter.next().transpose()? {
				if line.first_field == "f" {
					let desc = line.next()?.into();
					let names = line.list()?.map(FieldName::from).into();

					let mapping = FieldMapping { desc, names };

					let mut field: FieldNowodeMapping<N> = FieldNowodeMapping::new(mapping);

					let mut iter = iter.next_level();
					while let Some(line) = iter.next().transpose()? {
						if line.first_field == "c" {
							let comment = JavadocMapping(line.end()?);
							if field.javadoc.replace(comment).is_some() {
								bail!("only one comment per field is allowed");
							}
						}
					}

					class.add_field(field)?;
				} else if line.first_field == "m" {
					let desc = line.next()?.into();
					let names = line.list()?.map(MethodName::from).into();

					let mapping = MethodMapping { desc, names };

					let mut method: MethodNowodeMapping<N> = MethodNowodeMapping::new(mapping);

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "p" {
							let index = line.next()?.parse()?;
							let names = line.list()?.map(ParameterName::from).into();

							let mapping = ParameterMapping { index, names };

							let mut parameter: ParameterNowodeMapping<N> = ParameterNowodeMapping::new(mapping);

							let mut iter = iter.next_level();
							while let Some(line) = iter.next().transpose()? {
								if line.first_field == "c" {
									let comment = JavadocMapping(line.end()?);
									if parameter.javadoc.replace(comment).is_some() {
										bail!("only one comment per parameter is allowed");
									}
								}
							}

							method.add_parameter(parameter)?;
						} else if line.first_field == "c" {
							let comment = JavadocMapping(line.end()?);
							if method.javadoc.replace(comment).is_some() {
								bail!("only one comment per method is allowed");
							}
						}
					}

					class.add_method(method)?;
				} else if line.first_field == "c" {
					let comment = JavadocMapping(line.end()?);
					if class.javadoc.replace(comment).is_some() {
						bail!("only one comment per class is allowed");
					}
				}
			}

			mappings.add_class(class)?;
		}
	}

	if let Some(line) = lines.next() {
		bail!("expected end of input, got: {line:?}");
	}

	Ok(mappings)
}

/// Writes the given mappings into a zip file, returning the zip file buffer.
///
/// This method places the mappings into the file `mappings/mappings.tiny` in the zip file.
pub fn write_zip_file<const N: usize>(mappings: &Mappings<N>) -> Result<Vec<u8>> {
	let mut zip = ZipWriter::new(Cursor::new(Vec::new()));

	zip.start_file("mappings/mappings.tiny", FileOptions::default())?;

	write(mappings, &mut zip)?;

	Ok(zip.finish()?.into_inner())
}

#[allow(clippy::tabs_in_doc_comments)]
/// Writes the given mappings into a `String`, in the tiny v2 format.
///
/// If the mapping somehow produces invalid UTF-8, then this method fails.
///
/// Note that this currently sorts the classes, fields, methods and parameters.
///
/// The example from the [`write`][fn@write] method could look like this:
/// ```
/// # use pretty_assertions::assert_eq;
/// use mappings_rw::tree::mappings::Mappings;
/// let input = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	D	E
/// c	A	B
/// 	f	I	bIsAfterA	d
/// 	m	()V	methodB	methodBSecondName
/// 	f	I	aIsBeforeB	c
/// 	m	()V	methodA	methodASecondName
/// ";
///
/// let reader = &mut input.as_bytes();
/// let mappings: Mappings<2> = mappings_rw::tiny_v2::read(reader).unwrap();
///
/// let written = mappings_rw::tiny_v2::write_string(&mappings).unwrap();
///
/// let output = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	A	B
/// 	f	I	aIsBeforeB	c
/// 	f	I	bIsAfterA	d
/// 	m	()V	methodA	methodASecondName
/// 	m	()V	methodB	methodBSecondName
/// c	D	E
/// ";
///
/// assert_eq!(written, output);
/// ```
///
/// This method is of most use in test cases, where you also use the `pretty_assertions` crate for viewing string diffs.
pub fn write_string<const N: usize>(mappings: &Mappings<N>) -> Result<String> {
	let vec = write_vec(mappings)?;
	String::from_utf8(vec).context("failed to convert file to utf8")
}

#[allow(clippy::tabs_in_doc_comments)]
/// Writes the given mappings into a `Vec<u8>`, in the tiny v2 format.
///
/// Note that this currently sorts the classes, fields, methods and parameters.
///
/// The example from the [`write`][fn@write] method could look like this:
/// ```
/// # use pretty_assertions::assert_eq;
/// use mappings_rw::tree::mappings::Mappings;
/// let input = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	D	E
/// c	A	B
/// 	f	I	bIsAfterA	d
/// 	m	()V	methodB	methodBSecondName
/// 	f	I	aIsBeforeB	c
/// 	m	()V	methodA	methodASecondName
/// ";
///
/// let reader = &mut input.as_bytes();
/// let mappings: Mappings<2> = mappings_rw::tiny_v2::read(reader).unwrap();
///
/// let buf = mappings_rw::tiny_v2::write_vec(&mappings).unwrap();
/// let written = String::from_utf8(buf).unwrap();
///
/// let output = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	A	B
/// 	f	I	aIsBeforeB	c
/// 	f	I	bIsAfterA	d
/// 	m	()V	methodA	methodASecondName
/// 	m	()V	methodB	methodBSecondName
/// c	D	E
/// ";
///
/// assert_eq!(written, output);
/// ```
///
/// Note that there's also the helper method [`write_string`] that also tries to convert that `Vec<u8>` into a `String`.
pub fn write_vec<const N: usize>(mappings: &Mappings<N>) -> Result<Vec<u8>> {
	let mut vec = Vec::new();
	write(mappings, &mut vec)?;
	Ok(vec)
}

fn write_namespaces<const N: usize>(w: &mut impl Write, namespaces: &Namespaces<N>) -> Result<()> {
	for namespace in namespaces.names() {
		write!(w, "\t{namespace}")?;
	}
	writeln!(w)?;
	Ok(())
}

fn write_names<const N: usize, T>(w: &mut impl Write, names: &Names<N, T>) -> Result<()>
	where
			for<'a> &'a str: From<&'a T>,
{
	for name in names.names() {
		let name = name.map(|x| x.into());
		write!(w, "\t{}", name.unwrap_or(""))?;
	}
	writeln!(w)?;
	Ok(())
}

#[allow(clippy::tabs_in_doc_comments)]
/// Writes the given mappings to the given writer, in the tiny v2 format.
///
/// Note that this currently sorts the classes, fields, methods and parameters.
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use mappings_rw::tree::mappings::Mappings;
/// let input = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	D	E
/// c	A	B
/// 	f	I	bIsAfterA	d
/// 	m	()V	methodB	methodBSecondName
/// 	f	I	aIsBeforeB	c
/// 	m	()V	methodA	methodASecondName
/// ";
///
/// let reader = &mut input.as_bytes();
/// let mappings: Mappings<2> = mappings_rw::tiny_v2::read(reader).unwrap();
///
/// let mut buf: Vec<u8> = Vec::new();
/// mappings_rw::tiny_v2::write(&mappings, &mut buf).unwrap();
/// let written = String::from_utf8(buf).unwrap();
///
/// let output = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	A	B
/// 	f	I	aIsBeforeB	c
/// 	f	I	bIsAfterA	d
/// 	m	()V	methodA	methodASecondName
/// 	m	()V	methodB	methodBSecondName
/// c	D	E
/// ";
///
/// assert_eq!(written, output);
/// ```
///
/// Note that there are also the helper methods [`write_vec`] for writing into a `Vec<u8>` directly,
/// and the helper method [`write_string`] that also tries to convert that `Vec<u8>` into a `String`.
pub fn write<const N: usize>(mappings: &Mappings<N>, w: &mut impl Write) -> Result<()> {
	write!(w, "tiny\t2\t0")?;
	write_namespaces(w, &mappings.info.namespaces)?;

	if let Some(ref comment) = mappings.javadoc {
		writeln!(w, "\tc\t{}", comment.0)?;
	}

	let mut classes: Vec<_> = mappings.classes.values().collect();
	classes.sort_by_key(|x| &x.info);
	for class in classes {
		write!(w, "c")?;
		write_names(w, &class.info.names)?;

		if let Some(ref comment) = class.javadoc {
			writeln!(w, "\tc\t{}", comment.0)?;
		}

		let mut fields: Vec<_> = class.fields.values().collect();
		fields.sort_by_key(|x| &x.info);
		for field in fields {
			write!(w, "\tf\t{}", field.info.desc.as_str())?;
			write_names(w, &field.info.names)?;

			if let Some(ref comment) = field.javadoc {
				writeln!(w, "\t\tc\t{}", comment.0)?;
			}
		}

		let mut methods: Vec<_> = class.methods.values().collect();
		methods.sort_by_key(|x| &x.info);
		for method in methods {
			write!(w, "\tm\t{}", method.info.desc.as_str())?;
			write_names(w, &method.info.names)?;

			if let Some(ref comment) = method.javadoc {
				writeln!(w, "\t\tc\t{}", comment.0)?;
			}

			let mut parameters: Vec<_> = method.parameters.values().collect();
			parameters.sort_by_key(|x| &x.info);
			for parameter in parameters {
				write!(w, "\t\tp\t{}", parameter.info.index)?;
				write_names(w, &parameter.info.names)?;

				if let Some(ref comment) = parameter.javadoc {
					writeln!(w, "\t\t\tc\t{}", comment.0)?;
				}
			}
		}
	}

	Ok(())
}
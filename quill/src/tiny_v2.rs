//! Functions to read and write mappings in the "Tiny v2" format.
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
//! Note that all writing sorts the tiny files.

use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use duke::tree::class::ClassName;
use duke::tree::field::FieldName;
use duke::tree::method::{MethodName, ParameterName};
use crate::lines::tiny_line::TinyLine;
use crate::lines::{Line, WithMoreIdentIter};
use crate::tree::mappings::{ClassMapping, FieldMapping, JavadocMapping, MappingInfo, MethodMapping, ParameterMapping, ClassNowodeMapping, FieldNowodeMapping, Mappings, MethodNowodeMapping, ParameterNowodeMapping};
use crate::tree::names::{Names, Namespaces};
use crate::tree::NodeInfo;

/// Reads a `.tiny` file (tiny v2), by opening the file given by the path.
///
/// It's recommended to check that the namespaces are indeed the ones expected.
/// See [`Namespaces::check_that`] for more info.
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use std::path::Path;
/// use quill::tree::mappings::Mappings;
///
/// let path = Path::new("tests/read_file_input_tiny_v2.txt");
/// let mappings: Mappings<2> = quill::tiny_v2::read_file(path).unwrap();
///
/// mappings.info.namespaces.check_that(["namespaceA", "namespaceB"]).unwrap();
/// assert_eq!(mappings.classes.len(), 2);
/// ```
pub fn read_file<const N: usize>(path: impl AsRef<Path>) -> Result<Mappings<N>> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("failed to read mappings file {:?} as tiny v2 file", path.as_ref()))
}

#[allow(clippy::tabs_in_doc_comments)]
/// Reads the tiny v2 format, from the given reader.
///
/// It's recommended to check that the namespaces are indeed the ones expected.
/// See [`Namespaces::check_that`] for more info.
///
/// ```
/// # use pretty_assertions::assert_eq;
/// use quill::tree::mappings::Mappings;
/// let string = "\
/// tiny	2	0	namespaceA	namespaceB	namespaceC
/// c	A	B	C
/// 	f	LA;	a	b	c
/// 	m	(LA;)V	a	b	c
/// ";
///
/// let reader = &mut string.as_bytes();
/// let mappings: Mappings<3> = quill::tiny_v2::read(reader).unwrap();
///
/// mappings.info.namespaces.check_that(["namespaceA", "namespaceB", "namespaceC"]).unwrap();
/// assert_eq!(mappings.classes.len(), 1);
/// ```
pub fn read<const N: usize>(reader: impl Read) -> Result<Mappings<N>> {
	if N < 2 {
		bail!("must read at least two namespaces, {N} is less than that");
	}

	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| -> Result<TinyLine> {
			TinyLine::new(line_number + 1, &line?)
		})
		.peekable();

	let mut header = lines.next().context("no header line")??;
	let header_line_number = header.get_line_number();

	if header.first_field != "tiny" || header.next()? != "2" || header.next()? != "0" {
		bail!("header version isn't tiny v2.0, in line {header:?}");
	}

	let namespaces = header.list()?.try_into()
		.with_context(|| anyhow!("on line {header_line_number}"))?;

	let mut mappings = Mappings::new(MappingInfo { namespaces });

	WithMoreIdentIter::new(&mut lines).on_every_line(|iter, line| {
		if line.first_field == "c" {
			let names = line.list()?.map(ClassName::from).try_into()?;
			let mapping = ClassMapping { names };
			let class: ClassNowodeMapping<N> = ClassNowodeMapping::new(mapping);
			let class = mappings.add_class(class)?;

			iter.next_level().on_every_line(|iter, mut line| {
				if line.first_field == "f" {
					let desc = line.next()?.into();
					let names = line.list()?.map(FieldName::from).try_into()?;
					let mapping = FieldMapping { desc, names };
					let field: FieldNowodeMapping<N> = FieldNowodeMapping::new(mapping);
					let field = class.add_field(field)?;

					iter.next_level().on_every_line(|_, line| {
						if line.first_field == "c" {
							add_comment(&mut field.javadoc, line)
						} else {
							Ok(())
						}
					}).context("reading field sub-sections")
				} else if line.first_field == "m" {
					let desc = line.next()?.into();
					let names = line.list()?.map(MethodName::from).try_into()?;
					let mapping = MethodMapping { desc, names };
					let method: MethodNowodeMapping<N> = MethodNowodeMapping::new(mapping);
					let method = class.add_method(method)?;

					iter.next_level().on_every_line(|iter, mut line| {
						if line.first_field == "p" {
							let index = line.next()?.parse()?;
							let names = line.list()?.map(ParameterName::from).try_into()?;
							let mapping = ParameterMapping { index, names };
							let parameter: ParameterNowodeMapping<N> = ParameterNowodeMapping::new(mapping);
							let parameter = method.add_parameter(parameter)?;

							iter.next_level().on_every_line(|_, line| {
								if line.first_field == "c" {
									add_comment(&mut parameter.javadoc, line)
								} else {
									Ok(())
								}
							}).context("reading parameter sub-sections")
						} else if line.first_field == "c" {
							add_comment(&mut method.javadoc, line)
						} else {
							Ok(())
						}
					}).context("reading method sub-sections")
				} else if line.first_field == "c" {
					add_comment(&mut class.javadoc, line)
				} else {
					Ok(())
				}
			}).context("reading class sub-sections")
		} else {
			Ok(())
		}
	}).context("reading lines")?;

	if let Some(line) = lines.next() {
		bail!("expected end of input, got: {line:?}");
	}

	Ok(mappings)
}

fn add_comment(javadoc: &mut Option<JavadocMapping>, line: TinyLine) -> Result<()> {
	let comment = JavadocMapping(line.end()?);
	if let Some(javadoc) = javadoc {
		bail!("only one comment is allowed, got {javadoc:?} and {comment:?}")
	} else {
		*javadoc = Some(comment);
		Ok(())
	}
}

#[allow(clippy::tabs_in_doc_comments)]
/// Writes the given mappings into a `String`, in the tiny v2 format.
///
/// If the mapping somehow produces invalid UTF-8, then this method fails.
///
/// This is equivalent to first calling [`write_vec`] and then [`String::from_utf8`].
///
/// This method is of most use in test cases, where you also use the `pretty_assertions` crate for viewing string diffs.
pub fn write_string<const N: usize>(mappings: &Mappings<N>) -> Result<String> {
	let vec = write_vec(mappings)?;
	String::from_utf8(vec).context("failed to convert written mappings to utf8")
}

#[allow(clippy::tabs_in_doc_comments)]
/// Writes the given mappings into a `Vec<u8>`, in the tiny v2 format.
///
/// This is equivalent to letting [`write`][fn@write] write into a `Vec<u8>`.
///
/// Note that there's also the helper method [`write_string`] that also tries to convert the `Vec<u8>` into a `String`.
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

fn write_names<const N: usize>(w: &mut impl Write, names: &Names<N, impl AsRef<str>>) -> Result<()> {
	for name in names.names() {
		let name = name.as_ref().map(|x| x.as_ref());
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
/// use quill::tree::mappings::Mappings;
/// let input = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	D	E
/// c	A	B
/// 	f	I	bIsAfterA	e
/// 	f	I	bIsAfterAa	d
/// 	f	J	bIsAfterA	e
/// 	f	J	bIsAfterAa	d
/// 	m	()V	methodB	methodBSecondName
/// 	f	I	aIsBeforeB	c
/// 	m	()V	methodA	methodASecondName
/// 	m	(I)V	methodXb	a
/// 	m	(I)V	methodXa	b
/// 	m	(I)V	methodYa	a
/// 	m	(J)V	methodXb	a
/// 	m	(J)V	methodXa	b
/// ";
///
/// let reader = &mut input.as_bytes();
/// let mappings: Mappings<2> = quill::tiny_v2::read(reader).unwrap();
///
/// let mut buf: Vec<u8> = Vec::new();
/// quill::tiny_v2::write(&mappings, &mut buf).unwrap();
/// let written = String::from_utf8(buf).unwrap();
///
/// let output = "\
/// tiny	2	0	namespaceA	namespaceB
/// c	A	B
/// 	f	I	aIsBeforeB	c
/// 	f	I	bIsAfterA	e
/// 	f	I	bIsAfterAa	d
/// 	f	J	bIsAfterA	e
/// 	f	J	bIsAfterAa	d
/// 	m	()V	methodA	methodASecondName
/// 	m	()V	methodB	methodBSecondName
/// 	m	(I)V	methodXa	b
/// 	m	(I)V	methodXb	a
/// 	m	(I)V	methodYa	a
/// 	m	(J)V	methodXa	b
/// 	m	(J)V	methodXb	a
/// c	D	E
/// ";
///
/// assert_eq!(written, output);
/// ```
///
/// Note that there are also the helper methods [`write_vec`] for writing into a `Vec<u8>` directly,
/// and the helper method [`write_string`] that also tries to convert that `Vec<u8>` into a `String`.
pub fn write<const N: usize>(mappings: &Mappings<N>, w: &mut impl Write) -> Result<()> {
	// the buffering makes it much faster
	let mut w = BufWriter::new(w);
	let w = &mut w;

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
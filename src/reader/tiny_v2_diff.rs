use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use crate::reader::diff::{Action, ClassDiff, FieldDiff, MappingInfo, MethodDiff, ParameterDiff, TinyV2Diff};
use crate::reader::tiny_v2_line::{Line, WithMoreIdentIter};
use crate::tree::{Class, Field, Method, Parameter};

pub(crate) fn read_file(path: impl AsRef<Path> + Debug) -> Result<TinyV2Diff> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read mappings file {path:?}"))
}

pub(crate) fn read(reader: impl Read) -> Result<TinyV2Diff> {
	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| -> Result<Line> {
			Line::new(line_number + 1, line?)
		})
		.peekable();

	let mut header = lines.next().context("No header")??;

	if header.first_field != "tiny" || header.next()? != "2" || header.end()? != "0" {
		bail!("Header version isn't tiny v2.0");
	}

	let mut mappings = TinyV2Diff::new(MappingInfo {});

	let mut iter = WithMoreIdentIter::new(0, &mut lines);
	while let Some(mut line) = iter.next().transpose()? {
		if line.first_field == "c" {
			let mut class = Class::new(ClassDiff {
				src: line.next()?,
				dst: line.action()?,
				jav: Action::None,
			});

			let mut iter = iter.next_level();
			while let Some(mut line) = iter.next().transpose()? {
				if line.first_field == "f" {
					let mut field = Field::new(FieldDiff {
						desc: line.next()?,
						src: line.next()?,
						dst: line.action()?,
						jav: Action::None,
					});

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "c" {
							field.inner_mut().jav = line.action()?;
						}
					}

					class.add_field(field);
				} else if line.first_field == "m" {
					let mut method = Method::new(MethodDiff {
						desc: line.next()?,
						src: line.next()?,
						dst: line.action()?,
						jav: Action::None,
					});

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "p" {
							let mut parameter = Parameter::new(ParameterDiff {
								index: line.next()?.parse()?,
								src: line.next()?,
								dst: line.action()?,
								jav: Action::None,
							});

							let mut iter = iter.next_level();
							while let Some(mut line) = iter.next().transpose()? {
								if line.first_field == "c" {
									parameter.inner_mut().jav = line.action()?;
								}
							}

							method.add_parameter(parameter);
						} else if line.first_field == "c" {
							method.inner_mut().jav = line.action()?;
						}
					}

					class.add_method(method);
				} else if line.first_field == "c" {
					class.inner_mut().jav = line.action()?;
				}
			}

			mappings.add_class(class);
		}
	}

	if let Some(line) = lines.next() {
		bail!("Expected end of input, got: {line:?}");
	}

	Ok(mappings)
}
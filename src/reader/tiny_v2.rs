use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use crate::reader::tiny_v2_line::{Line, WithMoreIdentIter};
use crate::reader::tree::{ClassMapping, FieldMapping, MappingInfo, MethodMapping, ParameterMapping, TinyV2Mappings};
use crate::tree::{Class, Field, Method, Parameter};

pub(crate) fn read_file(path: impl AsRef<Path> + Debug) -> Result<TinyV2Mappings> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read mappings file {path:?}"))
}

pub(crate) fn read(reader: impl Read) -> Result<TinyV2Mappings> {
	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| -> Result<Line> {
			Line::new(line_number + 1, line?)
		})
		.peekable();

	let mut header = lines.next().context("No header")??;

	if header.first_field != "tiny" || header.next()? != "2" || header.next()? != "0" {
		bail!("Header version isn't tiny v2.0");
	}

	let mut mappings = TinyV2Mappings::new(MappingInfo {
		src_namespace: header.next()?,
		dst_namespace: header.end()?,
	});

	let mut iter = WithMoreIdentIter::new(0, &mut lines);
	while let Some(mut line) = iter.next().transpose()? {
		if line.first_field == "c" {
			let mut class = Class::new(ClassMapping {
				src: line.next()?,
				dst: line.end()?,
				jav: None,
			});

			let mut iter = iter.next_level();
			while let Some(mut line) = iter.next().transpose()? {
				if line.first_field == "f" {
					let mut field = Field::new(FieldMapping {
						desc: line.next()?,
						src: line.next()?,
						dst: line.end()?,
						jav: None,
					});

					let mut iter = iter.next_level();
					while let Some(line) = iter.next().transpose()? {
						if line.first_field == "c" {
							let comment = line.end()?;
							if field.inner_mut().jav.replace(comment).is_some() {
								bail!("Only one comment per field is allowed");
							}
						}
					}

					class.add_field(field);
				} else if line.first_field == "m" {
					let mut method = Method::new(MethodMapping {
						desc: line.next()?,
						src: line.next()?,
						dst: line.end()?,
						jav: None,
					});

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "p" {
							let mut parameter = Parameter::new(ParameterMapping {
								index: line.next()?.parse()?,
								src: line.next()?,
								dst: line.end()?,
								jav: None,
							});

							let mut iter = iter.next_level();
							while let Some(line) = iter.next().transpose()? {
								if line.first_field == "c" {
									let comment = line.end()?;
									if parameter.inner_mut().jav.replace(comment).is_some() {
										bail!("Only one comment per parameter is allowed");
									}
								}
							}

							method.add_parameter(parameter);
						} else if line.first_field == "c" {
							let comment = line.end()?;
							if method.inner_mut().jav.replace(comment).is_some() {
								bail!("Only one comment per method is allowed");
							}
						}
					}

					class.add_method(method);
				} else if line.first_field == "c" {
					let comment = line.end()?;
					if class.inner_mut().jav.replace(comment).is_some() {
						bail!("Only one comment per class is allowed");
					}
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
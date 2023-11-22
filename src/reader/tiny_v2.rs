use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use crate::reader::tiny_v2_line::{Line, WithMoreIdentIter};
use crate::tree::{ClassNowode, FieldNowode, MethodNowode, ParameterNowode};
use crate::tree::mappings::{ClassMapping, FieldMapping, JavadocMapping, MappingInfo, MethodMapping, ParameterMapping, ClassNowodeMapping, FieldNowodeMapping, Mappings, MethodNowodeMapping, ParameterNowodeMapping};

pub(crate) fn read_file<const N: usize>(path: impl AsRef<Path> + Debug) -> Result<Mappings<N>> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read mappings file {path:?}"))
}

pub(crate) fn read<const N: usize>(reader: impl Read) -> Result<Mappings<N>> {
	if N < 2 {
		bail!("Must read at least two namespaces, {N} is less than that");
	}

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

	let mut mappings = Mappings::new(MappingInfo {
		namespaces: header.list()?,
	});

	let mut iter = WithMoreIdentIter::new(0, &mut lines);
	while let Some(line) = iter.next().transpose()? {
		if line.first_field == "c" {
			let names = line.list()?;

			let mapping = ClassMapping { names };
			let class_key = mapping.get_key();

			let mut class: ClassNowodeMapping<N> = ClassNowode::new(mapping);

			let mut iter = iter.next_level();
			while let Some(mut line) = iter.next().transpose()? {
				if line.first_field == "f" {
					let desc = line.next()?;
					let names = line.list()?;

					let mapping = FieldMapping { desc, names };
					let field_key = mapping.get_key();

					let mut field: FieldNowodeMapping<N> = FieldNowode::new(mapping);

					let mut iter = iter.next_level();
					while let Some(line) = iter.next().transpose()? {
						if line.first_field == "c" {
							let comment = JavadocMapping(line.end()?);
							if field.javadoc.replace(comment).is_some() {
								bail!("Only one comment per field is allowed");
							}
						}
					}

					class.add_field(field_key, field)?;
				} else if line.first_field == "m" {
					let desc = line.next()?;
					let names = line.list()?;

					let mapping = MethodMapping { desc, names };
					let method_key = mapping.get_key();

					let mut method: MethodNowodeMapping<N> = MethodNowode::new(mapping);

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "p" {
							let index = line.next()?.parse()?;
							let names = line.list()?;

							let mapping = ParameterMapping { index, names };
							let parameter_key = mapping.get_key();

							let mut parameter: ParameterNowodeMapping<N> = ParameterNowode::new(mapping);

							let mut iter = iter.next_level();
							while let Some(line) = iter.next().transpose()? {
								if line.first_field == "c" {
									let comment = JavadocMapping(line.end()?);
									if parameter.javadoc.replace(comment).is_some() {
										bail!("Only one comment per parameter is allowed");
									}
								}
							}

							method.add_parameter(parameter_key, parameter)?;
						} else if line.first_field == "c" {
							let comment = JavadocMapping(line.end()?);
							if method.javadoc.replace(comment).is_some() {
								bail!("Only one comment per method is allowed");
							}
						}
					}

					class.add_method(method_key, method)?;
				} else if line.first_field == "c" {
					let comment = JavadocMapping(line.end()?);
					if class.javadoc.replace(comment).is_some() {
						bail!("Only one comment per class is allowed");
					}
				}
			}

			mappings.add_class(class_key, class)?;
		}
	}

	if let Some(line) = lines.next() {
		bail!("Expected end of input, got: {line:?}");
	}

	Ok(mappings)
}
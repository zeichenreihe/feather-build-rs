use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use class_file::tree::class::ClassName;
use class_file::tree::field::FieldName;
use class_file::tree::method::{MethodName, ParameterName};
use crate::reader::tiny_v2_line::{Line, WithMoreIdentIter};
use crate::tree::mappings::{ClassMapping, FieldMapping, JavadocMapping, MappingInfo, MethodMapping, ParameterMapping, ClassNowodeMapping, FieldNowodeMapping, Mappings, MethodNowodeMapping, ParameterNowodeMapping};
use crate::tree::NodeInfo;

pub(crate) fn read_file<const N: usize>(path: impl AsRef<Path> + Debug) -> Result<Mappings<N>> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("failed to read mappings file {path:?}"))
}

pub(crate) fn read<const N: usize>(reader: impl Read) -> Result<Mappings<N>> {
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
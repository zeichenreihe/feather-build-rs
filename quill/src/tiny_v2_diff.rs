use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use duke::tree::class::ClassName;
use duke::tree::field::{FieldDescriptor, FieldName, FieldNameAndDesc};
use duke::tree::method::{MethodDescriptor, MethodName, MethodNameAndDesc, ParameterName};
use crate::lines::tiny_line::TinyLine;
use crate::lines::WithMoreIdentIter;
use crate::tree::mappings::ParameterKey;
use crate::tree::mappings_diff::{Action, ClassNowodeDiff, FieldNowodeDiff, MappingsDiff, MethodNowodeDiff, ParameterNowodeDiff};
use crate::tree::NodeInfo;

pub fn read_file(path: impl AsRef<Path> + Debug) -> Result<MappingsDiff> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("failed to read mappings file {path:?}"))
}

pub(crate) fn read(reader: impl Read) -> Result<MappingsDiff> {
	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| TinyLine::new(line_number + 1, &line?))
		.peekable();

	let mut header = lines.next().context("no header")??;

	if header.first_field != "tiny" || header.next()? != "2" || header.end()? != "0" {
		bail!("header version isn't tiny v2.0");
	}

	let mut mappings = MappingsDiff::new(Action::None);

	let mut iter = WithMoreIdentIter::new(&mut lines);
	while let Some(mut line) = iter.next().transpose()? {
		if line.first_field == "c" {
			let class_key: ClassName = line.next()?.into();

			let action = line.action()?;

			let mut class = ClassNowodeDiff::new(action);

			let mut iter = iter.next_level();
			while let Some(mut line) = iter.next().transpose()? {
				if line.first_field == "f" {
					let desc: FieldDescriptor = line.next()?.into();
					let name: FieldName = line.next()?.into();

					let action = line.action()?;
					let field_key = FieldNameAndDesc { desc, name };

					let mut field = FieldNowodeDiff::new(action);

					let mut iter = iter.next_level();
					while let Some(line) = iter.next().transpose()? {
						if line.first_field == "c" {
							let action = line.action()?;
							if field.javadoc.replace(action).is_some() {
								bail!("only one comment diff per field is allowed")
							}
						}
					}

					class.add_field(field_key, field)?;
				} else if line.first_field == "m" {
					let desc: MethodDescriptor = line.next()?.into();
					let name: MethodName = line.next()?.into();

					let action = line.action()?;
					let method_key = MethodNameAndDesc { desc, name };

					let mut method = MethodNowodeDiff::new(action);

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "p" {
							let index = line.next()?.parse()?;
							let src: ParameterName = line.next()?.into();

							let action = line.action()?;
							let parameter_key = ParameterKey { index, name: src };

							let mut parameter = ParameterNowodeDiff::new(action);

							let mut iter = iter.next_level();
							while let Some(line) = iter.next().transpose()? {
								if line.first_field == "c" {
									let action = line.action()?;
									if parameter.javadoc.replace(action).is_some() {
										bail!("only one comment diff per parameter is allowed")
									}
								}
							}

							method.add_parameter(parameter_key, parameter)?;
						} else if line.first_field == "c" {
							let action = line.action()?;
							if method.javadoc.replace(action).is_some() {
								bail!("only one comment diff per method is allowed")
							}
						}
					}

					class.add_method(method_key, method)?;
				} else if line.first_field == "c" {
					let action = line.action()?;
					if class.javadoc.replace(action).is_some() {
						bail!("only one comment diff per class is allowed")
					}
				}
			}

			mappings.add_class(class_key, class)?;
		}
	}

	if let Some(line) = lines.next() {
		let line = line?;
		bail!("expected end of input, got: {line:?}");
	}

	Ok(mappings)
}
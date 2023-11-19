use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use crate::reader::tiny_v2_line::{Line, WithMoreIdentIter};
use crate::tree::{NodeJavadocMut, ClassNowode, FieldNowode, MethodNowode, ParameterNowode};
use crate::tree::mappings::{ClassKey, ClassMapping, FieldKey, FieldMapping, JavadocMapping, MethodKey, MethodMapping, ParameterKey, ParameterMapping};
use crate::tree::mappings_diff::{Action, TinyV2MappingsDiff};

pub(crate) fn read_file(path: impl AsRef<Path> + Debug) -> Result<TinyV2MappingsDiff> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("Failed to read mappings file {path:?}"))
}

pub(crate) fn read(reader: impl Read) -> Result<TinyV2MappingsDiff> {
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

	let mut mappings = TinyV2MappingsDiff::new(Action::None);

	let mut iter = WithMoreIdentIter::new(0, &mut lines);
	while let Some(mut line) = iter.next().transpose()? {
		if line.first_field == "c" {
			let src = line.next()?;

			let action = line.action(|dst| ClassMapping {
				src: src.clone(),
				dst,
			})?;
			let class_key = ClassKey { src };

			let mut class = ClassNowode::new(action);

			let mut iter = iter.next_level();
			while let Some(mut line) = iter.next().transpose()? {
				if line.first_field == "f" {
					let desc = line.next()?;
					let src = line.next()?;

					let action = line.action(|dst| FieldMapping {
						desc: desc.clone(),
						src: src.clone(),
						dst,
					})?;
					let field_key = FieldKey { desc, src };

					let mut field = FieldNowode::new(action);

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "c" {
							let action = line.action(|jav| JavadocMapping { jav })?;
							if field.node_javadoc_mut().replace(action).is_some() {
								bail!("Only one comment diff per field is allowed")
							}
						}
					}

					class.add_field(field_key, field)?;
				} else if line.first_field == "m" {
					let desc = line.next()?;
					let src = line.next()?;

					let action = line.action(|dst| MethodMapping {
						desc: desc.clone(),
						src: src.clone(),
						dst,
					})?;
					let method_key = MethodKey { desc, src };

					let mut method = MethodNowode::new(action);

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "p" {
							let index = line.next()?.parse()?;
							let src = line.next()?;

							let action = line.action(|dst| ParameterMapping {
								index,
								src: src.clone(),
								dst,
							})?;
							let parameter_key = ParameterKey { index, src };

							let mut parameter = ParameterNowode::new(action);

							let mut iter = iter.next_level();
							while let Some(mut line) = iter.next().transpose()? {
								if line.first_field == "c" {
									let action = line.action(|jav| JavadocMapping { jav })?;
									if parameter.node_javadoc_mut().replace(action).is_some() {
										bail!("Only one comment diff per parameter is allowed")
									}
								}
							}

							method.add_parameter(parameter_key, parameter)?;
						} else if line.first_field == "c" {
							let action = line.action(|jav| JavadocMapping { jav })?;
							if method.node_javadoc_mut().replace(action).is_some() {
								bail!("Only one comment diff per method is allowed")
							}
						}
					}

					class.add_method(method_key, method)?;
				} else if line.first_field == "c" {
					let action = line.action(|jav| JavadocMapping { jav })?;
					if class.node_javadoc_mut().replace(action).is_some() {
						bail!("Only one comment diff per class is allowed")
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
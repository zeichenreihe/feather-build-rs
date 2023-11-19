use std::fmt::Debug;
use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use crate::reader::tiny_v2_line::{Line, WithMoreIdentIter};
use crate::tree::{NodeJavadocMut, ClassNowode, FieldNowode, MethodNowode, ParameterNowode};
use crate::tree::mappings::{ClassKey, ClassMapping, FieldKey, FieldMapping, JavadocMapping, MappingInfo, MethodKey, MethodMapping, ParameterKey, ParameterMapping, TinyV2Class, TinyV2Field, TinyV2Mappings, TinyV2Method, TinyV2Parameter};

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
			let src = line.next()?;
			let dst = line.end()?;

			let class_key = ClassKey { src: src.clone() };
			let mapping = ClassMapping { src, dst };

			let mut class: TinyV2Class = ClassNowode::new(mapping);

			let mut iter = iter.next_level();
			while let Some(mut line) = iter.next().transpose()? {
				if line.first_field == "f" {
					let desc = line.next()?;
					let src = line.next()?;
					let dst = line.end()?;

					let field_key = FieldKey { desc: desc.clone(), src: src.clone() };
					let mapping = FieldMapping { desc, src, dst };

					let mut field: TinyV2Field = FieldNowode::new(mapping);

					let mut iter = iter.next_level();
					while let Some(line) = iter.next().transpose()? {
						if line.first_field == "c" {
							let jav = line.end()?;
							let comment = JavadocMapping { jav };
							if field.node_javadoc_mut().replace(comment).is_some() {
								bail!("Only one comment per field is allowed");
							}
						}
					}

					class.add_field(field_key, field)?;
				} else if line.first_field == "m" {
					let desc = line.next()?;
					let src = line.next()?;
					let dst = line.end()?;

					let method_key = MethodKey { desc: desc.clone(), src: src.clone() };
					let mapping = MethodMapping { desc, src, dst };

					let mut method: TinyV2Method = MethodNowode::new(mapping);

					let mut iter = iter.next_level();
					while let Some(mut line) = iter.next().transpose()? {
						if line.first_field == "p" {
							let index = line.next()?.parse()?;
							let src = line.next()?;
							let dst = line.end()?;

							let parameter_key = ParameterKey { index, src: src.clone() };
							let mapping = ParameterMapping { index, src, dst };

							let mut parameter: TinyV2Parameter = ParameterNowode::new(mapping);

							let mut iter = iter.next_level();
							while let Some(line) = iter.next().transpose()? {
								if line.first_field == "c" {
									let jav = line.end()?;
									let comment = JavadocMapping { jav };
									if parameter.node_javadoc_mut().replace(comment).is_some() {
										bail!("Only one comment per parameter is allowed");
									}
								}
							}

							method.add_parameter(parameter_key, parameter)?;
						} else if line.first_field == "c" {
							let jav = line.end()?;
							let comment = JavadocMapping { jav };
							if method.node_javadoc_mut().replace(comment).is_some() {
								bail!("Only one comment per method is allowed");
							}
						}
					}

					class.add_method(method_key, method)?;
				} else if line.first_field == "c" {
					let jav = line.end()?;
					let comment = JavadocMapping { jav };
					if class.node_javadoc_mut().replace(comment).is_some() {
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
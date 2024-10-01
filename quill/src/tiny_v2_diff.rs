use std::fs::File;
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use java_string::JavaString;
use duke::tree::class::ClassName;
use duke::tree::field::{FieldDescriptor, FieldName, FieldNameAndDesc};
use duke::tree::method::{MethodDescriptor, MethodName, MethodNameAndDesc};
use crate::lines::tiny_line::TinyLine;
use crate::lines::{Line, WithMoreIdentIter};
use crate::tiny_v2::unescape;
use crate::tree::mappings::{JavadocMapping, ParameterKey};
use crate::tree::mappings_diff::{Action, ClassNowodeDiff, FieldNowodeDiff, MappingsDiff, MethodNowodeDiff, ParameterNowodeDiff};
use crate::tree::NodeInfo;

pub fn read_file(path: impl AsRef<Path>) -> Result<MappingsDiff> {
	read(File::open(&path)?)
		.with_context(|| anyhow!("failed to read mappings file {:?} as tiny diff", path.as_ref()))
}

pub(crate) fn read(reader: impl Read) -> Result<MappingsDiff> {
	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| TinyLine::new(line_number + 1, &line?))
		.peekable();

	let mut header = lines.next().context("no header line")??;
	let header_line_number = header.get_line_number();

	if header.first_field != "tiny" || header.next()? != "2" || header.next()? != "0" || header.next().is_ok() {
		bail!("header version isn't tiny v2.0 (or doesn't end right after), in line {header_line_number:?}");
	}

	let mut mappings = MappingsDiff::new(Action::None);

	WithMoreIdentIter::new(&mut lines).on_every_line(|iter, mut line| {
		if line.first_field == "c" {
			let class_key: ClassName = JavaString::from(line.next()?).try_into()?;

			let action = line.action()?;
			let class = ClassNowodeDiff::new(action);
			let class = mappings.add_class(class_key, class)?;

			let mut had_comment = false;
			iter.next_level().on_every_line(|iter, mut line| {
				if line.first_field == "f" {
					let desc: FieldDescriptor = JavaString::from(line.next()?).try_into()?;
					let name: FieldName = JavaString::from(line.next()?).try_into()?;
					let field_key = FieldNameAndDesc { desc, name };

					let action = line.action()?;
					let field = FieldNowodeDiff::new(action);
					let field = class.add_field(field_key, field)?;

					let mut had_comment = false;
					iter.next_level().on_every_line(|_, line| {
						if line.first_field == "c" {
							add_comment(&mut had_comment, &mut field.javadoc, line)
						} else {
							Ok(())
						}
					}).context("reading field sub-sections")
				} else if line.first_field == "m" {
					let desc: MethodDescriptor = JavaString::from(line.next()?).try_into()?;
					let name: MethodName = JavaString::from(line.next()?).try_into()?;
					let method_key = MethodNameAndDesc { desc, name };

					let action = line.action()?;
					let method = MethodNowodeDiff::new(action);
					let method = class.add_method(method_key, method)?;

					let mut had_comment = false;
					iter.next_level().on_every_line(|iter, mut line| {
						if line.first_field == "p" {
							let index = line.next()?.parse()?;
							let parameter_key = ParameterKey { index };

							let src = line.next()?;
							if !src.is_empty() {
								bail!("expected no src field for a parameter in a tiny diff");
							}

							let action = line.action()?;
							let parameter = ParameterNowodeDiff::new(action);
							let parameter = method.add_parameter(parameter_key, parameter)?;

							let mut had_comment = false;
							iter.next_level().on_every_line(|_, line| {
								if line.first_field == "c" {
									add_comment(&mut had_comment, &mut parameter.javadoc, line)
								} else {
									Ok(())
								}
							}).context("reading parameter sub-sections")
						} else if line.first_field == "c" {
							add_comment(&mut had_comment, &mut method.javadoc, line)
						} else {
							Ok(())
						}
					}).context("reading method sub-sections")
				} else if line.first_field == "c" {
					add_comment(&mut had_comment, &mut class.javadoc, line)
				} else {
					Ok(())
				}
			}).context("reading class sub-sections")
		} else {
			Ok(())
		}
	}).context("reading lines")?;

	if let Some(line) = lines.next() {
		let line = line?;
		bail!("expected end of input, got: {line:?}");
	}

	Ok(mappings)
}

fn add_comment(had_comment: &mut bool, javadoc: &mut Action<JavadocMapping>, line: TinyLine) -> Result<()> {
	let action: Action<String> = line.action_string()?;
	let action = match action {
		Action::Add(b) => Action::Add(JavadocMapping(unescape(b))),
		Action::Remove(a) => Action::Remove(JavadocMapping(unescape(a))),
		Action::Edit(a, b) => Action::Edit(JavadocMapping(unescape(a)), JavadocMapping(unescape(b))),
		Action::None => Action::None,
	};
	if *had_comment {
		bail!("only one comment diff is allowed, got {javadoc:?} and {action:?}");
	} else {
		*had_comment = true;
		*javadoc = action;
		Ok(())
	}
}

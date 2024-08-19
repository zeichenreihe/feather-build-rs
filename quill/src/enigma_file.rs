use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use crate::enigma_file::enigma_line::EnigmaLine;
use crate::lines::WithMoreIdentIter;
use crate::tree::mappings::{ClassMapping, ClassNowodeMapping, FieldMapping, FieldNowodeMapping, JavadocMapping, Mappings, MethodMapping, MethodNowodeMapping, ParameterMapping, ParameterNowodeMapping};
use crate::tree::names::Names;
use crate::tree::NodeInfo;

const CLASS: &str = "CLASS";
const FIELD: &str = "FIELD";
const METHOD: &str = "METHOD";
const PARAMETER: &str = "ARG";
const COMMENT: &str = "COMMENT";

pub(crate) fn read_file_into(path: impl AsRef<Path>, mappings: &mut Mappings<2>) -> Result<()> {
	read_into(File::open(&path)?, mappings)
		.with_context(|| anyhow!("failed to read mappings file {:?} as enigma file", path.as_ref()))
}

pub(crate) fn read_into(reader: impl Read, mappings: &mut Mappings<2>) -> Result<()> {
	let mut lines = BufReader::new(reader)
		.lines()
		.enumerate()
		.map(|(line_number, line)| -> Result<EnigmaLine> {
			EnigmaLine::new(line_number + 1, &line?)
		})
		.peekable();

	let mut iter = WithMoreIdentIter::new(&mut lines);
	while let Some(line) = iter.next().transpose()? {

		// TODO: all bails don't contain line_number yet!

		match line.first_field.as_str() {
			CLASS => {
				// We use recursion here to parse classes contained in classes...
				parse_class(mappings, &mut iter, line, None)?;

				fn parse_class(
					mappings: &mut Mappings<2>,
					iter: &mut WithMoreIdentIter<impl Iterator<Item=Result<EnigmaLine>>>,
					line: EnigmaLine,
					parent: Option<&String>
				) -> Result<()> {
					let (src, dst) = match line.fields.as_slice() {
						[src] => (src, None),
						[src, mod_] if is_modifier(mod_) => (src, None),
						[src, dst] => (src, Some(dst)),
						[src, dst, _mod] => (src, Some(dst)),
						slice => bail!("illegal number of arguments ({}) for class mapping, expected 1-3, got {:?}", slice.len(), slice),
					};

					let src = if let Some(parent) = parent {
						format!("{parent}${src}")
					} else {
						src.clone()
					};
					let parent = src.clone();
					let mut class = ClassNowodeMapping::new(ClassMapping {
						names: Names::try_from([Some(src.into()), dst.map(|x| x.clone().into())])?,
					});

					let mut iter = iter.next_level();
					while let Some(line) = iter.next().transpose()? {
						match line.first_field.as_str() {
							CLASS => {
								parse_class(mappings, &mut iter, line, Some(&parent))?
							},
							FIELD => {
								let (src, dst, desc) = match line.fields.as_slice() {
									[src, desc] => (src, None, desc),
									[src, desc, mod_] if is_modifier(mod_) => (src, None, desc),
									[src, dst, desc] => (src, Some(dst), desc),
									[src, dst, desc, _mod] => (src, Some(dst), desc),
									slice => bail!("illegal number of arguments ({}) for field mapping, expected 2-4, got {:?}", slice.len(), slice),
								};
								let mut field = FieldNowodeMapping::new(FieldMapping {
									desc: desc.to_owned().into(),
									names: Names::try_from([Some(src.clone().into()), dst.map(|x| x.clone().into())])?,
								});

								let mut iter = iter.next_level();
								while let Some(line) = iter.next().transpose()? {
									match line.first_field.as_str() {
										COMMENT => insert_comment(&mut field.javadoc, line),
										tag => bail!("unknown mapping target {tag:?} for inside field, allowed are: `COMMENT`"), // TODO: on line <line_number>
									}
								}

								class.add_field(field)?;
							},
							METHOD => {
								let (src, dst, desc) = match line.fields.as_slice() {
									[src, desc] => (src, None, desc),
									[src, desc, mod_] if is_modifier(mod_) => (src, None, desc),
									[src, dst, desc] => (src, Some(dst), desc),
									[src, dst, desc, _mod] => (src, Some(dst), desc),
									slice => bail!("illegal number of arguments ({}) for method mapping, expected 2-4, got {:?}", slice.len(), slice),
								};
								let mut method = MethodNowodeMapping::new(MethodMapping {
									desc: desc.to_owned().into(),
									names: Names::try_from([Some(src.clone().into()), dst.map(|x| x.clone().into())])?,
								});

								let mut iter = iter.next_level();
								while let Some(line) = iter.next().transpose()? {
									match line.first_field.as_str() {
										PARAMETER => {
											let (raw_index, dst) = match line.fields.as_slice() {
												[raw_index, dst] => (raw_index, dst),
												slice => bail!("illegal number of arguments ({}) for parameter mapping, expected 2, got {:?}", slice.len(), slice),
											};

											let index: usize = raw_index.parse()
												.with_context(|| anyhow!("illegal parameter index {raw_index:?}, index cannot be negative"))?;

											let mut parameter = ParameterNowodeMapping::new(ParameterMapping {
												index,
												names: [None, Some(dst.clone().into())].try_into()?,
											});

											let mut iter = iter.next_level();
											while let Some(line) = iter.next().transpose()? {
												match line.first_field.as_str() {
													COMMENT => insert_comment(&mut parameter.javadoc, line),
													tag => bail!("unknown mapping target {tag:?} for inside parameter, allowed are: `COMMENT`"), // TODO: on line <line_number>
												}
											}

											method.add_parameter(parameter)?;
										},

										COMMENT => insert_comment(&mut method.javadoc, line),

										tag => bail!("unknown mapping target {tag:?} for inside method, allowed are: `ARG`, `COMMENT`"), // TODO: on line <line_number>
									}
								}

								class.add_method(method)?;
							},
							COMMENT => insert_comment(&mut class.javadoc, line),
							tag => bail!("unknown mapping target {tag:?} for inside class, allowed are: `CLASS`, `FIELD`, `METHOD`, `COMMENT`"), // TODO: on line <line_number>
						}
					}

					mappings.add_class(class)
				}
			},
			tag => bail!("unknown mapping target {tag:?} for inside root, allowed are: `CLASS`"), // TODO: on line <line_number>
		}
	}

	Ok(())
}

fn is_modifier(s: &str) -> bool {
	const MODIFIER: &str = "ACC:";
	s.starts_with(MODIFIER)
}

fn insert_comment(javadoc: &mut Option<JavadocMapping>, mut line: EnigmaLine) {
	let string = line.fields.join(" ");

	if let Some(javadoc) = javadoc {
		javadoc.0.push_str("\\n");
		javadoc.0.push_str(&string);
	} else {
		*javadoc = Some(JavadocMapping(string));
	}
}


mod enigma_line {
	use anyhow::{anyhow, Context, Result};
	use crate::lines::Line;

	#[derive(Debug)]
	pub(crate) struct EnigmaLine {
		line_number: usize,
		pub(crate) idents: usize,
		pub(crate) first_field: String,
		pub(crate) fields: Vec<String>,
	}

	impl EnigmaLine {
		pub(crate) fn new(line_number: usize, line: &str) -> Result<EnigmaLine> {
			let idents = line.chars().take_while(|x| *x == '\t').count();
			// TODO: there was some other code (related to inner classes?) that did this better!
			//  we may not use the count to index strings!, we must use the char_indices!
			let line = &line[idents..];

			// if the line is a `COMMENT` then it may contain `#`, otherwise everything after `#` is a comment
			let line = if line.starts_with(crate::enigma_file::COMMENT) {
				line
			} else if let Some((non_comment, _)) = line.split_once('#') {
				non_comment
			} else {
				line
			};

			const JAVA_WHITESPACE: [char; 6] = [' ', '\t', '\n', '\x0b', '\x0c', '\x0d'];
			let mut fields = line.split(JAVA_WHITESPACE).map(|x| x.to_owned());

			let first_field = fields.next()
				.with_context(|| anyhow!("no first field in line {line_number}"))?;

			Ok(EnigmaLine {
				line_number,
				idents,
				first_field,
				fields: fields.collect(),
			})
		}
	}

	impl Line for EnigmaLine {
		fn get_idents(&self) -> usize {
			self.idents
		}
		fn get_line_number(&self) -> usize {
			self.line_number
		}
	}
}

// TODO: tests
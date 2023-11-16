
use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::marker::PhantomData;
use crate::tiny::{AddMember, SetJavadoc};

pub(crate) fn try_read<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<String> {
	if let Some(x) = iter.next() {
		Ok(x.to_owned())
	} else {
		bail!("No item given")
	}
}

pub(crate) fn try_read_nonempty<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<String> {
	if let Some(x) = iter.next() {
		if x.is_empty() {
			bail!("Entry may not be empty")
		} else {
			Ok(x.to_owned())
		}
	} else {
		bail!("No item given")
	}
}
pub(crate) fn try_read_optional<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Option<String>> {
	if let Some(x) = iter.next() {
		if x.is_empty() {
			Ok(None)
		} else {
			Ok(Some(x.to_owned()))
		}
	} else {
		Ok(None)
	}
}

pub(crate) trait ReadFromColumnIter: Sized {
	fn read_from_column_iter<'a>(iter: &mut impl Iterator<Item=&'a str>) -> Result<Self>;
}

#[derive(Debug)]
pub(crate) struct Parse<D, C, F, M, P, J> {
	output: D,
	class: Option<C>,
	field: Option<F>,
	method: Option<M>,
	parameter: Option<P>,
	phantom_data: PhantomData<J>,
}

impl<D, C, F, M, P, J> Parse<D, C, F, M, P, J>
	where
		D: ReadFromColumnIter + AddMember<C>,
		C: ReadFromColumnIter + SetJavadoc<J> + AddMember<F> + AddMember<M>,
		F: ReadFromColumnIter + SetJavadoc<J>,
		M: ReadFromColumnIter + SetJavadoc<J> + AddMember<P>,
		P: ReadFromColumnIter + SetJavadoc<J>,
		J: ReadFromColumnIter,
{
	fn set_class(&mut self, class: Option<C>) {
		if let Some(c) = std::mem::replace(&mut self.class, class) {
			self.output.add_member(c);
		}
	}
	fn set_field(&mut self, field: Option<F>) -> Result<()> {
		if let Some(f) = std::mem::replace(&mut self.field, field) {
			self.class.as_mut()
				.context("cannot read field mapping: not in a class?")?
				.add_member(f)
		}
		Ok(())
	}
	fn set_method(&mut self, method: Option<M>) -> Result<()> {
		if let Some(m) = std::mem::replace(&mut self.method, method) {
			self.class.as_mut()
				.context("cannot read method mapping: not in a class?")?
				.add_member(m);
		}
		Ok(())
	}
	fn set_parameter(&mut self, parameter: Option<P>) -> Result<()> {
		if let Some(p) = std::mem::replace(&mut self.parameter, parameter) {
			self.method.as_mut()
				.context("cannot read parameter mapping: not in a method?")?
				.add_member(p);
		}
		Ok(())
	}

	fn parse_line(&mut self, line: String) -> Result<()> {
		let mut iter = line.split('\t')
			.peekable();

		let idents = {
			let mut x = 0usize;
			while iter.next_if(|x| x.is_empty()).is_some() {
				x += 1;
			}
			x
		};

		match (idents, iter.next()) {
			(1, Some("c")) => { // class comment
				if let Some(ref mut class) = self.class {
					class.set_javadoc(J::read_from_column_iter(&mut iter)?);
				} else {
					bail!("cannot read class javadocs: not in a class?");
				}
			},
			(2, Some("c")) => { // field/method comment
				if let Some(ref mut field) = self.field {
					field.set_javadoc(J::read_from_column_iter(&mut iter)?);
				} else if let Some(ref mut method) = self.method {
					method.set_javadoc(J::read_from_column_iter(&mut iter)?);
				} else {
					bail!("cannot read field/method javadocs: not in field or method?");
				}
			},
			(3, Some("c")) => { // parameter comment
				if let Some(ref mut parameter) = self.parameter {
					parameter.set_javadoc(J::read_from_column_iter(&mut iter)?);
				} else {
					bail!("cannot read parameter javadocs: not in a parameter?");
				}
			},
			(0, Some("c")) => { // class
				self.set_parameter(None)?;
				self.set_method(None)?;
				self.set_field(None)?;
				self.set_class(Some(C::read_from_column_iter(&mut iter)?));
			},
			(1, Some("f")) => {
				self.set_parameter(None)?;
				self.set_method(None)?;
				self.set_field(Some(F::read_from_column_iter(&mut iter)?))?;
			},
			(1, Some("m")) => {
				self.set_parameter(None)?;
				self.set_method(Some(M::read_from_column_iter(&mut iter)?))?;
				self.set_field(None)?;
			},
			(2, Some("p")) => {
				self.set_parameter(Some(P::read_from_column_iter(&mut iter)?))?;
				self.set_field(None)?;
			},
			s => bail!("unknown mapping target {s:?}: {:?}", iter.collect::<Vec<_>>()),
		}
		if iter.next().is_none() {
			Ok(())
		} else {
			bail!("line doesn't end")
		}
	}

	pub(crate) fn parse(reader: impl Read) -> Result<D> {
		let mut lines = BufReader::new(reader)
			.lines()
			.enumerate();

		let header = lines.next()
			.ok_or_else(|| anyhow!("No Header"))?.1?;

		let mut header_fields = header.split("\t");

		if Some("tiny") != header_fields.next() {
			bail!("Not a tiny file");
		}
		if Some("2") != header_fields.next() {
			bail!("Tiny file of major version other than 2");
		}
		if Some("0") != header_fields.next() {
			bail!("Tiny file of minor version other than 0");
		}

		let mut parser: Parse<D, C, F, M, P, J> = Parse {
			output: D::read_from_column_iter(&mut header_fields)?,
			class: None,
			field: None,
			method: None,
			parameter: None,
			phantom_data: PhantomData,
		};

		for (line_number, line) in lines {
			parser.parse_line(line?)
				.with_context(|| anyhow!("In line {}", line_number + 1))?
		}

		parser.set_parameter(None)?;
		parser.set_method(None)?;
		parser.set_field(None)?;

		parser.set_class(None);

		Ok(parser.output)
	}
}


use std::iter::Peekable;
use anyhow::{anyhow, bail, Context, Result};
use crate::reader::diff::Action;

#[derive(Debug)]
pub(super) struct Line {
	line_number: usize,
	idents: usize,
	pub(super) first_field: String,
	fields: std::vec::IntoIter<String>,
}

impl Line {
	pub(super) fn new(line_number: usize, line: String) -> Result<Line> {
		let n = line.chars().take_while(|x| *x == '\t').count();
		let line = &line[n..];

		let mut fields = line.split('\t').map(|x| x.to_owned());

		let first_field = fields.next()
			.with_context(|| anyhow!("No first field in line {line_number}"))?;

		let vec: Vec<String> = fields.collect();

		Ok(Line {
			line_number,
			idents: n,
			first_field,
			fields: vec.into_iter(),
		})
	}

	pub(super) fn next(&mut self) -> Result<String> {
		self.fields.next()
			.with_context(|| anyhow!("Expected another field in line: {self:?}"))
	}

	pub(super) fn end(mut self) -> Result<String> {
		let next = self.next()?;

		if self.fields.as_slice().len() != 0 {
			bail!("Line contained more fields than expected: {self:?}");
		}

		Ok(next)
	}

	pub(super) fn action(mut self) -> Result<Action<String>> {
		let a = self.fields.next().unwrap_or(String::new());
		let b = self.fields.next().unwrap_or(String::new());

		if self.fields.as_slice().len() != 0 {
			bail!("Line contained more fields than expected: {self:?}");
		}

		Ok(Action::new(a, b))
	}
}

pub(super) struct WithMoreIdentIter<'a, I: Iterator<Item=Result<Line>>> {
	depth: usize,
	iter: &'a mut Peekable<I>,
}

impl<'a, I: Iterator<Item=Result<Line>>> WithMoreIdentIter<'a, I> {
	pub(super) fn new(depth: usize, iter: &'a mut Peekable<I>) -> WithMoreIdentIter<'a, I> {
		WithMoreIdentIter { depth, iter }
	}

	pub(super) fn next_level(&mut self) -> WithMoreIdentIter<'_, I> {
		WithMoreIdentIter {
			depth: self.depth + 1,
			iter: self.iter,
		}
	}
}

impl<I: Iterator<Item=Result<Line>>> Iterator for WithMoreIdentIter<'_, I> {
	type Item = Result<Line>;

	fn next(&mut self) -> Option<Self::Item> {
		let line = self.iter.peek()?.as_ref().ok()?;

		if line.idents < self.depth {
			// cancel an inner loop
			None
		} else if line.idents == self.depth {
			// actually give back the value
			self.iter.next()
		} else {
			Some(Err(anyhow!("Expected an indentation of {} for line: {:#?}", self.depth, line)))
		}
	}
}
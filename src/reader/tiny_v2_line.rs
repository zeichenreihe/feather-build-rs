use std::cmp::Ordering;
use std::iter::Peekable;
use anyhow::{anyhow, bail, Context, Result};
use crate::tree::mappings_diff::Action;

#[derive(Debug)]
pub(super) struct Line {
	line_number: usize,
	idents: usize,
	pub(super) first_field: String,
	fields: std::vec::IntoIter<String>,
}

impl Line {
	pub(super) fn new(line_number: usize, line: String) -> Result<Line> {
		let idents = line.chars().take_while(|x| *x == '\t').count();
		let line = &line[idents..];

		let mut fields = line.split('\t').map(|x| x.to_owned());

		let first_field = fields.next()
			.with_context(|| anyhow!("no first field in line {line_number}"))?;

		let vec: Vec<String> = fields.collect();

		Ok(Line {
			line_number,
			idents,
			first_field,
			fields: vec.into_iter(),
		})
	}

	pub(super) fn next(&mut self) -> Result<String> {
		self.fields.next()
			.with_context(|| anyhow!("expected another field in line {}: {self:?}", self.line_number))
			.map(|x| x.to_owned())
	}

	pub(super) fn end(mut self) -> Result<String> {
		let next = self.next()?;

		if !self.fields.as_slice().is_empty() {
			bail!("line {} contained more fields than expected: {self:?}", self.line_number);
		}

		Ok(next)
	}

	pub(super) fn list<const N: usize>(self) -> Result<[String; N]> {
		let vec: Vec<_> = self.fields.collect();

		if vec.len() != N {
			bail!("line {} contained more or less fields ({}) than the expected {N}: {:?}", self.line_number, vec.len(), vec);
		}

		Ok(vec.try_into().unwrap()) // can't panic, we checked the size
	}

	pub(super) fn action<T>(mut self) -> Result<Action<T>>
		where
			T: From<String>,
	{
		let a = self.fields.next().filter(|x| !x.is_empty());
		let b = self.fields.next().filter(|x| !x.is_empty());

		if !self.fields.as_slice().is_empty() {
			bail!("line {} contained more fields than expected: {:?}", self.line_number, self);
		}

		Ok(match (a, b) {
			(None, None) => Action::None,
			(None, Some(b)) => Action::Add(b.into()),
			(Some(a), None) => Action::Remove(a.into()),
			(Some(a), Some(b)) if a == b => Action::None,
			(Some(a), Some(b)) => Action::Edit(a.into(), b.into()),
		})
	}
}

pub(super) struct WithMoreIdentIter<'a, I: Iterator<Item=Result<Line>>> {
	depth: usize,
	iter: &'a mut Peekable<I>,
}

impl<'a, I: Iterator<Item=Result<Line>>> WithMoreIdentIter<'a, I> {
	pub(super) fn new(iter: &'a mut Peekable<I>) -> WithMoreIdentIter<'a, I> {
		WithMoreIdentIter { depth: 0, iter }
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
		match self.iter.peek()? {
			Ok(line) => {
				match line.idents.cmp(&self.depth) {
					Ordering::Less => None, // cancel an inner loop
					Ordering::Equal => self.iter.next(), // actually give back the value
					Ordering::Greater => Some(Err(anyhow!("expected an indentation of {} for line {}: {:#?}", self.depth, line.line_number, line))),
				}
			},
			Err(_) => self.iter.next(),
		}
	}
}
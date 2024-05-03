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
			.with_context(|| anyhow!("Expected another field in line {}: {self:?}", self.line_number))
	}

	pub(super) fn end(mut self) -> Result<String> {
		let next = self.next()?;

		if !self.fields.as_slice().is_empty() {
			bail!("Line {} contained more fields than expected: {self:?}", self.line_number);
		}

		Ok(next)
	}

	pub(super) fn list<const N: usize>(self) -> Result<[String; N]> {
		let vec: Vec<_> = self.fields.collect();

		if vec.len() != N {
			bail!("Line {} contained more or less fields ({}) than the expected {N}: {:?}", self.line_number, vec.len(), vec);
		}

		Ok(vec.try_into().unwrap()) // can't panic, we checked the size
	}

	pub(super) fn action<T, F>(mut self, f: F) -> Result<Action<T>>
	where
		F: Fn(String) -> T,
	{
		let a = self.fields.next();
		let b = self.fields.next();

		if !self.fields.as_slice().is_empty() {
			bail!("Line {} contained more fields than expected: {:?}", self.line_number, self);
		}

		// an empty string means no mapping there!
		let a = a.and_then(|x| if x.is_empty() { None } else { Some(x) });
		let b = b.and_then(|x| if x.is_empty() { None } else { Some(x) });

		Ok(match (a, b) {
			(None, None) => Action::None,
			(None, Some(b)) => Action::Add(f(b)),
			(Some(a), None) => Action::Remove(f(a)),
			(Some(a), Some(b)) if a != b => Action::Edit(f(a), f(b)),
			(Some(_), Some(_)) => Action::None,
		})
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

		match line.idents.cmp(&self.depth) {
			Ordering::Less => None, // cancel an inner loop
			Ordering::Equal => self.iter.next(), // actually give back the value
			Ordering::Greater => Some(Err(anyhow!("Expected an indentation of {} for line {}: {:#?}", self.depth, line.line_number, line))),
		}
	}
}
use std::cmp::Ordering;
use std::fmt::Debug;
use std::iter::Peekable;
use anyhow::{anyhow, Result};

pub(crate) trait Line: Debug {
	fn get_idents(&self) -> usize;
	fn get_line_number(&self) -> usize;
}

pub(super) struct WithMoreIdentIter<'a, I: Iterator> {
	depth: usize,
	iter: &'a mut Peekable<I>,
}

impl<'a, I, L> WithMoreIdentIter<'a, I>
where
	I: Iterator<Item=Result<L>>,
	L: Line,
{
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

impl<I, L> Iterator for WithMoreIdentIter<'_, I>
where
	I: Iterator<Item=Result<L>>,
	L: Line,
{
	type Item = Result<L>;

	fn next(&mut self) -> Option<Self::Item> {
		match self.iter.peek()? {
			Ok(line) => {
				match line.get_idents().cmp(&self.depth) {
					Ordering::Less => None, // cancel an inner loop
					Ordering::Equal => self.iter.next(), // actually give back the value
					Ordering::Greater => Some(Err(anyhow!("expected an indentation of {} for line {}: {:#?}", self.depth, line.get_line_number(), line))),
				}
			},
			Err(_) => self.iter.next(),
		}
	}
}


pub(crate) mod tiny_line {
	use anyhow::{anyhow, bail, Context, Result};
	use crate::lines::Line;
	use crate::tree::mappings_diff::Action;

	#[derive(Debug)]
	pub(crate) struct TinyLine {
		line_number: usize,
		idents: usize,
		pub(crate) first_field: String,
		fields: std::vec::IntoIter<String>,
	}

	impl TinyLine {
		pub(crate) fn new(line_number: usize, line: &str) -> Result<TinyLine> {
			let idents = line.chars().take_while(|x| *x == '\t').count();
			// TODO: there was some other code (related to inner classes?) that did this better!
			//  we may not use the count to index strings!, we must use the char_indicies!
			let line = &line[idents..];

			let mut fields = line.split('\t').map(|x| x.to_owned());

			let first_field = fields.next()
				.with_context(|| anyhow!("no first field in line {line_number}"))?;

			let vec: Vec<String> = fields.collect();

			Ok(TinyLine {
				line_number,
				idents,
				first_field,
				fields: vec.into_iter(),
			})
		}

		pub(crate) fn next(&mut self) -> Result<String> {
			self.fields.next()
				.with_context(|| anyhow!("expected another field in line {}: {self:?}", self.line_number))
		}

		pub(crate) fn end(mut self) -> Result<String> {
			let next = self.next()?;

			if !self.fields.as_slice().is_empty() {
				bail!("line {} contained more fields than expected: {self:?}", self.line_number);
			}

			Ok(next)
		}

		pub(crate) fn list<const N: usize>(self) -> Result<[String; N]> {
			let vec: Vec<_> = self.fields.collect();

			<[String; N]>::try_from(vec)
				.map_err(|vec| anyhow!("line {} contained more or less fields ({}) than the expected {N}: {:?}", self.line_number, vec.len(), vec))
		}

		pub(crate) fn action<T>(mut self) -> Result<Action<T>>
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

	impl Line for TinyLine {
		fn get_idents(&self) -> usize {
			self.idents
		}
		fn get_line_number(&self) -> usize {
			self.line_number
		}
	}
}

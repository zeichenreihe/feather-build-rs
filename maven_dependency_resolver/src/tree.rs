use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter};

pub mod helper {
	use crate::tree::Tree;

	/// Helper function for nicely creating trees.
	///
	/// Recommended use: import this function together with [`l`] and use it like:
	/// ```
	/// use maven_dependency_resolver::tree::helper::{l, t};
	///
	/// let tree = t("foo", [
	///     t("bar", [
	///         t("another element", [ l("yet another one"), ]),
	///     ]),
	///     l("baz"),
	/// ]);
	///
	/// assert_eq!(tree.children.len(), 2);
	/// ```
	///
	/// The name stands for "tree".
	pub fn t<T>(n: T, c: impl Into<Vec<Tree<T>>>) -> Tree<T> {
		Tree { data: n, children: c.into(), }
	}

	/// Helper function for nicely creating a leaf node.
	///
	/// Recommended use: import this function together with [`t`] and use it like shown there.
	///
	/// This is equivalent to calling [`t`] with the first argument and `[]` as the second argument.
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use maven_dependency_resolver::tree::helper::{l, t};
	/// assert_eq!(l(10), t(10, []));
	/// ```
	///
	/// The name stands for "leaf".
	pub fn l<T>(n: T) -> Tree<T> {
		t(n, [])
	}
}

#[derive(Clone, PartialEq)]
pub struct Tree<T> {
	pub data: T,
	pub children: Vec<Tree<T>>,
}

pub struct Forest;

impl<T> Tree<T> {
	pub fn new(data: T) -> Tree<T> {
		Tree { data, children: vec![] }
	}

	pub fn format_with<'tree, F, U>(&'tree self, f: F) -> FormattedTree<'tree, T, F>
	where
		F: Fn(&'tree T) -> U,
		U: 'tree,
	{
		FormattedTree { tree: self, f, palette: Palette::GRAPH }
	}
}

impl Forest {
	/// Retains all nodes specified by the predicate.
	///
	/// This takes in a forest, aka a collection of trees.
	///
	/// In other words, remove all nodes `e` (and children thereof) for which `f(&e)` returns `false`.
	///
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use maven_dependency_resolver::tree::helper::{l, t};
	/// use maven_dependency_resolver::tree::Forest;
	///
	/// let mut a = vec![
	///     l(0),
	///     t(1, [
	///         t(2, [
	///             t(3, [ l(4), l(5), l(6), ]),
	///             t(7, [ l(8), l(9), l(10), ]),
	///         ]),
	///         t(11, [ l(12), ]),
	///     ]),
	/// ];
	///
	/// Forest::breadth_first_retain(&mut a, |i| i % 3 != 0);
	///
	/// let b = [
	///     t(1, [
	///         t(2, [
	///             t(7, [ l(8), l(10), ]),
	///         ]),
	///         l(11),
	///     ]),
	/// ];
	///
	/// assert_eq!(a, b)
	///
	/// ```
	#[allow(clippy::needless_lifetimes)]
	pub fn breadth_first_retain<T>(forest: &mut Vec<Tree<T>>, mut f: impl FnMut(&T) -> bool) {
		forest.retain(|x| f(&x.data));
		let mut queue: VecDeque<_> = forest.iter_mut().collect();
		while let Some(t) = queue.pop_front() {
			t.children.retain(|x| f(&x.data));
			queue.extend(t.children.iter_mut())
		}
	}
}

impl<T: Debug> Debug for Tree<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.format_with(|x| x).fmt_both(f, Debug::fmt)
	}
}

impl<T: Display> Display for Tree<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.format_with(|x| x).fmt_both(f, Display::fmt)
	}
}

#[derive(Debug)]
pub struct FormattedTree<'tree, T, F> {
	tree: &'tree Tree<T>,
	f: F,
	palette: Palette,
}

impl<T, F> FormattedTree<'_, T, F> {
	pub fn with_palette(self, palette: Palette) -> Self {
		Self { palette, ..self }
	}
}

impl<'tree, T, F, U> FormattedTree<'tree, T, F>
where
	F: Fn(&'tree T) -> U,
	U: 'tree,
{
	// TODO: multiline testing! what if fmt(...) is multiline?
	fn fmt_both(&self, f: &mut Formatter, fmt: impl Fn(&U, &mut Formatter) -> std::fmt::Result) -> std::fmt::Result {
		fmt(&(self.f)(&self.tree.data), f)?;
		writeln!(f)?;

		let mut queue = VecDeque::new();
		for (i, child) in self.tree.children.iter().rev().enumerate() {
			let last = i == 0;
			queue.push_front((child, last, vec![]));
		}

		while let Some((node, last, is_last_path)) = queue.pop_front() {
			for (i, child) in node.children.iter().rev().enumerate() {
				let mut is_last_path = is_last_path.clone();
				is_last_path.push(last);

				queue.push_front((child, i == 0, is_last_path));
			}

			for last in is_last_path {
				f.pad(if last { self.palette.last_skip } else { self.palette.middle_skip })?;
			}
			f.pad(if last { self.palette.last_item } else { self.palette.middle_item })?;

			fmt(&(self.f)(&node.data), f)?;
			writeln!(f)?;
		}

		Ok(())
	}
}

impl<'tree, T, F, U> Display for FormattedTree<'tree, T, F>
where
	F: Fn(&'tree T) -> U,
	U: Display + 'tree,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.fmt_both(f, Display::fmt)
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Palette {
	pub middle_item: &'static str,
	pub middle_skip: &'static str,
	pub last_item: &'static str,
	pub last_skip: &'static str,
}

impl Palette {
	pub const ASCII: Palette = Palette {
		middle_item: "+- ",
		middle_skip: "|  ",
		last_item:  "\\- ",
		last_skip:   "   ",
	};
	pub const GRAPH: Palette = Palette {
		middle_item: "├── ",
		middle_skip: "│   ",
		last_item:   "└── ",
		last_skip:   "    ",
	};
}

pub mod breath_first_traversal {
	use std::collections::VecDeque;
	use crate::tree::{Forest, Tree};

	impl<T> Tree<T> {
		/// Traverse a tree breath first.
		///
		/// ```
		/// # use pretty_assertions::assert_eq;
		/// use maven_dependency_resolver::tree::helper::{l, t};
		///
		/// let tree = t(0, [
		///     t(1, [ l(3), l(4), ]),
		///     t(2, [ l(5), l(6), ]),
		/// ]);
		///
		/// assert_eq!(tree.into_breadth_first().collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5, 6]);
		/// ```
		pub fn into_breadth_first(self) -> BreathFirstIntoIter<T> {
			BreathFirstIntoIter { queue: vec![self].into() }
		}

		/// Traverse a tree breath first by reference.
		///
		/// ```
		/// # use pretty_assertions::assert_eq;
		/// use maven_dependency_resolver::tree::helper::{l, t};
		///
		/// let tree = t(0, [
		///     t(1, [ l(3), l(4), ]),
		///     t(2, [ l(5), l(6), ]),
		/// ]);
		///
		/// assert_eq!(tree.breadth_first().cloned().collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5, 6]);
		/// ```
		#[allow(clippy::needless_lifetimes)]
		pub fn breadth_first<'tree>(&'tree self) -> BreathFirstIter<'tree, T> {
			BreathFirstIter { queue: vec![self].into() }
		}
	}

	impl Forest {
		/// Traverse a forest breath first.
		///
		/// ```
		/// # use pretty_assertions::assert_eq;
		/// use maven_dependency_resolver::tree::Forest;
		/// use maven_dependency_resolver::tree::helper::{l, t};
		///
		/// let forest = vec![
		///     t(0, [
		///         t(2, [ l(5), l(6), ]),
		///         l(3),
		///     ]),
		///     t(1, [
		///         l(4),
		///     ]),
		/// ];
		///
		/// assert_eq!(Forest::into_breadth_first(forest).collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5, 6]);
		/// ```
		#[allow(clippy::wrong_self_convention)]
		pub fn into_breadth_first<T>(forest: Vec<Tree<T>>) -> BreathFirstIntoIter<T> {
			BreathFirstIntoIter { queue: forest.into() }
		}

		/// Traverse a forest breath first by reference.
		///
		/// ```
		/// # use pretty_assertions::assert_eq;
		/// use maven_dependency_resolver::tree::Forest;
		/// use maven_dependency_resolver::tree::helper::{l, t};
		///
		/// let forest = [
		///     t(0, [
		///         t(2, [ l(5), l(6), ]),
		///         l(3),
		///     ]),
		///     t(1, [
		///         l(4),
		///     ]),
		/// ];
		///
		/// assert_eq!(Forest::breadth_first(&forest).cloned().collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5, 6]);
		/// ```
		#[allow(clippy::needless_lifetimes)]
		pub fn breadth_first<'forest, T>(forest: &'forest [Tree<T>]) -> BreathFirstIter<'forest, T> {
			BreathFirstIter { queue: forest.iter().collect() }
		}
	}

	/// See [Forest::into_breadth_first] and [Tree::into_breadth_first].
	#[derive(Debug)]
	pub struct BreathFirstIntoIter<T> {
		queue: VecDeque<Tree<T>>,
	}

	impl<T> Iterator for BreathFirstIntoIter<T> {
		type Item = T;

		fn next(&mut self) -> Option<Self::Item> {
			self.queue.pop_front().map(|t| {
				self.queue.extend(t.children);
				t.data
			})
		}
	}

	/// See [Forest::breath_first] and [Tree::breath_first].
	#[derive(Debug)]
	pub struct BreathFirstIter<'tree, T> {
		queue: VecDeque<&'tree Tree<T>>,
	}

	impl<'tree, T> Iterator for BreathFirstIter<'tree, T> {
		type Item = &'tree T;

		fn next(&mut self) -> Option<Self::Item> {
			self.queue.pop_front().map(|t| {
				self.queue.extend(&t.children);
				&t.data
			})
		}
	}
}

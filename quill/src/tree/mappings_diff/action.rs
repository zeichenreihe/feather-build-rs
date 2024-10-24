

/// Represents an diff action.
///
/// A diff action is isomorphic to `(Option<T>, Option<T>)`. [`Action::from_tuple`] and [`Action::to_tuple`] are the isomorphisms.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum Action<T> {
	#[default]
	None,
	Add(T),
	Remove(T),
	Edit(T, T),
}

impl<T: PartialEq> Action<T> {
	pub fn is_diff(&self) -> bool {
		match self {
			Action::None => false,
			Action::Add(_) => true,
			Action::Remove(_) => true,
			Action::Edit(a, b) => a != b,
		}
	}
}

impl<T> Action<T> {
	/// Inverse of [`Action::to_tuple`].
	pub fn from_tuple(a: Option<T>, b: Option<T>) -> Action<T> {
		match (a, b) {
			(None, None) => Action::None,
			(None, Some(b)) => Action::Add(b),
			(Some(a), None) => Action::Remove(a),
			(Some(a), Some(b)) => Action::Edit(a, b),
		}
	}

	/// Inverse of [`Action::from_tuple`].
	pub fn to_tuple(self) -> (Option<T>, Option<T>) {
		match self {
			Action::None => (None, None),
			Action::Add(b) => (None, Some(b)),
			Action::Remove(a) => (Some(a), None),
			Action::Edit(a, b) => (Some(a), Some(b)),
		}
	}

	pub fn as_ref(&self) -> Action<&T> {
		match self {
			Action::None => Action::None,
			Action::Add(b) => Action::Add(b),
			Action::Remove(a) => Action::Remove(a),
			Action::Edit(a, b) => Action::Edit(a, b),
		}
	}

	/// Flips the direction of the action.
	///
	/// The inverse of this function is itself.
	///
	/// Alternatively it can be constructed from [`Action::to_tuple`] and [`Action::from_tuple`]:
	/// ```
	/// # use pretty_assertions::assert_eq;
	/// use quill::tree::mappings_diff::Action;
	///
	/// let actions: [Action<i32>; 4] = [Action::None, Action::Add(1), Action::Remove(2), Action::Edit(3, 4)];
	/// for action in actions {
	///     let (a, b) = action.clone().to_tuple();
	///     let flipped = Action::from_tuple(b, a);
	///     assert_eq!(action.flip(), flipped)
	/// }
	/// ```
	pub fn flip(self) -> Action<T> {
		match self {
			Action::None => Action::None,
			Action::Add(b) => Action::Remove(b),
			Action::Remove(a) => Action::Add(a),
			Action::Edit(a, b) => Action::Edit(b, a),
		}
	}
}

/// See [`Action::from_tuple`].
impl<T> From<(Option<T>, Option<T>)> for Action<T> {
	fn from(value: (Option<T>, Option<T>)) -> Self {
		Action::from_tuple(value.0, value.1)
	}
}
/// See [`Action::to_tuple`].
impl<T> From<Action<T>> for (Option<T>, Option<T>) {
	fn from(value: Action<T>) -> Self {
		value.to_tuple()
	}
}
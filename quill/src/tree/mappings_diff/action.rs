

/// Represents an diff action.
#[derive(Debug, Clone, Default)]
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
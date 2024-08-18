use anyhow::Result;
use duke::tree::class::ClassName;
use crate::tree::mappings_diff::{Action, MappingsDiff};

impl MappingsDiff {
	// TODO: doc
	/// Removed so called "dummy" mappings.
	///
	/// Whether or not a mapping is considered a dummy mapping only depends on the mapping in the namespace given.
	///
	/// # Removal Rules
	/// - a class mappings is removed if in the given namespace its name starts with `C_` or `net/minecraft/unmapped/C_`, and
	///   there are no members, i.e. fields, methods, javadoc, left.
	/// - a field mapping is removed if in the given namespace its name starts with `f_`, and it doesn't have any javadoc.
	/// - a method mapping is removed if in the given namespace its name starts with `m_`, or its name is equal to either
	///   `<init>` or `<clinit>`, and it doesn't have any members, i.e. javadoc or parameter mappings.
	/// - a parameter mapping is removed if its name starts with `p_` and it doesn't have any javadoc.
	pub fn insert_dummy_and_contract_inner_names(mut self) -> Result<Self> {

		fn javadoc_is_diff<T: PartialEq>(action: &Option<Action<T>>) -> bool {
			action.as_ref().is_some_and(|action| action.is_diff())
		}

		self.classes.retain(|k, v| {
			v.fields.retain(|k, v| {

				let validator_check = match &v.info {
					Action::Add(_) => {
						// new mappings should be ignored, as any un-mapped members should already be present as dummy mappings
						eprintln!("ignoring illegal field change {v:?}");
						false
					},
					Action::Remove(a) => {
						// removing a mapping is changed into a dummy mapping

						let b = k.name.clone();
						v.info = Action::Edit(a.clone(), b);
						true
					},
					Action::Edit(_, _) => true,
					Action::None => true,
				};

				validator_check && (
					v.info.is_diff() ||
						javadoc_is_diff(&v.javadoc)
				)
			});
			v.methods.retain(|k, v| {
				v.parameters.retain(|k, v| {

					let validator_check = match &v.info {
						Action::Add(_) => {
							// new mappings should be ignored, as any un-mapped members should already be present as dummy mappings
							eprintln!("ignoring illegal parameter change {v:?}");
							false
						},
						Action::Remove(a) => {
							// removing a mapping is changed into a dummy mapping

							let b = format!("p_{}", k.index).into();
							v.info = Action::Edit(a.clone(), b);
							true
						},
						Action::Edit(_, _) => true,
						Action::None => true,
					};

					validator_check && (
						v.info.is_diff() ||
							javadoc_is_diff(&v.javadoc)
					)
				});

				let validator_check = match &v.info {
					Action::Add(_) => {
						// new mappings should be ignored, as any un-mapped members should already be present as dummy mappings
						eprintln!("ignoring illegal method change {v:?}");
						false
					},
					Action::Remove(a) => {
						// removing a mapping is changed into a dummy mapping

						let b = k.name.clone();
						v.info = Action::Edit(a.clone(), b);
						true
					},
					Action::Edit(_, _) => true,
					Action::None => true,
				};

				(
					validator_check && (
						v.info.is_diff() ||
							javadoc_is_diff(&v.javadoc)
					)
				)
					|| !v.parameters.is_empty()
			});

			let validator_check = match &v.info {
				Action::Add(_) => {
					// new mappings should be ignored, as any un-mapped members should already be present as dummy mappings
					eprintln!("ignoring illegal class change {v:?}");
					false
				},
				Action::Remove(a) => {
					// removing a mapping is changed into a dummy mapping

					fn get_simplified(name: &ClassName) -> ClassName {
						name.as_ref().rsplit_once('$')
							.map_or_else(|| name.clone(), |(_, inner)| inner.to_owned().into())
					}

					let b = get_simplified(k);
					v.info = Action::Edit(a.clone(), b);
					true
				},
				Action::Edit(_, _) => true,
				Action::None => true,
			};

			(
				validator_check && (
					v.info.is_diff() ||
						javadoc_is_diff(&v.javadoc)
				)
			)
				|| !v.fields.is_empty()
				|| !v.methods.is_empty()
		});

		Ok(self)
	}
}

#[cfg(test)]
mod testing {
	// TODO: test internals?
}
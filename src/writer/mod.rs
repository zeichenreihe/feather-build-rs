
pub(crate) mod tiny_v2 {
	use std::io::Write;
	use anyhow::Result;
	use crate::tree::mappings::Mappings;
	use crate::tree::{Names, NodeData};

	pub(crate) fn write<const N: usize>(mappings: &Mappings<N>, mut writer: impl Write) -> Result<()> {
		writer.write(write_string(mappings)?.as_bytes())?;

		Ok(())
	}

	fn write_names<const N: usize>(writer: &mut String, names: &Names<N>) -> Result<()> {
		use std::fmt::Write;
		for name in names.names() {
			write!(writer, "\t{}", name.unwrap_or(&String::new()));
		}
		writeln!(writer);
		Ok(())
	}

	pub(crate) fn write_string<const N: usize>(mappings: &Mappings<N>) -> Result<String> {
		use std::fmt::Write;

		let mut s = String::new();

		let node = mappings.node_data();
		write!(s, "tiny\t2\t0")?;
		for namespace in &node.namespaces {
			write!(s, "\t{namespace}");
		}
		writeln!(s);

		if let Some(ref comment) = mappings.javadoc {
			writeln!(s, "\tc\t{}", comment.0)?;
		}

		let mut classes: Vec<_> = mappings.classes.values().collect();
		classes.sort_by(|a, b| a.node_data().cmp(b.node_data()));
		for class in classes {
			let node = class.node_data();
			write!(s, "c");
			write_names(&mut s, &node.names)?;

			if let Some(ref comment) = class.javadoc {
				writeln!(s, "\tc\t{}", comment.0)?;
			}

			let mut fields: Vec<_> = class.fields.values().collect();
			fields.sort_by(|a, b| a.node_data().cmp(b.node_data()));
			for field in fields {
				let node = field.node_data();
				write!(s, "\tf\t{}", node.desc);
				write_names(&mut s, &node.names)?;

				if let Some(ref comment) = field.javadoc {
					writeln!(s, "\t\tc\t{}", comment.0)?;
				}
			}

			let mut methods: Vec<_> = class.methods.values().collect();
			methods.sort_by(|a, b| a.node_data().cmp(b.node_data()));
			for method in methods {
				let node = method.node_data();
				write!(s, "\tm\t{}", node.desc)?;
				write_names(&mut s, &node.names)?;

				if let Some(ref comment) = method.javadoc {
					writeln!(s, "\t\tc\t{}", comment.0)?;
				}

				let mut parameters: Vec<_> = method.parameters.values().collect();
				parameters.sort_by(|a, b| a.node_data().cmp(b.node_data()));
				for parameter in parameters {
					let node = parameter.node_data();
					write!(s, "\t\tp\t{}", node.index)?;
					write_names(&mut s, &node.names)?;

					if let Some(ref comment) = parameter.javadoc {
						writeln!(s, "\t\t\tc\t{}", comment.0)?;
					}
				}
			}
		}

		Ok(s)
	}
}
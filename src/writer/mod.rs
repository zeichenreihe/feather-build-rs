
pub(crate) mod tiny_v2 {
	use std::io::Write;
	use anyhow::Result;
	use crate::tree::mappings::Mappings;
	use crate::tree::NodeData;

	pub(crate) fn write(mappings: &Mappings<2>, mut writer: impl Write) -> Result<()> {
		writer.write(write_string(mappings)?.as_bytes())?;

		Ok(())
	}

	pub(crate) fn write_string(mappings: &Mappings<2>) -> Result<String> {
		use std::fmt::Write;

		let mut s = String::new();

		let node = mappings.node_data();
		writeln!(s, "tiny\t2\t0\t{}\t{}", node.namespaces[0], node.namespaces[1])?;

		if let Some(ref comment) = mappings.javadoc {
			writeln!(s, "\tc\t{}", comment.0)?;
		}

		let mut classes: Vec<_> = mappings.classes.values().collect();
		classes.sort_by(|a, b| a.node_data().cmp(b.node_data()));
		for class in classes {
			let node = class.node_data();
			writeln!(s, "c\t{}\t{}", node.names[0], node.names[1])?;

			if let Some(ref comment) = class.javadoc {
				writeln!(s, "\tc\t{}", comment.0)?;
			}

			let mut fields: Vec<_> = class.fields.values().collect();
			fields.sort_by(|a, b| a.node_data().cmp(b.node_data()));
			for field in fields {
				let node = field.node_data();
				writeln!(s, "\tf\t{}\t{}\t{}", node.desc, node.names[0], node.names[1])?;

				if let Some(ref comment) = field.javadoc {
					writeln!(s, "\t\tc\t{}", comment.0)?;
				}
			}

			let mut methods: Vec<_> = class.methods.values().collect();
			methods.sort_by(|a, b| a.node_data().cmp(b.node_data()));
			for method in methods {
				let node = method.node_data();
				writeln!(s, "\tm\t{}\t{}\t{}", node.desc, node.names[0], node.names[1])?;

				if let Some(ref comment) = method.javadoc {
					writeln!(s, "\t\tc\t{}", comment.0)?;
				}

				let mut parameters: Vec<_> = method.parameters.values().collect();
				parameters.sort_by(|a, b| a.node_data().cmp(b.node_data()));
				for parameter in parameters {
					let node = parameter.node_data();
					writeln!(s, "\t\tp\t{}\t{}\t{}", node.index, node.names[0], node.names[1])?;

					if let Some(ref comment) = parameter.javadoc {
						writeln!(s, "\t\t\tc\t{}", comment.0)?;
					}
				}
			}
		}

		Ok(s)
	}
}
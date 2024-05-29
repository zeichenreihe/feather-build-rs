
pub(crate) mod tiny_v2 {
	use std::io::{Cursor, Write};
	use anyhow::{Context, Result};
	use zip::write::FileOptions;
	use zip::ZipWriter;
	use crate::tree::mappings::Mappings;
	use crate::tree::names::{Names, Namespaces};

	pub(crate) fn write_zip_file<const N: usize>(mappings: &Mappings<N>) -> Result<Vec<u8>> {
		let mut zip = ZipWriter::new(Cursor::new(Vec::new()));

		zip.start_file("mappings/mappings.tiny", FileOptions::default())?;

		write(mappings, &mut zip)?;

		Ok(zip.finish()?.into_inner())
	}

	#[cfg(test)]
	pub(crate) fn write_string<const N: usize>(mappings: &Mappings<N>) -> Result<String> {
		let vec = write_vec(mappings)?;
		String::from_utf8(vec).context("failed to convert file to utf8")
	}

	pub(crate) fn write_vec<const N: usize>(mappings: &Mappings<N>) -> Result<Vec<u8>> {
		let mut vec = Vec::new();
		write(mappings, &mut vec)?;
		Ok(vec)
	}

	fn write_namespaces<const N: usize>(w: &mut impl Write, namespaces: &Namespaces<N>) -> Result<()> {
		for namespace in namespaces.names() {
			write!(w, "\t{namespace}")?;
		}
		writeln!(w)?;
		Ok(())
	}

	fn write_names<const N: usize, T>(w: &mut impl Write, names: &Names<N, T>) -> Result<()>
	where
		for<'a> &'a str: From<&'a T>,
	{
		for name in names.names() {
			let name = name.map(|x| x.into());
			write!(w, "\t{}", name.unwrap_or(""))?;
		}
		writeln!(w)?;
		Ok(())
	}

	pub(crate) fn write<const N: usize>(mappings: &Mappings<N>, w: &mut impl Write) -> Result<()> {
		write!(w, "tiny\t2\t0")?;
		write_namespaces(w, &mappings.info.namespaces)?;

		if let Some(ref comment) = mappings.javadoc {
			writeln!(w, "\tc\t{}", comment.0)?;
		}

		let mut classes: Vec<_> = mappings.classes.values().collect();
		classes.sort_by_key(|x| &x.info);
		for class in classes {
			write!(w, "c")?;
			write_names(w, &class.info.names)?;

			if let Some(ref comment) = class.javadoc {
				writeln!(w, "\tc\t{}", comment.0)?;
			}

			let mut fields: Vec<_> = class.fields.values().collect();
			fields.sort_by_key(|x| &x.info);
			for field in fields {
				write!(w, "\tf\t{}", field.info.desc.as_str())?;
				write_names(w, &field.info.names)?;

				if let Some(ref comment) = field.javadoc {
					writeln!(w, "\t\tc\t{}", comment.0)?;
				}
			}

			let mut methods: Vec<_> = class.methods.values().collect();
			methods.sort_by_key(|x| &x.info);
			for method in methods {
				write!(w, "\tm\t{}", method.info.desc.as_str())?;
				write_names(w, &method.info.names)?;

				if let Some(ref comment) = method.javadoc {
					writeln!(w, "\t\tc\t{}", comment.0)?;
				}

				let mut parameters: Vec<_> = method.parameters.values().collect();
				parameters.sort_by_key(|x| &x.info);
				for parameter in parameters {
					write!(w, "\t\tp\t{}", parameter.info.index)?;
					write_names(w, &parameter.info.names)?;

					if let Some(ref comment) = parameter.javadoc {
						writeln!(w, "\t\t\tc\t{}", comment.0)?;
					}
				}
			}
		}

		Ok(())
	}
}
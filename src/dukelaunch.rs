use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Command;
use anyhow::{anyhow, bail, Context, Result};
use log::{error, trace};

/// The "Java File Separator". On Windows `;`, on unix-based ':'.
#[cfg(not(windows))]
const FILE_SEPARATOR: &str = ":";
#[cfg(windows)]
const FILE_SEPARATOR: &str = ";";

#[derive(Debug)]
pub struct JavaRunConfig<'a> {
	pub main_class: OsString,
	pub classpath: Vec<OsString>,
	pub jvm_args: Vec<OsString>,
	pub args: Vec<&'a OsStr>,
}

#[derive(Debug)]
pub(crate) struct JavaLauncher {
	java_command: OsString,
}

impl Default for JavaLauncher {
	fn default() -> Self {
		JavaLauncher { java_command: "java".into() }
	}
}

impl JavaLauncher {
	pub(crate) fn new(java_command: &(impl AsRef<OsStr> + ?Sized)) -> JavaLauncher {
		JavaLauncher { java_command: OsString::from(java_command) }
	}

	pub(crate) fn from_env_var() -> Option<JavaLauncher> {
		const JAVA_HOME: &str = "JAVA_HOME";

		std::env::var_os(JAVA_HOME)
			.map(|java_home| {
				// needs to be a PathBuf because that takes care of slashes at the end
				let mut path = PathBuf::from(java_home);
				path.push("bin/java");
				let java_command = OsString::from(path);

				trace!("located java via env var as {java_command:?}");

				JavaLauncher { java_command }
			})
	}

	/// Returns `Err(_)` if the java doesn't satisfy the given version.
	///
	/// This is done by running `java -version` as a process, and parsing it's output.
	pub(crate) fn check_java_version(&self, min_java_major_version: u16) -> Result<()> {
		let mut command = Command::new(&self.java_command);
		command.arg("-version");

		trace!("running {command:?} to get java version");
		let output = command.output()
			.with_context(|| anyhow!("failed to run {command:?}"))?;

		// `java -version` gives to stderr something like
		//     openjdk version "1.8.0_412"
		//     OpenJDK Runtime Environment (build 1.8.0_412-b08)
		//     OpenJDK 64-Bit Server VM (build 25.412-b08, mixed mode)
		// or something like
		//     openjdk version "17.0.11" 2024-04-16
		//     OpenJDK Runtime Environment (build 17.0.11+9)
		//     OpenJDK 64-Bit Server VM (build 17.0.11+9, mixed mode, sharing)
		let stderr = std::str::from_utf8(&output.stderr)
			.with_context(|| anyhow!("stderr of is not UTF-8: {:?}", &output.stderr))?;

		let version = java_dash_version_output_to_version(stderr)
			.with_context(|| anyhow!("failed to get java version from {output:?}"))?;

		trace!("that's java {version}");

		if version < min_java_major_version {
			bail!("java found as {:?} is of major version {version}, expected at least major version {min_java_major_version}", &self.java_command);
		}

		Ok(())
	}

	pub(crate) fn launch(&self, config: &JavaRunConfig) -> Result<()> {

		let mut command = Command::new(&self.java_command);

		command
			.args(&config.jvm_args)
			.args([OsStr::new("-classpath"), &config.classpath.join(OsStr::new(FILE_SEPARATOR))])
			.arg(&config.main_class)
			.args(&config.args);

		trace!("run: {} {}", command.get_program().to_string_lossy(), command.get_args().map(|x| x.to_string_lossy()).collect::<Vec<_>>().join(" "));

		let x = command.spawn()?.wait()?;

		if !x.success() {
			error!("java exited with error state {x:?}");
		} else {
			trace!("java exited with {x:?}");
		}

		Ok(())
	}
}

/// Parse the output of `java -version` into the major java version
///
/// `java -version` writes to stderr a predictable format, where
/// in the first line, the third field is the version of `java`.
/// This field is enclosed in a pair of `"`.
///
/// This string can be in the old format, used for java 8 and below,
/// which is something like `1.8.0_412` and `1.7.0_52` where the 8 and 7,
/// respectively, is the major version.
///
/// There's also the new format, used for java 9 and above, which is
/// something like `11.0.23`, `17.0.11` and `22`, where the 11, 17 and
/// 22 respectively, is the major version.
///
/// # Example
/// An output to stderr of
/// ```
/// openjdk version "1.8.0_412"
/// OpenJDK Runtime Environment (build 1.8.0_412-b08)
/// OpenJDK 64-Bit Server VM (build 25.412-b08, mixed mode)
/// ```
/// would parse to a version number `8` (see the `1.8` in the first line).
///
/// And an output to stderr of
/// ```
/// openjdk version "17.0.11" 2024-04-16
/// OpenJDK Runtime Environment (build 17.0.11+9)
/// OpenJDK 64-Bit Server VM (build 17.0.11+9, mixed mode, sharing)
/// ```
/// would parse to a version number `17` (see the `17` in the first line).
fn java_dash_version_output_to_version(stderr: &str) -> Result<u16> {
	// Get the first line, so something like
	//     openjdk version "1.8.0_412"
	// or
	//     openjdk version "17.0.11" 2024-04-16
	let line = stderr.lines().next().with_context(|| anyhow!("expected a line on stderr, got {stderr:?}"))?;

	// Something like `"1.8.0_412"` or `"17.0.11"` (notice the quotes)
	let quoted_version = line.split(' ').nth(2).with_context(|| anyhow!("expected third item (space separated) of {line:?} to exist"))?;

	// remove the quotes
	let version_string = quoted_version.strip_prefix('\"')
		.and_then(|x| x.strip_suffix('\"'))
		.with_context(|| anyhow!("expected version string to start with \" and end with \", got {quoted_version:?}"))?;

	let number = if let Some(rest) = version_string.strip_prefix("1.") {
		// Old format (Java 8 and below)
		rest.split_once('.').map_or(rest, |(major, _)| major)
	} else if let Some((major, _)) = version_string.split_once('.') {
		// New format (Java 9 and higher)
		major
	} else {
		version_string
	};

	number.parse()
		.with_context(|| anyhow!("failed to parse {number:?} of java version {version_string:?}"))
}

#[cfg(test)]
mod testing {
	use anyhow::Result;
	use crate::dukelaunch::java_dash_version_output_to_version;

	#[test]
	fn parse_java_version_java_8() -> Result<()> {
		let stderr = "\
				openjdk version \"1.8.0_412\"\n\
				OpenJDK Runtime Environment (build 1.8.0_412-b08)\n\
				OpenJDK 64-Bit Server VM (build 25.412-b08, mixed mode)";
		let version = java_dash_version_output_to_version(stderr)?;
		assert_eq!(version, 8);
		Ok(())
	}


	#[test]
	fn parse_java_version_java_11() -> Result<()> {
		let stderr = "\
				openjdk version \"11.0.23\" 2024-04-16\n\
				OpenJDK Runtime Environment (build 11.0.23+9)\n\
				OpenJDK 64-Bit Server VM (build 11.0.23+9, mixed mode)";
		let version = java_dash_version_output_to_version(stderr)?;
		assert_eq!(version, 11);
		Ok(())
	}

	#[test]
	fn parse_java_version_java_17() -> Result<()> {
		let stderr = "\
				openjdk version \"17.0.11\" 2024-04-16\n\
				OpenJDK Runtime Environment (build 17.0.11+9)\n\
				OpenJDK 64-Bit Server VM (build 17.0.11+9, mixed mode, sharing)";
		let version = java_dash_version_output_to_version(stderr)?;
		assert_eq!(version, 17);
		Ok(())
	}

	#[test]
	fn parse_java_version_java_22() -> Result<()> {
		let stderr = "\
				openjdk version \"22\" 2024-03-19\n\
				OpenJDK Runtime Environment (build 22)\n\
				OpenJDK 64-Bit Server VM (build 22, mixed mode, sharing)";
		let version = java_dash_version_output_to_version(stderr)?;
		assert_eq!(version, 22);
		Ok(())
	}
}
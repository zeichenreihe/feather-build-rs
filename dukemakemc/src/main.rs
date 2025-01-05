use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use walkdir::WalkDir;


#[derive(Debug)]
struct FilePacket<'a> {
	name: Cow<'a, str>,
	content: Cow<'a, [u8]>,
}

#[derive(Debug)]
enum Packet<'a> {
	File(FilePacket<'a>),
	Exit,
	MultiFile(Vec<FilePacket<'a>>),
	Message {
		message: Cow<'a, str>,
	},
}

impl<'a> Packet<'a> {
	// TODO: there's some packets java has but rust doesn't
	const EXIT_PACKET: i32 = 0;
	const FILE_PACKET: i32 = 1;
	const MULTI_FILE_PACKET: i32 = 2;
	const MESSAGE_PACKET: i32 = 3;

	fn as_str(&'a self) -> Result<&'a str> {
		match self {
			Packet::Message { message } => Ok(&message),
			packet => bail!("packet is not a string packet {packet:?}"),
		}
	}

	fn as_file(&'a self) -> Result<(&'a str, &'a [u8])> {
		match self {
			Packet::File(FilePacket { name, content }) => Ok((&name, &content)),
			packet => bail!("packet is not a file packet {packet:?}"),
		}
	}
}

struct Connection {
	stream: UnixStream,
}

impl Connection {
	fn send_packet(&mut self, packet: Packet) -> Result<()> {
		fn make_id(id: i32) -> [u8; 4] {
			id.to_be_bytes()
		}
		fn len_of_data(slice: &[u8]) -> Result<[u8; 4]>  {
			i32::try_from(slice.len())
				.map(|java_int| java_int.to_be_bytes())
				.with_context(|| anyhow!("slice len doesn't fit into java int (i32): {}", slice.len()))
		}
		fn encode_len(length: usize) -> Result<[u8; 4]>  {
			i32::try_from(length)
				.map(|java_int| java_int.to_be_bytes())
				.with_context(|| anyhow!("length doesn't fit into java int (i32): {}", length))
		}

		match packet {
			// TODO: proper buffering, reducing number of write calls!
			Packet::Exit => {
				let buf = make_id(Packet::EXIT_PACKET);

				self.stream.write_all(&buf)?;
				Ok(())
			},
			Packet::File(FilePacket { name, content }) => {
				let buf: Vec<u8> = make_id(Packet::FILE_PACKET).into_iter()
					.chain(len_of_data(name.as_bytes())?)
					.chain(len_of_data(&content)?)
					.collect();

				self.stream.write_all(&buf)?;
				self.stream.write_all(name.as_bytes())?;
				self.stream.write_all(&content)?;
				Ok(())
			},
			Packet::MultiFile(files) => {
				let buf: Vec<u8> = make_id(Packet::MULTI_FILE_PACKET).into_iter()
					.chain(encode_len(files.len())?)
					.collect();
				self.stream.write_all(&buf)?;
				for FilePacket { name, content} in files {
					let buf: Vec<u8> = len_of_data(name.as_bytes())?.into_iter()
						.chain(len_of_data(&content)?)
						.collect();

					self.stream.write_all(&buf)?;
					self.stream.write_all(name.as_bytes())?;
					self.stream.write_all(&content)?;
				}
				Ok(())
			}
			Packet::Message { message } => {
				let buf: Vec<u8> = make_id(Packet::MESSAGE_PACKET).into_iter()
					.chain(len_of_data(message.as_bytes())?)
					.collect();

				self.stream.write_all(&buf)?;
				self.stream.write_all(message.as_bytes())?;
				Ok(())
			},
		}
	}

	fn recv_packet(&mut self) -> Result<Packet, PacketRecvErr> {
		fn read_i32(reader: &mut impl Read) -> Result<i32, PacketRecvErr> {
			let mut buf = [0u8; 4];
			reader.read_exact(&mut buf)
				.map_err(PacketRecvErr::Io)?;
			Ok(i32::from_be_bytes(buf))
		}
		fn read_java_len(reader: &mut impl Read) -> Result<usize, PacketRecvErr> {
			let len = read_i32(reader)?;
			if len > i32::MAX {
				return Err(PacketRecvErr::Anyhow(anyhow!("got a negative length from java: {len}")));
			}
			Ok(len as usize)
		}
		fn read_u8_vec(reader: &mut impl Read, len: usize) -> Result<Vec<u8>, PacketRecvErr> {
			let mut buf = vec![0u8; len];
			reader.read_exact(&mut buf)
				.map_err(PacketRecvErr::Io)?;
			Ok(buf)
		}
		fn vec_to_string(vec: Vec<u8>) -> Result<String, PacketRecvErr> {
			String::from_utf8(vec)
				.context("string from java isn't valid utf8")
				.map_err(PacketRecvErr::Anyhow)
		}

		let id = read_i32(&mut self.stream)?;
		match id {
			Packet::FILE_PACKET => {
				let name_len = read_java_len(&mut self.stream)?;
				let content_len = read_java_len(&mut self.stream)?;

				let name = vec_to_string(read_u8_vec(&mut self.stream, name_len)?)?;
				let content = read_u8_vec(&mut self.stream, content_len)?;

				Ok(Packet::File(FilePacket { name: Cow::Owned(name), content: Cow::Owned(content) }))
			},
			Packet::MESSAGE_PACKET => {
				let len = read_java_len(&mut self.stream)?;
				let message = vec_to_string(read_u8_vec(&mut self.stream, len)?)?;

				Ok(Packet::Message { message: Cow::Owned(message) })
			},
			_ => Err(PacketRecvErr::Anyhow(anyhow!("unknown packet id {id}"))),
		}
	}
}

enum PacketRecvErr {
	Anyhow(anyhow::Error),
	Io(std::io::Error),
}
impl PacketRecvErr {
	fn into_anyhow(self) -> anyhow::Error {
		match self {
			PacketRecvErr::Anyhow(e) => e,
			PacketRecvErr::Io(e) => anyhow::Error::from(e),
		}
	}
}

fn main() -> Result<()> {
	let cli: Cli = Cli::parse();

	println!("foo {cli:?}");

	let Cli { verbose, base_directory, command} = cli;
	let base_directory = base_directory.map_or_else(std::env::current_dir, Ok)
		.with_context(|| anyhow!("failed to get current working directory"))?;

	let files = match command {
		Command::Build { project_structure, .. } => {
			let files = get_files(&base_directory, &project_structure)?;

			files
		},
		Command::BuildDev { .. } => todo!(),
	};

	let addr = Path::new("/tmp/foo");

	// try to remove the socket is already bound
	// we ignore any errors here
	let _ = std::fs::remove_file(addr);
	let listener = UnixListener::bind(addr)?;

	let mut child = std::process::Command::new("/usr/lib/jvm/java-23-openjdk/bin/java")
		.arg("-classpath")
		.arg("/home/zeichenreihe/projects/feather-build-rs/dukemakemc/javalib")
		.arg("dukemakemc.Main")
		.arg("")
		.arg("run")
		.arg(addr)
		.spawn()?;

	let mut connection = Connection {
		stream: listener.accept().unwrap().0,
	};

	let s = "Hello Worlddddd!";
	let packet = Packet::Message { message: Cow::Borrowed(s) };
	connection.send_packet(packet)?;

	let packet = connection.recv_packet().map_err(PacketRecvErr::into_anyhow)?;
	let string = packet.as_str()?;

	println!("rust says {string:?}");

	let files: Vec<_> = files
		.into_iter()
		.map(|file| std::fs::read(&file).map(|content| (file, content)))
		.collect::<Result<_, _>>()?;
	let packets = files.iter()
		.map(|(path, content)| FilePacket {
			name: path.to_string_lossy(),
			content: Cow::Borrowed(&content)
		})
		.collect();
	connection.send_packet(Packet::MultiFile(packets))?;

	loop {
		let exit_status = child.try_wait()?;

		if let Some(exit_status) = exit_status {
			// java has exited, something went wrong
			dbg!(exit_status);
			break;
			//todo!("something went wrong!");
		} else {
			// java is still running
			let packet = connection.recv_packet();
			match packet {
				Ok(packet) => {
					dbg!(&packet);
					match packet {
						Packet::Exit => {
							break;
						},
						_ => {},
					}
				},
				Err(e) => {
					match e {
						PacketRecvErr::Anyhow(e) => { dbg!(e); },
						PacketRecvErr::Io(ioe) => {
							match ioe.kind() {
								std::io::ErrorKind::UnexpectedEof => {
									println!("unexpected eof");
									break;
								}
								_ => { dbg!(ioe); },
							}
						},
					}
				},
			}
		}
	}
/*
	let packet = connection.recv_packet()?;
	let (name, content) = packet.as_file()?;

	println!("name: {name:?}");
	println!("content: {content:x?}");*/

	let status = child.wait().unwrap();
	// TODO: handle exit status

	println!("java finished with {status:?}");

	Ok(())
}



fn get_files(base_directory: &Path, structure: &ProjectStructure) -> Result<Vec<PathBuf>> {
	match structure {
		ProjectStructure::SimpleJava => {
			let src_dir = base_directory.join("src");
			WalkDir::new(&src_dir)
				.follow_links(false) // default, TODO: make this an option for the command line?
				.into_iter()
				.filter(|res| res.as_ref().is_ok_and(|res| !res.file_type().is_dir()))
				.map(|res| res.map(|entry| entry.into_path() ))
				.collect::<Result<_, walkdir::Error>>()
				.with_context(|| anyhow!("failed to get files (recursively) for src dir {src_dir:?}"))
		},
	}
}


#[derive(Debug, Parser)]
struct Cli {
	/// Be verbose.
	#[arg(short = 'v', long = "verbose")]
	verbose: bool,

	// in the future make this a --manifest-path (like cargo-build)
	#[arg(short = 'C')]
	base_directory: Option<PathBuf>,

	#[command(subcommand)]
	command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
	/// Builds a release, using target mappings
	Build {
		#[arg(long = "structure", value_enum, default_value_t)]
		project_structure: ProjectStructure,
	},
	/// Builds the project, using named mappings
	BuildDev {

	},
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum ProjectStructure {
	#[default]
	/// A `src` folder containing '.java' files inside package directories.
	SimpleJava,
}

impl Display for ProjectStructure {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}

// store the replacement patterns somewhere

// directories: (support multiple of the ones with * after them)
// - module root
//   - src
//     - main
//       - java (source dir) *
//       - resources (resource dir) *
//     - template (not sure if we impl this or not)
//       - java (source template dir) *
//       - resources (resource template dir) *
//     - test
//       - java (source test dir) *
//       - resources (resource test dir) *

// usually takes the form of the directories above
struct JavaModule {
	name: String,

}

impl JavaModule {
	// get_java_version() -> u8
	// module_dependencies() -> iter/vec<??>
	// dependencies() -> iter/vec<??>

	// compile() -> CompileResult
	//  make a processor replacing all replacement patterns
	//  decide on args for current vs. wanted java version
	//  - walk source dirs into source files
	//  - add replaced processor as source
	//  - (depending on if test) add source test dirs (same as source dirs above)
	//  - add compile dependencies as classpath
	//  - add module dependencies compilation output to classpath
	//  and let the caller decide what to do
}

// ok so we have:
// some info on dependencies
// (some sort of "profile" to run)

// decompile:
// ???

// build:
// - get deps
// (- replace custom strings in template java files from src/template)
// - compile java files (from src/main) (including the templated templates)
// - remap to target namespace
//
// mixins need special!

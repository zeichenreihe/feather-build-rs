use std::io::Read;
use anyhow::Result;
use crate::specialized_methods::class_file::instruction::opcode::Opcode;
use crate::specialized_methods::class_file::MyRead;

pub(crate) mod opcode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Instructions;

impl Instructions {
	pub(crate) fn parse(bytes: &[u8]) -> Result<()> {
		let mut reader = OpcodeReader::new(bytes);
		while reader.pos < bytes.len() {
			Opcode::parse(&mut reader)?;
		}

		Ok(())
	}
}

struct OpcodeReader<R: Read> {
	reader: R,
	pos: usize,
}

impl<R: Read> OpcodeReader<R> {
	fn new(reader: R) -> OpcodeReader<R> {
		OpcodeReader {
			reader,
			pos: 0,
		}
	}
}

impl<R: Read> Read for OpcodeReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let n = self.reader.read(buf)?;
		self.pos += n;
		Ok(n)
	}
}

pub(crate) trait CodeReader: MyRead {
	fn move_to_next_4_byte_boundary(&mut self) -> Result<()>;
}

impl<R: Read> CodeReader for OpcodeReader<R> {
	fn move_to_next_4_byte_boundary(&mut self) -> Result<()> {
		match self.pos % 4 {
			0 => {},
			1 => drop(self.read_n::<3>()?),
			2 => drop(self.read_n::<2>()?),
			3 => drop(self.read_n::<1>()?),
			_ => unreachable!("usize % 4 can only give 0..4"),
		}
		Ok(())
	}
}
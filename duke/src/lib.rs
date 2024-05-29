//! A crate for reading and writing [Java Class Files](https://docs.oracle.com/javase/specs/jvms/se22/html/jvms-4.html).
// TODO: more doc

pub mod tree;
pub mod visitor;
mod class_reader;
mod jstring;
mod simple_class_writer;

mod macros;
mod class_constants;

use std::fmt::Debug;
use std::io::{Read, Seek, SeekFrom, Write};
use anyhow::{anyhow, bail, Context, Result};
use crate::tree::class::ClassFile;
use crate::visitor::MultiClassVisitor;

// TODO: add some traits like Hash, Eq, PartialEq, ... to most of the structs in tree:: !

// TODO: consider Display implementations of these "Name"/"Descriptor" struct in tree::

// TODO: make a variant that takes in a "Jar" and reads it...
//  done in the super crate, but maybe extract it into separate crate?
pub fn read_class_multi<V>(reader: &mut (impl Read + Seek), visitor: V) -> Result<V>
where
    V: MultiClassVisitor,
{
    class_reader::read(reader, visitor)
}

/// Reads a single java class file from the reader.
pub fn read_class(reader: &mut (impl Read + Seek)) -> Result<ClassFile> {
    let mut classes: Vec<ClassFile> = class_reader::read(reader, Vec::new())?;

    if classes.len() != 1 {
        bail!("there was no class inside it");
    }

    Ok(classes.pop().unwrap())
}

pub fn write_class(writer: &mut impl Write, class: &ClassFile) -> Result<()> {
    simple_class_writer::write(writer, class)
}

trait OptionExpansion<T> {
    fn insert_if_empty(&mut self, value: T) -> Result<()>;
}
impl<T> OptionExpansion<T> for Option<T> where T: Debug {
    fn insert_if_empty(&mut self, value: T) -> Result<()> {
        if let Some(old) = self {
            bail!("got {old:?} and {value:?}");
        } else {
            *self = Some(value);
            Ok(())
        }
    }
}

trait ClassRead {
    fn marker(&mut self) -> Result<u64>;
    fn skip(&mut self, n: i64) -> Result<()>;
    fn goto(&mut self, pos: u64) -> Result<()>;
    fn with_pos<T>(&mut self, pos: u64, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let marker = self.marker()?;
        self.goto(pos)?;
        let r = f(self)?;
        self.goto(marker)?;
        Ok(r)
    }

    fn read_n<const N: usize>(&mut self) -> Result<[u8; N]>;
    fn read_u8(&mut self) -> Result<u8> {
        Ok(u8::from_be_bytes(self.read_n().context("couldn't read u8, perhaps the data's end is reached?")?))
    }
    fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.read_n().context("couldn't read u16, perhaps the data's end is reached?")?))
    }
    fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.read_n().context("couldn't read u32, perhaps the data's end is reached?")?))
    }
    fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.read_n().context("couldn't read u64, perhaps the data's end is reached?")?))
    }
    fn read_i8(&mut self) -> Result<i8> {
        Ok(i8::from_be_bytes(self.read_n().context("couldn't read i8, perhaps the data's end is reached?")?))
    }
    fn read_i16(&mut self) -> Result<i16> {
        Ok(i16::from_be_bytes(self.read_n().context("couldn't read i16, perhaps the data's end is reached?")?))
    }
    fn read_i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.read_n().context("couldn't read i32, perhaps the data's end is reached?")?))
    }
    fn read_i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.read_n().context("couldn't read i64, perhaps the data's end is reached?")?))
    }

    fn read_u8_as_usize(&mut self) -> Result<usize> {
        Ok(self.read_u8()? as usize)
    }
    fn read_u16_as_usize(&mut self) -> Result<usize> {
        Ok(self.read_u16()? as usize)
    }
    fn read_u32_as_usize(&mut self) -> Result<usize> {
        Ok(self.read_u32()? as usize)
    }
    fn read_u8_vec(&mut self, size: usize) -> Result<Vec<u8>>;
    fn read_vec<T, S, E>(&mut self, get_size: S, mut get_element: E) -> Result<Vec<T>>
        where
            S: FnOnce(&mut Self) -> Result<usize>,
            E: FnMut(&mut Self) -> Result<T>
    {
        let size = get_size(self)?;
        let mut vec = Vec::with_capacity(size);
        for _ in 0..size {
            vec.push(get_element(self)?);
        }
        Ok(vec)
    }
}
impl<T: Read + Seek> ClassRead for T {
    fn marker(&mut self) -> Result<u64> {
        Ok(self.stream_position()?)
    }
    fn skip(&mut self, n: i64) -> Result<()> {
        self.seek(SeekFrom::Current(n))?;
        Ok(())
    }
    fn goto(&mut self, pos: u64) -> Result<()> {
        self.seek(SeekFrom::Start(pos))?;
        Ok(())
    }

    fn read_n<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
    fn read_u8_vec(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut vec = std::vec::from_elem(0, size);
        self.read_exact(&mut vec)?;
        Ok(vec)
    }
}

trait ClassWrite {
    fn write_u8(&mut self, a: u8) -> Result<()> {
        self.write_u8_slice(&[a]).context("couldn't write u8")
    }
    fn write_u16(&mut self, value: u16) -> Result<()> {
        self.write_u8_slice(&value.to_be_bytes()).context("couldn't write u16")
    }
    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.write_u8_slice(&value.to_be_bytes()).context("couldn't write u32")
    }
    fn write_u64(&mut self, value: u64) -> Result<()> {
        self.write_u8_slice(&value.to_be_bytes()).context("couldn't write u64")
    }
    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.write_u8_slice(&value.to_be_bytes()).context("couldn't write i8")
    }
    fn write_i16(&mut self, value: i16) -> Result<()> {
        self.write_u8_slice(&value.to_be_bytes()).context("couldn't write i16")
    }
    fn write_i32(&mut self, value: i32) -> Result<()> {
        self.write_u8_slice(&value.to_be_bytes()).context("couldn't write i32")
    }
    fn write_i64(&mut self, value: i64) -> Result<()> {
        self.write_u8_slice(&value.to_be_bytes()).context("couldn't write i64")
    }

    fn write_usize_as_u8(&mut self, value: usize) -> Result<()> {
        self.write_u8(u8::try_from(value).with_context(|| anyhow!("failed to convert {value} to u8 for writing: value too large"))?)
    }
    fn write_usize_as_u16(&mut self, value: usize) -> Result<()> {
        self.write_u16(u16::try_from(value).with_context(|| anyhow!("failed to convert {value} to u16 for writing: value too large"))?)
    }
    fn write_usize_as_u32(&mut self, value: usize) -> Result<()> {
        self.write_u32(u32::try_from(value).with_context(|| anyhow!("failed to convert {value} to u32 for writing: value too large"))?)
    }


    fn write_u8_slice(&mut self, buf: &[u8]) -> Result<()>;
    #[allow(clippy::needless_lifetimes)]
    // TODO: make a minimal working example out of eliding lifetimes, and possibly open a bug for clippys suggestion
    fn write_slice<'t, T>(
        &mut self,
        slice: &'t [T],
        put_size: impl FnOnce(&mut Self, usize) -> Result<()>,
        mut put_element: impl FnMut(&mut Self, &'t T) -> Result<()>
    ) -> Result<()> {
        put_size(self, slice.len())?;
        for value in slice {
            put_element(self, value)?;
        }
        Ok(())
    }
}


impl<T: Write> ClassWrite for T {
    fn write_u8_slice(&mut self, buf: &[u8]) -> Result<()> {
        self.write_all(buf).context("failed to write &[u8]")
    }
}

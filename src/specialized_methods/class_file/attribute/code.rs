use std::io::{Cursor, Read, Seek, SeekFrom};
use anyhow::{bail, Result};
use bytes::Buf;
use crate::specialized_methods::class_file::MyRead;
use crate::specialized_methods::class_file::pool::Pool;
use crate::tree::descriptor::{FieldDescriptor, MethodDescriptor};

#[derive(Debug, Clone, Default)]
pub(crate) struct CodeAnalysis {
	pub(crate) get_static: Vec<(String, String, FieldDescriptor)>,
	pub(crate) put_static: Vec<(String, String, FieldDescriptor)>,
	pub(crate) get_field: Vec<(String, String, FieldDescriptor)>,
	pub(crate) put_field: Vec<(String, String, FieldDescriptor)>,
	pub(crate) invoke_virtual: Vec<(String, String, MethodDescriptor)>,
	pub(crate) invoke_special: Vec<(String, String, MethodDescriptor)>,
	pub(crate) invoke_static: Vec<(String, String, MethodDescriptor)>,
	pub(crate) invoke_interface: Vec<(String, String, MethodDescriptor)>,
}

fn seek_to_four_byte_boundary<R>(reader: &mut R) -> Result<()>
where
	R: Read + Seek,
{
	let pos = reader.stream_position()?;

	let len = (4 - (pos % 4)) % 4;

	reader.seek(SeekFrom::Current(len as i64))?;

	Ok(())
}

impl CodeAnalysis {
	pub(crate) fn analyze(code: Vec<u8>, pool: &Pool) -> Result<CodeAnalysis> {
		let mut analysis = CodeAnalysis::default();

		let mut cursor = Cursor::new(code);

		while cursor.has_remaining() {
			for_each_opcode(&mut cursor, &mut analysis, pool)?;
		}

		Ok(analysis)
	}
}

fn for_each_opcode<R>(reader: &mut R, analysis: &mut CodeAnalysis, pool: &Pool) -> Result<()>
where
	R: Read + Seek,
{
	match reader.read_u8()? {
		0x00 | // nop
		0x01 | // aconst_null
		0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x08 | // iconst_<i>
		0x09 | 0x0a | // lconst_<l>
		0x0b | 0x0c | 0x0d | // fconst_<f>
		0x0e | 0x0f => {}, // dconst_<d>
		0x10 => { // bipush
			let _byte = reader.read_u8()?;
		},
		0x11 => { // sipush
			let _byte = reader.read_u16()?;
		},
		0x12 => { // ldc
			let _index = reader.read_u8()?;
		},
		0x13 | // ldc_w
		0x14 => { // ldc2_w
			let _index = reader.read_u16()?;
		},
		0x15 | // iload
		0x16 | // lload
		0x17 | // fload
		0x18 | // dload
		0x19 => { // aload
			let _index = reader.read_u8()?;
		},
		0x1a | 0x1b | 0x1c | 0x1d | // iload_<n>
		0x1e | 0x1f | 0x20 | 0x21 | // lload_<n>
		0x22 | 0x23 | 0x24 | 0x25 | // fload_<n>
		0x26 | 0x27 | 0x28 | 0x29 | // dload_<n>
		0x2a | 0x2b | 0x2c | 0x2d | // aload_<n>
		0x2e | // iaload
		0x2f | // laload
		0x30 | // faload
		0x31 | // daload
		0x32 | // aaload
		0x33 | // baload
		0x34 | // caload
		0x35 => {}, // saload
		0x36 | // istore
		0x37 | // lstore
		0x38 | // fstore
		0x39 | // dstore
		0x3a => { // astore
			let _index = reader.read_u8()?;
		},
		0x3b | 0x3c | 0x3d | 0x3e | // istore_<n>
		0x3f | 0x40 | 0x41 | 0x42 | // lstore_<n>
		0x43 | 0x44 | 0x45 | 0x46 | // fstore_<n>
		0x47 | 0x48 | 0x49 | 0x4a | // dstore_<n>
		0x4b | 0x4c | 0x4d | 0x4e | // astore_<n>
		0x4f | // istore
		0x50 | // lastore
		0x51 | // fastore
		0x52 | // dastore
		0x53 | // aastore
		0x54 | // bastore
		0x55 | // castore
		0x56 | // sastore
		0x57 | // pop
		0x58 | // pop2
		0x59 | // dup
		0x5a | // dup_x1
		0x5b | // dup_x2
		0x5c | // dup2
		0x5d | // dup2_x1
		0x5e | // dup2_x2
		0x5f | // swap
		0x60 | // iadd
		0x61 | // ladd
		0x62 | // fadd
		0x63 | // dadd
		0x64 | // isub
		0x65 | // lsub
		0x66 | // fsub
		0x67 | // dsub
		0x68 | // imul
		0x69 | // lmul
		0x6a | // fmul
		0x6b | // dmul
		0x6c | // idiv
		0x6d | // ldiv
		0x6e | // fdiv
		0x6f | // ddiv
		0x70 | // irem
		0x71 | // lrem
		0x72 | // frem
		0x73 | // drem
		0x74 | // ineg
		0x75 | // lneg
		0x76 | // fneg
		0x77 | // dneg
		0x78 | // ishl
		0x79 | // lshl
		0x7a | // ishr
		0x7b | // lshr
		0x7c | // iushr
		0x7d | // lushr
		0x7e | // iand
		0x7f | // land
		0x80 | // ior
		0x81 | // lor
		0x82 | // ixor
		0x83 => {}, // lxor
		0x84 => { // iinc
			let _index = reader.read_u8()?;
			let _const = reader.read_u8()?;
		},
		0x85 | // i2l
		0x86 | // i2f
		0x87 | // i2d
		0x88 | // l2i
		0x89 | // l2f
		0x8a | // l2d
		0x8b | // f2i
		0x8c | // f2l
		0x8d | // f2d
		0x8e | // d2i
		0x8f | // d2l
		0x90 | // d2f
		0x91 | // i2b
		0x92 | // i2c
		0x93 | // i2s
		0x94 | // lcmp
		0x95 | 0x96 | // fcmp<op>
		0x97 | 0x98 => {}, // dcmp<op>
		0x99 | 0x9a | 0x9b | 0x9c | 0x9d | 0x9e | // if<cond>
		0x9f | 0xa0 | 0xa1 | 0xa2 | 0xa3 | 0xa4 | // if_icmp<cond>
		0xa5 | 0xa6 | // if_acmp<cond>
		0xa7 | // goto
		0xa8 => { // jsr
			let _branch = reader.read_i16()?;
		},
		0xa9 => { // ret
			let _index = reader.read_u8()?;
		},
		0xaa => { // tableswitch
			seek_to_four_byte_boundary(reader)?;

			let default_target = reader.read_i32()?;
			let low = reader.read_i32()?;
			let high = reader.read_i32()?;

			let mut targets = Vec::new();
			for _ in low + 1 .. high {
				let branch_target = reader.read_i32()?;

				targets.push(branch_target);
			}
		},
		0xab => { // lookupswitch
			seek_to_four_byte_boundary(reader)?;

			let default_target = reader.read_i32()?;
			let npairs = reader.read_u32_as_usize()?;

			let mut targets = Vec::with_capacity(npairs);
			for _ in 0..npairs {
				let match_ = reader.read_i32()?;
				let branch_target = reader.read_i32()?;

				targets.push((match_, branch_target));
			}
		},
		0xac | // ireturn
		0xad | // lreturn
		0xae | // freturn
		0xaf | // dreturn
		0xb0 | // areturn
		0xb1 => {}, // return
		0xb2 => { // getstatic
			let index = reader.read_u16_as_usize()?;

			analysis.get_static.push(pool.get_field_ref(index)?);
		},
		0xb3 => { // putstatic
			let index = reader.read_u16_as_usize()?;

			analysis.put_static.push(pool.get_field_ref(index)?);
		},
		0xb4 => { // getfield
			let index = reader.read_u16_as_usize()?;

			analysis.get_field.push(pool.get_field_ref(index)?);
		},
		0xb5 => { // putfield
			let index = reader.read_u16_as_usize()?;

			analysis.put_field.push(pool.get_field_ref(index)?);
		},
		0xb6 => { // invokevirtual
			let index = reader.read_u16_as_usize()?;

			analysis.invoke_virtual.push(pool.get_method_ref(index)?);
		},
		0xb7 => { // invokespecial
			let index = reader.read_u16_as_usize()?;

			analysis.invoke_special.push(pool.get_method_ref(index)?);
		},
		0xb8 => { // invokestatic
			let index = reader.read_u16_as_usize()?;

			analysis.invoke_static.push(pool.get_method_ref(index)?);
		},
		0xb9 => { // invokeinterface
			let index = reader.read_u16_as_usize()?;
			let _count = reader.read_u8()?;
			let _zero = reader.read_u8()?;

			analysis.invoke_interface.push(pool.get_method_ref(index)?);
		},
		0xba => { // invokedynamic
			let _index = reader.read_u16_as_usize()?;
			let _zero = reader.read_u16()?;
		},
		0xbb => { // new
			let _index = reader.read_u16()?;
		},
		0xbc => { // newarray
			let _atype = reader.read_u8()?;
		},
		0xbd => { // anewarray
			let _index = reader.read_u16()?;
		},
		0xbe | // arraylength
		0xbf => {}, // athrow
		0xc0 | // checkcast
		0xc1 => { // instanceof
			let _index = reader.read_u16()?;
		},
		0xc2 | // monitorenter
		0xc3 => {}, // monitorexit
		0xc4 => { // wide
			match reader.read_u8()? {
				0x15 | // iload
				0x16 | // lload
				0x17 | // fload
				0x18 | // dload
				0x19 | // aload
				0x36 | // istore
				0x37 | // lstore
				0x38 | // fstore
				0x39 | // dstore
				0x3a => { // astore
					let _index = reader.read_u16()?;
				},
				0x84 => { // iinc
					let _index = reader.read_u16()?;
					let _const = reader.read_u16()?;
				},
				0xa0 => { // ret
					let _index = reader.read_u16()?;
				},
				opcode => bail!("illegal wide opcode: {opcode:x}"),
			}
		},
		0xc5 => { // multianewarray
			let _index = reader.read_u16()?;
			let _dimensions = reader.read_u8()?;
		},
		0xc6 | // ifnull,
		0xc7 => { // ifnonnull
			let _branch = reader.read_i16()?;
		},
		0xc8 | // goto_w
		0xc9 => { // jsr_w
			let _branch = reader.read_i32()?;
		},

		0xca => {}, // breakpoint

		0xfe => {}, // impdep1
		0xff => {}, // impdep2

		opcode => bail!("Illegal opcode {opcode:x}"),
	}
	Ok(())
}
macro_rules! jvms_notation {
	// rules used for writing
	(write, $_w:ident, $_v:expr, $_t:ident $(<$_it:tt> $([$_iat:tt])?)? ;$_nw:ident) => {};
	(write, $w:ident, $v:expr, Vec<$it:tt> $([$iat:tt])? ) => {
		$( jvms_notation!(write, $w, $v.len() as $iat, $iat); )?
		for i in $v {
			jvms_notation!(write, $w, i, $it);
		}
	};
	(write, $w:ident, $v:expr, u8) => { std::io::Write::write_all($w, &$v.to_be_bytes())?; };
	(write, $w:ident, $v:expr, u16) => { std::io::Write::write_all($w, &$v.to_be_bytes())?; };
	(write, $w:ident, $v:expr, u32) => { std::io::Write::write_all($w, &$v.to_be_bytes())?; };
	(write, $w:ident, $v:expr, $_t:ty) => { $v.jvms_write($w)?; };
	// rules used for reading
	(read, $_r:ident, $_p:ident, $_t:ident $(<$_it:tt> $([$_iat:tt])?)? ;$_nw:ident = $nwe:expr) => { $nwe };
	(read, $r:ident, $p:ident, Vec<$it:tt> $([$iat:tt])? $({$l:expr})? ) => {{
		$( let len = jvms_notation!(read, $r, $p, $iat); )?
		$( let len = $l; )?
		let mut vec = Vec::with_capacity(len as usize);
		for _ in 0..len {
			let i = jvms_notation!(read, $r, $p, $it);
			vec.push(i);
		}
		vec
	}};
	(read, $r:ident, $_p:ident, u8) => {{
		let mut buf = [0u8; 1];
		$r.read_exact(&mut buf)?;
		u8::from_be_bytes(buf)
	}};
	(read, $r:ident, $_p:ident, u16) => {{
		let mut buf = [0u8; 2];
		$r.read_exact(&mut buf)?;
		u16::from_be_bytes(buf)
	}};
	(read, $r:ident, $_p:ident, u32) => {{
		let mut buf = [0u8; 4];
		$r.read_exact(&mut buf)?;
		u32::from_be_bytes(buf)
	}};
	(read, $r:ident, $p:ident, $t:ty) => {
		<$t>::jvms_read($r, $p)?
	};
	// rules used for checking read constants
	(check, $c:ident, $cv:literal) => {
		if $c != $cv {
			return Err(std::io::Error::other(format!("Unexpected constant value: expected {:?} (0x{:x?}), got {:?} (0x{:x?}) for constant `{}`", $cv, $cv, $c, $c, stringify!($c))));
		}
	};
	(check, $c:ident, $_cv:expr) => { let _ = $c; };
	// rules used for calculating lengths
	(len, $v:expr, Vec<$it:tt> $([$iat:tt])? ) => {{
		let mut len = 0 $( + jvms_notation!(len, $v, $iat) )?;
		for _i in $v {
			len += jvms_notation!(len, _i, $it);
		}
		len
	}};
	(len, $_v:expr, u8) => { 1 };
	(len, $_v:expr, u16) => { 2 };
	(len, $_v:expr, u32) => { 4 };
	(len, $v:expr, $_t:ty) => { $v.jvms_len() };
	// rules actually used in the definition of things
	(
		struct $n:ident $($s:ident)? {
			$( const $c_0:ident: $ct_0:ident = $cv_0:expr, )*
			$(
				mut $i:ident: $it:ident $( <$iit:tt> $([$iat:tt])? $({$l:expr})? )? $( ;$ps:expr )?,
				$( const $c_1:ident: $ct_1:ident = $cv_1:expr, )*
			)*
		}
	) => {
		#[derive(Debug, Clone, PartialEq)]
		pub struct $n {
			$( pub $i: $it $(<$iit>)?, )*
		}

		impl $n {
			fn jvms_write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
				$( let $s = self; )?
				$( let _ = $s; )?
				$( jvms_notation!(write, writer, $cv_0 as $ct_0, $ct_0); )*
				$(
					jvms_notation!(write, writer, &self.$i, $it $( <$iit> $([$iat])? )?);
					$( jvms_notation!(write, writer, $cv_1 as $ct_1, $ct_1); )*
				)*
				Ok(())
			}

			fn jvms_read(reader: &mut impl std::io::Read, pool: Option<&Vec<CpInfo>>) -> std::io::Result<$n> {
				$( let $c_0 = jvms_notation!(read, reader, pool, $ct_0); jvms_notation!(check, $c_0, $cv_0); )*
				$(
					let $i = jvms_notation!(read, reader, pool, $it $( <$iit> $([$iat])? $({$l})? )?);
					$( let pool = $ps; )?
					$( let $c_1 = jvms_notation!(read, reader, pool, $ct_1); jvms_notation!(check, $c_1, $cv_1); )*
				)*
				let _ = pool;
				Ok($n {
					$( $i, )*
				})
			}

			fn jvms_len(&self) -> u32 {
				$( let $s = self; )?
				$( let _ = $s; )?
				0
				$( + jvms_notation!(len, $cv_0, $ct_0) )*
				$(
					+ jvms_notation!(len, &self.$i, $it $( <$iit> $([$iat])? )? )
					$( + jvms_notation!(len, $cv_1, $ct_1) )*
				)*
			}
		}
	};
	(
		enum $n:ident $([$p:ident])? {
			$t:ident: $tt:ident,
			$( $v:ident $($s:ident)? {
				= $tv:expr => $tm:pat $(if $tme:expr)?,
				$( const $c_0:ident: $ct_0:ident = $cv_0:expr, )*
				$(
					mut $i:ident: $it:ident $( <$iit:tt> $([$iat:tt])? $({$l:expr})? )? $($nw:ident = $nwe:expr)?,
					$( const $c_1:ident: $ct_1:ident = $cv_1:expr, )*
				)*
			}, )*
			$( _ { $fm:pat => $f:expr, }, )?
		}
	) => {
		#[derive(Debug, Clone, PartialEq)]
		pub enum $n {
			$( $v {
				$( $i: $it $(<$iit>)?, )*
			}, )*
		}

		impl $n {
			fn jvms_write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
				match self {
					$( $($s @ )? $n::$v {
						$( $i, )*
					} => {
						$( let _ = $s; )?
						jvms_notation!(write, writer, $tv as $tt, $tt);
						$( jvms_notation!(write, writer, $cv_0 as $ct_0, $ct_0); )*
						$(
							jvms_notation!(write, writer, $i, $it $( <$iit> $([$iat])? )? $(;$nw)?);
							$( jvms_notation!(write, writer, $cv_1 as $ct_1, $ct_1); )*
						)*
					}, )*
				}
				Ok(())
			}

			#[allow(clippy::redundant_locals)]
			fn jvms_read(reader: &mut impl std::io::Read, pool: Option<&Vec<CpInfo>>) -> std::io::Result<$n> {
				$( let $p = pool; )?
				let $t = jvms_notation!(read, reader, pool, $tt);
				match $t {
					$( $tm $( if $tme )?=> {
						$( let $c_0 = jvms_notation!(read, reader, pool, $ct_0); jvms_notation!(check, $c_0, $cv_0); )*
						$(
							let $i = jvms_notation!(read, reader, pool, $it $( <$iit> $([$iat])? $({$l})? )? $(;$nw = $nwe)?);
							$( let $c_1 = jvms_notation!(read, reader, pool, $ct_1); jvms_notation!(check, $c_1, $cv_1); )*
						)*
						let _ = pool;
						Ok($n::$v {
							$( $i, )*
						})
					}, )*
					$( $fm => { $f }, )?
				}
			}

			fn jvms_len(&self) -> u32 {
				match self {
					$( $($s @ )? $n::$v {
						$( $i, )*
					} => {
						$( let _ = $s; )?
						0
						$( + jvms_notation!(len, $cv_0, $ct_0) )*
						$(
							+ { let _i = $i; jvms_notation!(len, _i, $it $( <$iit> $([$iat])? )? ) }
							$( + jvms_notation!(len, $cv_1, $ct_1) )*
						)*
					}, )*
				}
			}
		}
	}
}

pub(super) use jvms_notation;

macro_rules! notation {
	// rules used for writing
	(write, $_w:ident, $_v:expr, $_t:ident $(<$_it:tt> $([$_iat:tt])?)? ;$_nw:ident) => {};
	(write, $w:ident, $v:expr, Vec<$it:tt> $([$iat:tt])? ) => {
		$( notation!(write, $w, $v.len() as $iat, $iat); )?
		for i in $v {
			notation!(write, $w, i, $it);
		}
	};
	(write, $w:ident, $v:expr, u8) => { std::io::Write::write_all($w, &$v.to_be_bytes())?; };
	(write, $w:ident, $v:expr, u16) => { std::io::Write::write_all($w, &$v.to_be_bytes())?; };
	(write, $w:ident, $v:expr, u32) => { std::io::Write::write_all($w, &$v.to_be_bytes())?; };
	(write, $w:ident, $v:expr, $_t:ty) => { $v._write($w)?; };
	// rules used for reading
	(read, $_r:ident, $_p:ident, $_t:ident $(<$_it:tt> $([$_iat:tt])?)? ;$_nw:ident = $nwe:expr) => { $nwe };
	(read, $r:ident, $p:ident, Vec<$it:tt> $([$iat:tt])? $({$l:expr})? ) => {{
		$( let len = notation!(read, $r, $p, $iat); )?
		$( let len = $l; )?
		let mut vec = Vec::with_capacity(len as usize);
		for _ in 0..len {
			let i = notation!(read, $r, $p, $it);
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
		<$t>::_read($r, $p)?
	};
	// rules used for checking read constants
	(check, $c:ident, $cv:literal) => {
		if $c != $cv {
			return Err(std::io::Error::other(format!(
				"Unexpected constant value: expected {:?} (0x{:x?}), got {:?} (0x{:x?}) for constant `{}`", $cv, $cv, $c, $c, stringify!($c)
			)));
		}
	};
	(check, $c:ident, $_cv:expr) => { let _ = $c; };
	// rules used for calculating lengths
	(len, $_v:expr, $_t:ident $(<$_it:tt> $([$_iat:tt])?)? ;$_nw:ident) => { 0 };
	(len, $v:expr, Vec<$it:tt> $([$iat:tt])? ) => {{
		let mut len = 0 $( + notation!(len, $v, $iat) )?;
		for _i in $v {
			len += notation!(len, _i, $it);
		}
		len
	}};
	(len, $_v:expr, u8) => { 1 };
	(len, $_v:expr, u16) => { 2 };
	(len, $_v:expr, u32) => { 4 };
	(len, $v:expr, $_t:ty) => { $v._len() };
	// rules actually used in the definition of things
	(
		$( #[$nd:meta] )?
		struct $n:ident $($s:ident)? {
			$( const $c_0:ident: $ct_0:ident = $cv_0:expr, )*
			$(
				$( #[$id:meta] )?
				mut $i:ident: $it:ident $( <$iit:tt> $([$iat:tt])? $({$l:expr})? )? $( ;$ps:expr )?,
				$( const $c_1:ident: $ct_1:ident = $cv_1:expr, )*
			)*
		}
	) => {
		$( #[$nd] )?
		#[derive(Debug, Clone, PartialEq)]
		pub struct $n {
			$(
				$( #[$id] )?
				pub $i: $it $(<$iit>)?,
			)*
		}

		impl $n {
			fn _write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
				$( let $s = self; )?
				$( let _ = $s; )?
				$( notation!(write, writer, $cv_0 as $ct_0, $ct_0); )*
				$(
					notation!(write, writer, &self.$i, $it $( <$iit> $([$iat])? )?);
					$( notation!(write, writer, $cv_1 as $ct_1, $ct_1); )*
				)*
				Ok(())
			}

			fn _read(reader: &mut impl std::io::Read, pool: Option<&Vec<CpInfo>>) -> std::io::Result<$n> {
				$( let $c_0 = notation!(read, reader, pool, $ct_0); notation!(check, $c_0, $cv_0); )*
				$(
					let $i = notation!(read, reader, pool, $it $( <$iit> $([$iat])? $({$l})? )?);
					$( let pool = $ps; )?
					$( let $c_1 = notation!(read, reader, pool, $ct_1); notation!(check, $c_1, $cv_1); )*
				)*
				let _ = pool;
				Ok($n {
					$( $i, )*
				})
			}

			fn _len(&self) -> u32 {
				$( let $s = self; )?
				$( let _ = $s; )?
				0
				$( + notation!(len, $cv_0, $ct_0) )*
				$(
					+ notation!(len, &self.$i, $it $( <$iit> $([$iat])? )? )
					$( + notation!(len, $cv_1, $ct_1) )*
				)*
			}
		}
	};
	(
		$( #[$nd:meta] )?
		enum $n:ident $([$p:ident])? {
			$t:ident: $tt:ident,
			$(
				$( #[$vd:meta] )?
				$v:ident $($s:ident)? {
					= $tv:expr => $tm:pat $(if $tme:expr)?,
					$( const $c_0:ident: $ct_0:ident = $cv_0:expr, )*
					$(
						$( #[$id:meta] )?
						mut $i:ident: $it:ident $( <$iit:tt> $([$iat:tt])? $({$l:expr})? )? $($nw:ident = $nwe:expr)?,
						$( const $c_1:ident: $ct_1:ident = $cv_1:expr, )*
					)*
				},
			)*
			$( _ { $fm:pat => $f:expr, }, )?
		}
	) => {
		$( #[$nd] )?
		#[derive(Debug, Clone, PartialEq)]
		pub enum $n {
			$(
				$( #[$vd] )?
				$v {
					$(
						$( #[$id] )?
						$i: $it $(<$iit>)?,
					)*
				},
			)*
		}

		impl $n {
			fn _write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
				match self {
					$( $($s @ )? $n::$v {
						$( $i, )*
					} => {
						$( let _ = $s; )?
						notation!(write, writer, $tv as $tt, $tt);
						$( notation!(write, writer, $cv_0 as $ct_0, $ct_0); )*
						$(
							notation!(write, writer, $i, $it $( <$iit> $([$iat])? )? $(;$nw)?);
							$( notation!(write, writer, $cv_1 as $ct_1, $ct_1); )*
						)*
					}, )*
				}
				Ok(())
			}

			#[allow(clippy::redundant_locals)]
			fn _read(reader: &mut impl std::io::Read, pool: Option<&Vec<CpInfo>>) -> std::io::Result<$n> {
				$( let $p = pool; )?
				let $t = notation!(read, reader, pool, $tt);
				match $t {
					$( $tm $( if $tme )?=> {
						$( let $c_0 = notation!(read, reader, pool, $ct_0); notation!(check, $c_0, $cv_0); )*
						$(
							let $i = notation!(read, reader, pool, $it $( <$iit> $([$iat])? $({$l})? )? $(;$nw = $nwe)?);
							$( let $c_1 = notation!(read, reader, pool, $ct_1); notation!(check, $c_1, $cv_1); )*
						)*
						let _ = pool;
						Ok($n::$v {
							$( $i, )*
						})
					}, )*
					$( $fm => { $f }, )?
				}
			}

			fn _len(&self) -> u32 {
				match self {
					$( $($s @ )? $n::$v {
						$( $i, )*
					} => {
						$( let _ = $s; )?
						notation!(len, $tv as $tt, $tt)
						$( + notation!(len, $cv_0, $ct_0) )*
						$(
							+ { let _i = $i; notation!(len, _i, $it $( <$iit> $([$iat])? )? $(;$nw)? ) }
							$( + notation!(len, $cv_1, $ct_1) )*
						)*
					}, )*
				}
			}
		}
	}
}

pub(super) use notation;

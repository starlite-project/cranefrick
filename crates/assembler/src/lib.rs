#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::{
	error::Error as StdError,
	fmt::{Debug, Display, Error as FmtError, Formatter, Result as FmtResult},
	io::{self, Error as IoError, prelude::*},
	path::Path,
	slice,
};

use frick_ir::BrainIr;

#[derive(Debug)]
pub enum AssemblyError<E> {
	Custom(&'static str),
	Backend(E),
	Fmt(FmtError),
	NotImplemented(BrainIr),
	Io(IoError),
}

impl<E> AssemblyError<E> {
	pub fn backend(e: impl Into<E>) -> Self {
		Self::Backend(e.into())
	}
}

impl<E> Display for AssemblyError<E> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::NotImplemented(i) => {
				f.write_str("instruction ")?;
				Debug::fmt(&i, f)?;
				f.write_str(" is not yet implemented")
			}
			Self::Io(..) => f.write_str("an IO error occurred"),
			Self::Custom(s) => f.write_str(s),
			Self::Backend(..) => f.write_str("an error occurred from the backend"),
			Self::Fmt(..) => f.write_str("an error occurred during formatting"),
		}
	}
}

impl<E> StdError for AssemblyError<E>
where
	E: StdError + 'static,
{
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Fmt(e) => Some(e),
			Self::Io(e) => Some(e),
			Self::Backend(e) => Some(e),
			Self::Custom(..) | Self::NotImplemented(..) => None,
		}
	}
}

impl<E: InnerAssemblyError> From<E> for AssemblyError<E> {
	fn from(value: E) -> Self {
		Self::Backend(value)
	}
}

impl<E> From<FmtError> for AssemblyError<E> {
	fn from(value: FmtError) -> Self {
		Self::Fmt(value)
	}
}

impl<E> From<IoError> for AssemblyError<E> {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

pub trait Assembler {
	type Error: StdError + 'static;
	type Module<'ctx>: AssembledModule
	where
		Self: 'ctx;

	fn assemble<'ctx>(
		&'ctx self,
		ops: &[BrainIr],
		output_path: &Path,
	) -> Result<Self::Module<'ctx>, AssemblyError<Self::Error>>;
}

pub trait AssembledModule {
	type Error: StdError + 'static;

	fn execute(&self) -> Result<(), Self::Error>;
}

pub trait InnerAssemblyError {}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn frick_assembler_write(value: u32) {
	if cfg!(target_os = "windows") && value >= 128 {
		return;
	}

	let mut stdout = io::stdout().lock();

	let value = {
		let Some(ch) = char::from_u32(value) else {
			return;
		};

		ch as u8
	};

	let result = stdout
		.write_all(slice::from_ref(&value))
		.and_then(|()| stdout.flush());

	if let Err(e) = result {
		panic!("error occurred during write: {e}");
	}
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn frick_assembler_read(buf: *mut u8) {
	let mut stdin = io::stdin().lock();
	loop {
		let mut value = 0;
		let err = stdin.read_exact(slice::from_mut(&mut value));

		if let Err(e) = err {
			assert!(
				matches!(e.kind(), io::ErrorKind::UnexpectedEof),
				"error occurred during read: {e}"
			);

			value = 0;
		}

		if cfg!(target_os = "windows") && matches!(value, b'\r') {
			continue;
		}

		unsafe {
			*buf = value;
		}

		break;
	}
}

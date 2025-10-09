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
pub enum AssemblyError<E: InnerAssemblyError> {
	Custom(&'static str),
	Backend(E),
	Fmt(FmtError),
	NotImplemented(BrainIr),
	Io(IoError),
}

impl<E: InnerAssemblyError> Display for AssemblyError<E> {
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

impl<E: InnerAssemblyError> StdError for AssemblyError<E> {
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

impl<E: InnerAssemblyError> From<FmtError> for AssemblyError<E> {
	fn from(value: FmtError) -> Self {
		Self::Fmt(value)
	}
}

impl<E: InnerAssemblyError> From<IoError> for AssemblyError<E> {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

pub trait Assembler {
	type Error: InnerAssemblyError;
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

pub trait InnerAssemblyError: StdError + 'static {}

pub const TAPE_SIZE: usize = 0x8000;

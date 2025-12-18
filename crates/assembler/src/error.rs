use std::{
	borrow::Cow,
	error::Error as StdError,
	fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _},
	io::Error as IoError,
};

use frick_instructions::BrainInstructionType;
use frick_utils::Convert as _;
use inkwell::{builder::BuilderError, support::LLVMString, values::InstructionValueError};
use send_wrapper::SendWrapper;

#[derive(Debug)]
pub enum AssemblyError {
	Llvm(SendWrapper<LLVMString>),
	NoTargetMachine,
	IntrinsicNotFound(Cow<'static, str>),
	InvalidIntrinsicDeclaration(Cow<'static, str>),
	Inkwell(inkwell::Error),
	NotImplemented(BrainInstructionType),
	Io(IoError),
	PointerNotLoaded,
	NoValueInRegister(usize),
	NoLoopInfo,
	CannotGetConstant,
}

impl AssemblyError {
	pub(crate) const fn intrinsic_not_found(s: &'static str) -> Self {
		Self::IntrinsicNotFound(Cow::Borrowed(s))
	}

	pub(crate) const fn invalid_intrinsic_declaration(s: &'static str) -> Self {
		Self::InvalidIntrinsicDeclaration(Cow::Borrowed(s))
	}
}

impl Display for AssemblyError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Llvm(..) => f.write_str("an error occurred from LLVM"),
			Self::NoTargetMachine => f.write_str("unable to get target machine"),
			Self::Inkwell(..) => f.write_str("an error occurred during translation"),
			Self::IntrinsicNotFound(intrinsic) => {
				f.write_str("intrinsic \"")?;
				f.write_str(intrinsic)?;
				f.write_str("\" was not found")
			}
			Self::InvalidIntrinsicDeclaration(intrinsic) => {
				f.write_str("invalid declaration for intrinsic \"")?;
				f.write_str(intrinsic)?;
				f.write_char('"')
			}
			Self::NotImplemented(BrainInstructionType::NotImplemented) => {
				f.write_str("an operation is not implemented")
			}
			Self::NotImplemented(i) => {
				f.write_str("instruction ")?;
				Debug::fmt(&i, f)?;
				f.write_str(" is not implemented")
			}
			Self::Io(..) => f.write_str("an IO error has occurred"),
			Self::PointerNotLoaded => {
				f.write_str("pointer was not loaded before indexing into tape")
			}
			Self::NoValueInRegister(slot) => {
				f.write_str("no value was found in register ")?;
				Display::fmt(&slot, f)
			}
			Self::NoLoopInfo => f.write_str("no loop info was present when expected"),
			Self::CannotGetConstant => f.write_str("cannot create LLVM value from rust constant"),
		}
	}
}

impl StdError for AssemblyError {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Inkwell(e) => Some(e),
			Self::Llvm(e) => Some(&**e),
			Self::Io(e) => Some(e),
			Self::NoTargetMachine
			| Self::IntrinsicNotFound(..)
			| Self::InvalidIntrinsicDeclaration(..)
			| Self::NotImplemented(..)
			| Self::PointerNotLoaded
			| Self::NoValueInRegister(..)
			| Self::NoLoopInfo
			| Self::CannotGetConstant => None,
		}
	}
}

impl From<LLVMString> for AssemblyError {
	fn from(value: LLVMString) -> Self {
		Self::Llvm(SendWrapper::new(value))
	}
}

impl From<BuilderError> for AssemblyError {
	fn from(value: BuilderError) -> Self {
		Self::Inkwell(value.convert::<inkwell::Error>())
	}
}

impl From<inkwell::Error> for AssemblyError {
	fn from(value: inkwell::Error) -> Self {
		Self::Inkwell(value)
	}
}

impl From<InstructionValueError> for AssemblyError {
	fn from(value: InstructionValueError) -> Self {
		Self::Inkwell(value.convert::<inkwell::Error>())
	}
}

impl From<IoError> for AssemblyError {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod compiler;

use alloc::vec::Vec;

use cranefrick_hlir::BrainHlir;
use serde::{Deserialize, Serialize};

pub use self::compiler::*;

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainMlir {
	ChangeCell(i8),
	MovePtr(i64),
	SetCell(i8),
	GetInput,
	PutOutput,
	JumpRight,
	JumpLeft,
	DynamicLoop(Vec<Self>),
}

impl BrainMlir {
	#[must_use]
	pub const fn change_cell(value: i8) -> Self {
		Self::ChangeCell(value)
	}

	#[must_use]
	pub const fn move_ptr(offset: i64) -> Self {
		Self::MovePtr(offset)
	}

	#[must_use]
	pub const fn set_cell(value: i8) -> Self {
		Self::SetCell(value)
	}

	#[must_use]
	pub const fn get_input() -> Self {
		Self::GetInput
	}

	#[must_use]
	pub const fn put_output() -> Self {
		Self::PutOutput
	}

	#[must_use]
	pub const fn start_loop() -> Self {
		Self::JumpRight
	}

	#[must_use]
	pub const fn end_loop() -> Self {
		Self::JumpLeft
	}

	#[must_use]
	pub fn dynamic_loop(instrs: impl IntoIterator<Item = Self>) -> Self {
		Self::DynamicLoop(instrs.into_iter().collect())
	}
}

impl From<BrainHlir> for BrainMlir {
	fn from(value: BrainHlir) -> Self {
		match value {
			BrainHlir::IncrementCell => Self::ChangeCell(1),
			BrainHlir::DecrementCell => Self::ChangeCell(-1),
			BrainHlir::MovePtrLeft => Self::MovePtr(-1),
			BrainHlir::MovePtrRight => Self::MovePtr(1),
			BrainHlir::GetInput => Self::GetInput,
			BrainHlir::PutOutput => Self::PutOutput,
			BrainHlir::StartLoop => Self::JumpRight,
			BrainHlir::EndLoop => Self::JumpLeft,
		}
	}
}

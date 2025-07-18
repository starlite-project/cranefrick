#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod compiler;

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

pub use self::compiler::*;

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainMlir {
	ChangeCell(i8, i64),
	MovePtr(i64),
	SetCell(u8),
	GetInput,
	PutOutput,
	DynamicLoop(Vec<Self>),
}

impl BrainMlir {
	#[must_use]
	pub const fn change_cell(value: i8) -> Self {
		Self::change_cell_at(value, 0)
	}

	#[must_use]
	pub const fn change_cell_at(value: i8, offset: i64) -> Self {
		Self::ChangeCell(value, offset)
	}

	#[must_use]
	pub const fn move_ptr(offset: i64) -> Self {
		Self::MovePtr(offset)
	}

	#[must_use]
	pub const fn set_cell(value: u8) -> Self {
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
	pub fn dynamic_loop(instrs: impl IntoIterator<Item = Self>) -> Self {
		Self::DynamicLoop(instrs.into_iter().collect())
	}
}

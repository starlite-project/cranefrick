#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod compiler;

use alloc::vec::Vec;
use core::num::NonZeroI32;

use serde::{Deserialize, Serialize};

pub use self::compiler::*;

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainMlir {
	ChangeCell(
		i8,
		#[serde(skip_serializing_if = "Option::is_none")] Option<NonZeroI32>,
	),
	MovePointer(i32),
	SetCell(
		u8,
		#[serde(skip_serializing_if = "Option::is_none")] Option<NonZeroI32>,
	),
	GetInput,
	PutOutput,
	DynamicLoop(Vec<Self>),
	ScaleAndMoveValue(u8, i32),
}

impl BrainMlir {
	#[must_use]
	pub const fn change_cell(value: i8) -> Self {
		Self::change_cell_at(value, 0)
	}

	#[must_use]
	pub const fn change_cell_at(value: i8, offset: i32) -> Self {
		Self::ChangeCell(value, NonZeroI32::new(offset))
	}

	#[must_use]
	pub const fn move_pointer(offset: i32) -> Self {
		Self::MovePointer(offset)
	}

	#[must_use]
	pub const fn set_cell(value: u8) -> Self {
		Self::set_cell_at(value, 0)
	}

	#[must_use]
	pub const fn set_cell_at(value: u8, offset: i32) -> Self {
		Self::SetCell(value, NonZeroI32::new(offset))
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
	pub const fn scale_and_move_value(value: u8, offset: i32) -> Self {
		Self::ScaleAndMoveValue(value, offset)
	}

	#[must_use]
	pub const fn offset(&self) -> Option<i32> {
		match self {
			Self::ChangeCell(.., offset) | Self::SetCell(.., offset) => match offset {
				None => Some(0),
				Some(i) => Some(i.get()),
			},
			_ => None,
		}
	}

	#[must_use]
	pub const fn child_ops(&self) -> Option<&Vec<Self>> {
		match self {
			Self::DynamicLoop(ops) => Some(ops),
			_ => None,
		}
	}

	pub const fn child_ops_mut(&mut self) -> Option<&mut Vec<Self>> {
		match self {
			Self::DynamicLoop(ops) => Some(ops),
			_ => None,
		}
	}

	#[must_use]
	pub fn dynamic_loop(instrs: impl IntoIterator<Item = Self>) -> Self {
		Self::DynamicLoop(instrs.into_iter().collect())
	}
}

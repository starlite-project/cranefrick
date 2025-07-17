#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod compiler;

use cranefrick_hlir::BrainHlir;
use serde::{Deserialize, Serialize};

pub use self::compiler::*;

/// Mid-level intermediate representation. Not 1 to 1 for it's brainfuck equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainMlir {
	ChangeCell(i8),
	MovePtr(i64),
	SetCell(i8),
	GetInput,
	PutOutput,
	StartLoop,
	EndLoop,
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
			BrainHlir::StartLoop => Self::StartLoop,
			BrainHlir::EndLoop => Self::EndLoop,
		}
	}
}

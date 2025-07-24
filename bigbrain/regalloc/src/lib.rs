#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[macro_use]
mod index;
mod registers;

pub use self::{
	index::{Block, Instruction, InstructionRange, InstructionRangeIter},
	registers::*,
};

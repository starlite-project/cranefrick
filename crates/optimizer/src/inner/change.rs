use alloc::vec::Vec;

use frick_ir::BrainIr;
use frick_utils::{InsertOrPush as _, IntoIteratorExt as _};
use tracing::trace;

#[derive(Debug, Clone)]
pub enum Change {
	Remove,
	RemoveOffset(isize),
	Swap(Vec<BrainIr>),
	Replace(BrainIr),
}

impl Change {
	pub const fn remove() -> Self {
		Self::Remove
	}

	pub const fn remove_offset(offset: isize) -> Self {
		Self::RemoveOffset(offset)
	}

	pub fn swap(instrs: impl IntoIterator<Item = BrainIr>) -> Self {
		Self::Swap(instrs.collect_to())
	}

	pub const fn replace(instr: BrainIr) -> Self {
		Self::Replace(instr)
	}

	pub(super) fn apply<const N: usize>(self, ops: &mut Vec<BrainIr>, i: usize) {
		match self {
			Self::Remove => {
				let removed = ops.drain(i..(i + N)).collect::<Vec<_>>();

				trace!("removing instructions {removed:?}");
			}
			Self::RemoveOffset(offset) => {
				let removed = ops.remove(i.wrapping_add_signed(offset));

				trace!("removing instruction {removed:?} at offset {offset}");
			}
			Self::Swap(instrs) => {
				let mut replaced = Vec::with_capacity(N);

				for _ in 0..N {
					replaced.push(ops.remove(i));
				}

				trace!("swapping instructions {replaced:?} with {instrs:?}");

				instrs
					.into_iter()
					.rev()
					.for_each(|instr| ops.insert_or_push(i, instr));
			}
			Self::Replace(instr) => {
				let mut replaced = Vec::with_capacity(N);

				for _ in 0..N {
					replaced.push(ops.remove(i));
				}

				trace!("replacing instructions {replaced:?} with {instr:?}");

				ops.insert_or_push(i, instr);
			}
		}
	}
}

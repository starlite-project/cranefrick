use cranefrick_utils::{InsertOrPush as _, IntoIteratorExt as _};
use tracing::trace;

use crate::BrainIr;

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

	pub const fn replace(i: BrainIr) -> Self {
		Self::Replace(i)
	}

	#[tracing::instrument(skip(self, ops, size))]
	pub fn apply(self, ops: &mut Vec<BrainIr>, i: usize, size: usize) {
		match self {
			Self::Remove => {
				let removed = ops.drain(i..(i + size)).collect::<Vec<_>>();

				trace!("removing instructions {removed:?}");
			}
			Self::RemoveOffset(offset) => {
				let removed = ops.remove(i.wrapping_add_signed(offset));

				trace!("removing instruction {removed:?}");
			}
			Self::Swap(instrs) => {
				let mut replaced = Vec::with_capacity(size);

				for _ in 0..size {
					replaced.push(ops.remove(i));
				}

				trace!("swapping instructions {replaced:?} with {instrs:?}");

				for instr in instrs.into_iter().rev() {
					ops.insert_or_push(i, instr);
				}
			}
			Self::Replace(instr) => {
				let mut replaced = Vec::with_capacity(size);

				for _ in 0..size {
					replaced.push(ops.remove(i));
				}

				trace!("replacing instructions {replaced:?} with {instr:?}");

				ops.insert_or_push(i, instr);
			}
		}
	}
}

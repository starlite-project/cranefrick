use frick_ir::BrainIr;
use frick_utils::{InsertOrPush as _, IntoIteratorExt as _};
use smallvec::SmallVec;
use tracing::trace;

#[derive(Debug, Clone)]
pub enum Change {
	Remove,
	RemoveOffset(isize),
	Swap(SmallVec<[BrainIr; 4]>),
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

	pub fn apply(self, ops: &mut Vec<BrainIr>, i: usize, size: usize) {
		match self {
			Self::Remove => {
				let removed = ops.drain(i..(i + size)).collect::<SmallVec<[BrainIr; 4]>>();

				trace!("removing instructions {removed:?}");
			}
			Self::RemoveOffset(offset) => {
				let removed = ops.remove(i.wrapping_add_signed(offset));

				trace!("removing instruction {removed:?}");
			}
			Self::Swap(instrs) => {
				let mut replaced: SmallVec<[BrainIr; 4]> = SmallVec::with_capacity(size);

				for _ in 0..size {
					replaced.push(ops.remove(i));
				}

				trace!("swapping instructions {replaced:?} with {instrs:?}");

				for instr in instrs.into_iter().rev() {
					ops.insert_or_push(i, instr);
				}
			}
			Self::Replace(instr) => {
				let mut replaced: SmallVec<[BrainIr; 4]> = SmallVec::with_capacity(size);

				for _ in 0..size {
					replaced.push(ops.remove(i));
				}

				trace!("replacing instructions {replaced:?} with {instr:?}");

				ops.insert_or_push(i, instr);
			}
		}
	}
}

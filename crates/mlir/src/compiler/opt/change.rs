use alloc::vec::Vec;

use cranefrick_utils::InsertOrPush as _;

use crate::BrainMlir;

#[derive(Debug, Clone)]
pub enum Change {
	Remove,
	RemoveOffset(isize),
	Swap(Vec<BrainMlir>),
	Replace(BrainMlir),
}

impl Change {
	pub const fn remove() -> Self {
		Self::Remove
	}

	pub const fn remove_offset(offset: isize) -> Self {
		Self::RemoveOffset(offset)
	}

	pub fn swap(instrs: impl IntoIterator<Item = BrainMlir>) -> Self {
		Self::Swap(instrs.into_iter().collect())
	}

	pub const fn replace(i: BrainMlir) -> Self {
		Self::Replace(i)
	}

	pub fn apply(self, ops: &mut Vec<BrainMlir>, i: usize, size: usize) {
		match self {
			Self::Remove => {
				ops.drain(i..(i + size));
			}
			Self::RemoveOffset(offset) => {
				ops.remove(i.wrapping_add_signed(offset));
			}
			Self::Swap(instrs) => {
				for _ in 0..size {
					ops.remove(i);
				}

				for instr in instrs.into_iter().rev() {
					ops.insert_or_push(i, instr);
				}
			}
			Self::Replace(instr) => {
				for _ in 0..size {
					ops.remove(i);
				}

				ops.insert_or_push(i, instr);
			}
		}
	}
}

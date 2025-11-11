use alloc::vec::Vec;

use frick_operations::{BrainOperation, BrainOperationType};
use frick_utils::{InsertOrPush as _, IntoIteratorExt as _};
use tracing::trace;

#[derive(Debug, Clone)]
pub enum Change {
	Remove,
	RemoveOffset(isize),
	Swap(Vec<BrainOperation>),
	Replace(BrainOperationType),
}

impl Change {
	pub const fn remove() -> Self {
		Self::Remove
	}

	pub const fn remove_offset(offset: isize) -> Self {
		Self::RemoveOffset(offset)
	}

	pub fn swap(ops: impl IntoIterator<Item = BrainOperation>) -> Self {
		Self::Swap(ops.collect_to())
	}

	pub const fn replace(instr: BrainOperationType) -> Self {
		Self::Replace(instr)
	}

	pub(super) fn apply<const N: usize>(self, ops: &mut Vec<BrainOperation>, i: usize) {
		match self {
			Self::Remove => {
				let removed = ops.drain(i..(i + N)).collect::<Vec<_>>();

				trace!("removing instructions {removed:?}");
			}
			Self::RemoveOffset(offset) => {
				let removed = ops.remove(i.wrapping_add_signed(offset));

				trace!("removing instruction {removed:?} at offset {offset}");
			}
			Self::Swap(new_ops) => {
				let mut replaced = Vec::with_capacity(N);

				for _ in 0..N {
					replaced.push(ops.remove(i));
				}

				trace!("swapping instructions {replaced:?} with {new_ops:?}");

				new_ops
					.into_iter()
					.rev()
					.for_each(|op| ops.insert_or_push(i, op));
			}
			Self::Replace(op_ty) => {
				let mut replaced = Vec::with_capacity(N);

				for _ in 0..N {
					replaced.push(ops.remove(i));
				}

				let span_start = replaced.iter().map(|x| x.span().start).min().unwrap();
				let span_end = replaced.iter().map(|x| x.span().end).max().unwrap();

				let new_op = BrainOperation::new(op_ty, span_start..span_end);

				trace!("replacing instructions {replaced:?} with {new_op:?}");

				ops.insert_or_push(i, new_op);
			}
		}
	}
}

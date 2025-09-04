mod impls;

use std::{marker::PhantomData, num::NonZero};

use frick_assembler::{AssembledModule, TAPE_SIZE};
use frick_ir::BrainIr;

use super::RustInterpreterError;

pub struct RustInterpreterModule<'ctx> {
	ops: Vec<BrainIr>,
	marker: PhantomData<&'ctx ()>,
}

impl RustInterpreterModule<'_> {
	pub(crate) const fn new(ops: Vec<BrainIr>) -> Self {
		Self {
			ops,
			marker: PhantomData,
		}
	}

	fn execute_op(op: &BrainIr, memory: &mut [u8; TAPE_SIZE], ptr: &mut usize) {
		match op {
			BrainIr::MovePointer(offset) => Self::move_pointer(*offset, ptr),
			BrainIr::SetCell(value, offset) => {
				Self::set_cell(*value, offset.map_or(0, NonZero::get), memory, *ptr);
			}
			BrainIr::ChangeCell(value, offset) => {
				Self::change_cell(*value, offset.map_or(0, NonZero::get), memory, *ptr);
			}
			BrainIr::SubCell(offset) => Self::sub_cell(*offset, memory, *ptr),
			BrainIr::OutputCell {
				value_offset: value,
				offset,
			} => Self::output_current_cell(
				value.map_or(0, NonZero::get),
				offset.map_or(0, NonZero::get),
				memory,
				*ptr,
			),
			BrainIr::OutputChar(c) => Self::output_char(*c),
			BrainIr::OutputChars(c) => Self::output_chars(c),
			BrainIr::InputIntoCell => Self::input_into_cell(memory, *ptr),
			BrainIr::DynamicLoop(ops) => Self::dynamic_loop(ops, memory, ptr),
			BrainIr::IfNotZero(ops) => Self::if_not_zero(ops, memory, ptr),
			BrainIr::FindZero(offset) => Self::find_zero(*offset, memory, ptr),
			BrainIr::MoveValueTo(factor, offset) => {
				Self::move_value_to(*factor, *offset, memory, *ptr);
			}
			BrainIr::TakeValueTo(factor, offset) => {
				Self::take_value_to(*factor, *offset, memory, ptr);
			}
			BrainIr::FetchValueFrom(factor, offset) => {
				Self::fetch_value_from(*factor, *offset, memory, *ptr);
			}
			BrainIr::ReplaceValueFrom(factor, offset) => {
				Self::replace_value_from(*factor, *offset, memory, *ptr);
			}
			BrainIr::ScaleValue(factor) => Self::scale_value(*factor, memory, *ptr),
			_ => unimplemented!("op {op:?}"),
		}
	}
}

impl AssembledModule for RustInterpreterModule<'_> {
	type Error = RustInterpreterError;

	fn execute(&self) -> Result<(), Self::Error> {
		let mut memory = [0u8; 30_000];
		let mut ptr = 0;

		for op in &self.ops {
			Self::execute_op(op, &mut memory, &mut ptr);
		}

		Ok(())
	}
}

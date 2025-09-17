mod impls;

use std::marker::PhantomData;

use frick_assembler::{AssembledModule, TAPE_SIZE};
use frick_ir::BrainIr;
use frick_utils::GetOrZero as _;

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
				Self::set_cell(*value, offset.get_or_zero(), memory, *ptr);
			}
			BrainIr::ChangeCell(value, offset) => {
				Self::change_cell(*value, offset.get_or_zero(), memory, *ptr);
			}
			BrainIr::SubCell(offset) => Self::sub_cell(*offset, memory, *ptr),
			BrainIr::Output(options) => Self::output(options, memory, *ptr),
			BrainIr::InputIntoCell => Self::input_into_cell(memory, *ptr),
			BrainIr::DynamicLoop(ops) => Self::dynamic_loop(ops, memory, ptr),
			BrainIr::IfNotZero(ops) => Self::if_not_zero(ops, memory, ptr),
			BrainIr::FindZero(offset) => Self::find_zero(*offset, memory, ptr),
			BrainIr::MoveValueTo(options) => {
				Self::move_value_to(*options, memory, *ptr);
			}
			BrainIr::TakeValueTo(options) => {
				Self::take_value_to(*options, memory, ptr);
			}
			BrainIr::FetchValueFrom(options) => {
				Self::fetch_value_from(*options, memory, *ptr);
			}
			BrainIr::ReplaceValueFrom(options) => {
				Self::replace_value_from(*options, memory, *ptr);
			}
			BrainIr::ScaleValue(factor) => Self::scale_value(*factor, memory, *ptr),
			BrainIr::SetRange { value, range } => {
				Self::set_range(*value, range.clone(), memory, *ptr);
			}
			_ => unimplemented!("op {op:?}"),
		}
	}
}

impl AssembledModule for RustInterpreterModule<'_> {
	type Error = RustInterpreterError;

	fn execute(&self) -> Result<(), Self::Error> {
		let mut memory = [0u8; TAPE_SIZE];
		let mut ptr = 0;

		for op in &self.ops {
			Self::execute_op(op, &mut memory, &mut ptr);
		}

		Ok(())
	}
}

use frick_spec::{POINTER_SIZE, TAPE_SIZE};
use frick_utils::Convert as _;
use inkwell::values::{BasicMetadataValueEnum, InstructionOpcode, InstructionValue};

use super::InnerAssembler;
use crate::{AssemblyError, ContextExt, ContextGetter as _};

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn add_loop_metadata_to_br(
		&self,
		instr: InstructionValue<'ctx>,
	) -> Result<(), AssemblyError> {
		if !matches!(instr.get_opcode(), InstructionOpcode::Br) {
			return Ok(());
		}

		let context = self.context();

		let mustprogress_metadata_node = {
			let key = context.metadata_string("llvm.loop.mustprogress");

			context.metadata_node(&[key.convert::<BasicMetadataValueEnum<'ctx>>()])
		};

		let loop_metadata_node =
			context.self_referential_distinct_metadata_node(&[mustprogress_metadata_node
				.convert::<BasicMetadataValueEnum<'ctx>>(
			)]);
		let loop_metadata_id = context.get_kind_id("llvm.loop");

		instr.set_metadata(loop_metadata_node, loop_metadata_id)?;

		Ok(())
	}

	pub(super) fn add_range_metadata_to_pointer_load(
		&self,
		instr: InstructionValue<'ctx>,
	) -> Result<(), AssemblyError> {
		if !matches!(instr.get_opcode(), InstructionOpcode::Load) {
			return Ok(());
		}

		let context = self.context();

		let ptr_int_type = context.custom_width_int_type(POINTER_SIZE as u32);

		let ptr_int_range_min = ptr_int_type.const_zero();
		let ptr_int_range_max = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		let range_metadata_node = context.metadata_node(&[
			ptr_int_range_min.convert::<BasicMetadataValueEnum<'ctx>>(),
			ptr_int_range_max.convert::<BasicMetadataValueEnum<'ctx>>(),
		]);
		let range_metadata_id = context.get_kind_id("range");

		instr.set_metadata(range_metadata_node, range_metadata_id)?;

		Ok(())
	}
}

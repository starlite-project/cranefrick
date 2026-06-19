use std::num::NonZero;

use frick_spec::{POINTER_SIZE, TAPE_SIZE};
use frick_utils::Convert as _;
use inkwell::values::{BasicMetadataValueEnum, InstructionOpcode, InstructionValue};

use super::InnerAssembler;
use crate::{AssemblyError, IntoContext as _};

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn add_range_metadata_to_pointer_load(
		&self,
		instr: InstructionValue<'ctx>,
	) -> Result<(), AssemblyError> {
		if !matches!(instr.get_opcode(), InstructionOpcode::Load) {
			return Ok(());
		}

		let context = self.into_context();

		let ptr_int_type = context
			.custom_width_int_type(unsafe { NonZero::new_unchecked(POINTER_SIZE as u32) })?;

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

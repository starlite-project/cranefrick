use frick_utils::Convert as _;
use inkwell::values::{BasicMetadataValueEnum, InstructionOpcode, InstructionValue};

use super::InnerAssembler;
use crate::{AssemblyError, ContextExt, ContextGetter as _};

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn add_nontemporal_metadata_to_mem(
		&self,
		instr: InstructionValue<'ctx>,
	) -> Result<(), AssemblyError> {
		if !matches!(
			instr.get_opcode(),
			InstructionOpcode::Load | InstructionOpcode::Store
		) {
			return Ok(());
		}

		let context = self.context();

		let i32_type = context.i32_type();

		let i32_one = i32_type.const_int(1, false);

		let nontemporal_metadata_node =
			context.metadata_node(&[i32_one.convert::<BasicMetadataValueEnum<'ctx>>()]);
		let nontemporal_metadata_id = context.get_kind_id("nontemporal");

		instr.set_metadata(nontemporal_metadata_node, nontemporal_metadata_id)?;

		Ok(())
	}

	pub(super) fn add_loop_metadata_to_br(
		&self,
		instr: InstructionValue<'ctx>,
	) -> Result<(), AssemblyError> {
		if !matches!(instr.get_opcode(), InstructionOpcode::Br) {
			return Ok(());
		}

		let context = self.context();

		let loop_metadata_node = context.self_referential_distinct_metadata_node();
		let loop_metadata_id = context.get_kind_id("llvm.loop");

		instr.set_metadata(loop_metadata_node, loop_metadata_id)?;

		Ok(())
	}
}

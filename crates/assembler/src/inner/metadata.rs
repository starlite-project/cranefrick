use inkwell::values::{InstructionOpcode, InstructionValue};

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

		let loop_metadata_node = context.self_referential_distinct_metadata_node(&[]);
		let loop_metadata_id = context.get_kind_id("llvm.loop");

		instr.set_metadata(loop_metadata_node, loop_metadata_id)?;

		Ok(())
	}
}

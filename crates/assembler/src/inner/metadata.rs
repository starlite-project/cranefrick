use inkwell::values::{InstructionOpcode, InstructionValue};

use super::InnerAssembler;
use crate::{AssemblyError, ContextGetter as _};

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn add_noalias_metadata_to_mem(
		&self,
		instr: InstructionValue<'ctx>,
	) -> Result<(), AssemblyError> {
		assert!(matches!(
			instr.get_opcode(),
			InstructionOpcode::Load | InstructionOpcode::Store
		));

		let context = self.context();

		let noalias_metadata_node = context.metadata_node(&[]);
		let noalias_metadata_id = context.get_kind_id("noalias");

		instr.set_metadata(noalias_metadata_node, noalias_metadata_id)?;

		Ok(())
	}
}

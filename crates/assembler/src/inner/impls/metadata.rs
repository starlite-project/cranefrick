use inkwell::values::{InstructionOpcode, InstructionValue};

use crate::{AssemblyError, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn add_noundef_metadata_to_load(
		&self,
		instr: InstructionValue<'ctx>,
	) -> Result<(), AssemblyError> {
		assert!(matches!(instr.get_opcode(), InstructionOpcode::Load));

		let context = self.context();

		let noundef_metadata_node = context.metadata_node(&[]);
		let noundef_metadata_id = context.get_kind_id("noundef");

		instr.set_metadata(noundef_metadata_node, noundef_metadata_id)?;

		Ok(())
	}
}

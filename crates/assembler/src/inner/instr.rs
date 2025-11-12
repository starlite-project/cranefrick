use frick_utils::InsertOrPush as _;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

use super::{AssemblyError, InnerAssembler};
use crate::ContextGetter as _;

impl<'ctx> InnerAssembler<'ctx> {
	pub(super) fn load_pointer(&self) -> Result<(), AssemblyError> {
		self.loaded_pointer.borrow_mut().replace(
			self.builder
				.build_load(self.pointers.pointer_ty, self.pointers.pointer, "\0")?
				.into_int_value(),
		);

		Ok(())
	}

	pub(super) fn load_cell_into_register(&self, slot: usize) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let gep = self.index_tape()?;

		let cell_value = self.builder.build_load(cell_type, gep, "\0")?;

		self.set_value_at(slot, cell_value)
	}

	pub(super) fn change_register_by_immediate(
		&self,
		slot: usize,
		value: i8,
	) -> Result<(), AssemblyError> {
		let cell_type = self.context().i8_type();

		let register_value = self.value_at(slot)?.into_int_value();

		let new_value = self.builder.build_int_add(
			register_value,
			cell_type.const_int(value as u64, false),
			"\0",
		)?;

		self.set_value_at(slot, new_value)
	}

	pub(super) fn store_register_into_cell(&self, slot: usize) -> Result<(), AssemblyError> {
		let gep = self.index_tape()?;

		let cell_value = self.value_at(slot)?;

		self.builder.build_store(gep, cell_value)?;

		Ok(())
	}

	fn index_tape(&self) -> Result<PointerValue<'ctx>, AssemblyError> {
		let cell_type = self.context().i8_type();

		let pointer_value = self
			.loaded_pointer
			.borrow()
			.ok_or(AssemblyError::PointerNotLoaded)?;

		Ok(unsafe {
			self.builder.build_in_bounds_gep(
				cell_type,
				self.pointers.tape,
				&[pointer_value],
				"\0",
			)?
		})
	}

	fn value_at(&self, slot: usize) -> Result<BasicValueEnum<'ctx>, AssemblyError> {
		self.registers
			.borrow()
			.get(slot)
			.copied()
			.ok_or_else(|| AssemblyError::NoValueInRegister(slot))
	}

	fn set_value_at(&self, slot: usize, value: impl BasicValue<'ctx>) -> Result<(), AssemblyError> {
		self.registers
			.borrow_mut()
			.insert_or_push(slot, value.as_basic_value_enum());

		Ok(())
	}
}

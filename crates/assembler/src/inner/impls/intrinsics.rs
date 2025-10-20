use frick_spec::TAPE_SIZE;
use inkwell::{
	builder::Builder,
	values::{BasicMetadataValueEnum, FunctionValue, IntValue, PointerValue},
};

use crate::{AssemblyError, ContextGetter, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip(self))]
	pub(super) fn start_lifetime(
		&self,
		alloc_len: IntValue<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<impl Drop, AssemblyError> {
		self.builder.build_call(
			self.functions.lifetime.start,
			&[alloc_len.into(), pointer.into()],
			"\0",
		)?;

		Ok(LifetimeEnd {
			builder: &self.builder,
			end: self.functions.lifetime.end,
			pointer,
			alloc_len,
			invariant_pointer: None,
		})
	}

	pub(super) fn start_output_lifetime(&self, len: u64) -> Result<impl Drop, AssemblyError> {
		let len_value = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(len, false)
		};

		self.start_lifetime(len_value, self.pointers.output)
	}

	pub(super) fn start_tape_invariant(&self) -> Result<impl Drop, AssemblyError> {
		let tape_len = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(TAPE_SIZE as u64, false)
		};

		self.start_invariant(tape_len, self.pointers.tape)
	}

	pub(super) fn start_invariant(
		&self,
		alloc_len: IntValue<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<impl Drop, AssemblyError> {
		let invariant_pointer = self
			.builder
			.build_call(
				self.functions.invariant.start,
				&[alloc_len.into(), pointer.into()],
				"\0",
			)?
			.try_as_basic_value()
			.unwrap_left()
			.into_pointer_value();

		Ok(LifetimeEnd {
			builder: &self.builder,
			end: self.functions.invariant.end,
			pointer,
			alloc_len,
			invariant_pointer: Some(invariant_pointer),
		})
	}
}

struct LifetimeEnd<'builder, 'ctx> {
	builder: &'builder Builder<'ctx>,
	end: FunctionValue<'ctx>,
	pointer: PointerValue<'ctx>,
	invariant_pointer: Option<PointerValue<'ctx>>,
	alloc_len: IntValue<'ctx>,
}

impl<'ctx> Drop for LifetimeEnd<'_, 'ctx> {
	fn drop(&mut self) {
		let params: &[BasicMetadataValueEnum<'ctx>] =
			if let Some(invariant_pointer) = self.invariant_pointer.take() {
				&[
					BasicMetadataValueEnum::from(invariant_pointer),
					BasicMetadataValueEnum::from(self.alloc_len),
					BasicMetadataValueEnum::from(self.pointer),
				]
			} else {
				&[
					BasicMetadataValueEnum::from(self.alloc_len),
					BasicMetadataValueEnum::from(self.pointer),
				]
			};

		self.builder.build_call(self.end, params, "\0").unwrap();
	}
}

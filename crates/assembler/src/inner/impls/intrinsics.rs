use inkwell::{
	builder::Builder,
	values::{BasicMetadataValueEnum, FunctionValue, IntValue, PointerValue},
};

use crate::{AssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
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

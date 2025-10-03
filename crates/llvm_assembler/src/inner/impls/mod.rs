mod cell;
mod io;
mod loops;
mod mem;
mod pointer;
mod value;

use inkwell::{
	IntPredicate,
	builder::Builder,
	values::{FunctionValue, IntValue, PointerValue},
};

use super::{InnerAssembler, LlvmAssemblyError};
use crate::ContextExt as _;

impl<'ctx> InnerAssembler<'ctx> {
	fn start_lifetime(
		&self,
		alloc_len: IntValue<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<impl Drop, LlvmAssemblyError> {
		struct LifetimeEnd<'builder, 'ctx> {
			builder: &'builder Builder<'ctx>,
			end: FunctionValue<'ctx>,
			pointer: PointerValue<'ctx>,
			alloc_len: IntValue<'ctx>,
		}

		impl Drop for LifetimeEnd<'_, '_> {
			fn drop(&mut self) {
				self.builder
					.build_call(self.end, &[self.alloc_len.into(), self.pointer.into()], "")
					.unwrap();
			}
		}

		self.builder.build_call(
			self.functions.lifetime.start,
			&[alloc_len.into(), pointer.into()],
			"",
		)?;

		let lifetime_end = LifetimeEnd {
			builder: &self.builder,
			end: self.functions.lifetime.end,
			pointer,
			alloc_len,
		};

		Ok(lifetime_end)
	}

	pub fn write_puts(&self) -> Result<(), LlvmAssemblyError> {
		let context = self.context();
		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let ptr_type = context.default_ptr_type();

		self.builder.unset_current_debug_location();

		let entry_block = context.append_basic_block(self.functions.puts, "entry");
		let try_block = context.append_basic_block(self.functions.puts, "try");
		let continue_block = context.append_basic_block(self.functions.puts, "continue");
		let catch_block = context.append_basic_block(self.functions.puts, "catch");
		let exit_block = context.append_basic_block(self.functions.puts, "exit");

		self.builder.position_at_end(entry_block);

		let (pointer_param, string_len) = {
			let params = self.functions.puts.get_params();

			(params[0].into_pointer_value(), params[1].into_int_value())
		};

		let null_pointer = ptr_type.const_null();

		let is_ptr_null =
			self.builder
				.build_int_compare(IntPredicate::NE, pointer_param, null_pointer, "")?;

		self.builder
			.build_direct_call(self.functions.assume, &[is_ptr_null.into()], "")?;

		let end_of_string = unsafe {
			self.builder
				.build_in_bounds_gep(i8_type, pointer_param, &[string_len], "")?
		};

		let i64_zero = i64_type.const_zero();

		let is_string_len_zero =
			self.builder
				.build_int_compare(IntPredicate::EQ, string_len, i64_zero, "")?;

		self.builder
			.build_conditional_branch(is_string_len_zero, exit_block, try_block)?;

		self.builder.position_at_end(try_block);

		let body_block_phi = self.builder.build_phi(ptr_type, "")?;

		let i64_one = i64_type.const_int(1, false);

		let next_index_gep = unsafe {
			self.builder.build_in_bounds_gep(
				i8_type,
				body_block_phi.as_basic_value().into_pointer_value(),
				&[i64_one],
				"",
			)?
		};

		body_block_phi.add_incoming(&[
			(&pointer_param, entry_block),
			(&next_index_gep, continue_block),
		]);

		let actual_value = self
			.builder
			.build_load(
				i8_type,
				body_block_phi.as_basic_value().into_pointer_value(),
				"",
			)?
			.into_int_value();

		let extended_character = self
			.builder
			.build_int_z_extend(actual_value, i32_type, "")?;

		let putchar_call = self.builder.build_direct_invoke(
			self.functions.putchar,
			&[extended_character.into()],
			continue_block,
			catch_block,
			"",
		)?;

		let putchar_value = putchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.builder.position_at_end(continue_block);

		let check_if_at_end =
			self.builder
				.build_int_compare(IntPredicate::EQ, next_index_gep, end_of_string, "")?;

		self.builder
			.build_conditional_branch(check_if_at_end, exit_block, try_block)?;

		self.builder.position_at_end(exit_block);

		let end_value = self.builder.build_phi(i32_type, "")?;

		end_value.add_incoming(&[
			(&i32_type.const_zero(), entry_block),
			(&putchar_value, continue_block),
		]);

		self.builder
			.build_return(Some(&end_value.as_basic_value()))?;

		self.builder.position_at_end(catch_block);

		let exception_type = context.struct_type(&[ptr_type.into(), i32_type.into()], false);

		let out = self.builder.build_landing_pad(
			exception_type,
			self.functions.eh_personality,
			&[],
			true,
			"",
		)?;

		self.builder.build_resume(out)?;

		Ok(())
	}
}

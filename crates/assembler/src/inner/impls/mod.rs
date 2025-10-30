mod cell;
mod intrinsics;
mod io;
mod loops;
mod mem;
mod pointer;
mod value;

use frick_utils::Convert as _;
use inkwell::{IntPredicate, types::BasicTypeEnum, values::BasicValueEnum};

use crate::{AssemblyError, ContextExt as _, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip(self))]
	pub fn write_puts(&self) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let ptr_type = context.default_ptr_type();

		self.builder.unset_current_debug_location();

		let entry_block = context.append_basic_block(self.functions.puts, "entry\0");
		let try_block = context.append_basic_block(self.functions.puts, "try\0");
		let continue_block = context.append_basic_block(self.functions.puts, "continue\0");
		let catch_block = context.append_basic_block(self.functions.puts, "catch\0");
		let exit_block = context.append_basic_block(self.functions.puts, "exit\0");

		self.builder.position_at_end(entry_block);

		let (ptr_param, string_len) = {
			let params = self.functions.puts.get_params();

			(params[0].into_pointer_value(), params[1].into_int_value())
		};

		let i64_zero = i64_type.const_zero();

		let is_str_empty =
			self.builder
				.build_int_compare(IntPredicate::EQ, i64_zero, string_len, "\0")?;

		self.builder
			.build_conditional_branch(is_str_empty, exit_block, try_block)?;

		self.builder.position_at_end(try_block);

		let idx_phi_value = self.builder.build_phi(i64_type, "\0")?;

		let char_ptr = self.gep(
			i8_type,
			ptr_param,
			idx_phi_value.as_basic_value().into_int_value(),
		)?;

		let raw_char = self.load_from(i8_type, char_ptr)?;

		let extended_char = self
			.builder
			.build_int_z_extend_or_bit_cast(raw_char, i32_type, "\0")?;

		if let Some(extended_char_instr) = extended_char.as_instruction() {
			extended_char_instr.set_non_negative_flag(true);
		}

		self.builder.build_direct_invoke(
			self.functions.putchar,
			&[extended_char.convert::<BasicValueEnum<'ctx>>()],
			continue_block,
			catch_block,
			"\0",
		)?;

		self.builder.position_at_end(continue_block);

		let i64_one = i64_type.const_int(1, false);

		let next_index = self.builder.build_int_add(
			idx_phi_value.as_basic_value().into_int_value(),
			i64_one,
			"\0",
		)?;

		idx_phi_value.add_incoming(&[(&i64_zero, entry_block), (&next_index, continue_block)]);

		let is_done =
			self.builder
				.build_int_compare(IntPredicate::EQ, next_index, string_len, "\0")?;

		self.builder
			.build_conditional_branch(is_done, exit_block, try_block)?;

		self.builder.position_at_end(catch_block);
		let exception_type = context.struct_type(
			&[
				ptr_type.convert::<BasicTypeEnum<'ctx>>(),
				i32_type.convert::<BasicTypeEnum<'ctx>>(),
			],
			false,
		);

		let exception = self.builder.build_landing_pad(
			exception_type,
			self.functions.eh_personality,
			&[],
			true,
			"\0",
		)?;

		self.builder.build_resume(exception)?;

		self.builder.position_at_end(exit_block);
		self.builder.build_return(None)?;

		Ok(())
	}
}

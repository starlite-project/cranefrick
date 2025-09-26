mod cell;
mod io;
mod loops;
mod mem;
mod pointer;
mod value;

use frick_assembler::TAPE_SIZE;
use inkwell::{IntPredicate, debug_info::AsDIScope as _};

use super::{InnerAssembler, LlvmAssemblyError};
use crate::ContextExt as _;

impl<'ctx> InnerAssembler<'ctx> {
	pub fn write_puts(&self) -> Result<(), LlvmAssemblyError> {
		let context = self.context();
		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let ptr_type = context.default_ptr_type();

		let debug_location = self.debug_builder.create_debug_location(
			self.context(),
			0,
			0,
			self.functions
				.puts
				.get_subprogram()
				.unwrap()
				.as_debug_info_scope(),
			None,
		);

		self.builder.set_current_debug_location(debug_location);

		let entry_block = context.append_basic_block(self.functions.puts, "entry");
		let body_block = context.append_basic_block(self.functions.puts, "body");
		let exit_block = context.append_basic_block(self.functions.puts, "exit");

		self.builder.position_at_end(entry_block);

		let pointer_param = self
			.functions
			.puts
			.get_first_param()
			.unwrap()
			.into_pointer_value();

		let string_len = self
			.builder
			.build_call(self.functions.strlen, &[pointer_param.into()], "string_len")?
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		let end_of_string = unsafe {
			self.builder.build_in_bounds_gep(
				i8_type,
				pointer_param,
				&[string_len],
				"end_of_string_gep",
			)?
		};

		let i64_zero = i64_type.const_zero();

		let is_string_len_zero = self.builder.build_int_compare(
			IntPredicate::EQ,
			string_len,
			i64_zero,
			"is_string_len_zero",
		)?;

		self.builder
			.build_conditional_branch(is_string_len_zero, exit_block, body_block)?;

		self.builder.position_at_end(body_block);

		let body_block_phi = self.builder.build_phi(ptr_type, "body_phi")?;

		body_block_phi.add_incoming(&[(&pointer_param, entry_block)]);

		let i64_one = i64_type.const_int(1, false);

		let next_index_gep = unsafe {
			self.builder.build_in_bounds_gep(
				i8_type,
				body_block_phi.as_basic_value().into_pointer_value(),
				&[i64_one],
				"next_string_index",
			)?
		};

		body_block_phi.add_incoming(&[(&next_index_gep, body_block)]);

		let actual_value = self
			.builder
			.build_load(
				i8_type,
				body_block_phi.as_basic_value().into_pointer_value(),
				"loaded_char",
			)?
			.into_int_value();

		let extended_character =
			self.builder
				.build_int_z_extend(actual_value, i32_type, "extend_for_putchar")?;

		let putchar_call = self.builder.build_call(
			self.functions.putchar,
			&[extended_character.into()],
			"putchar_call",
		)?;

		let putchar_value = putchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		let check_if_at_end = self.builder.build_int_compare(
			IntPredicate::EQ,
			next_index_gep,
			end_of_string,
			"check_if_at_end_of_string",
		)?;

		self.builder
			.build_conditional_branch(check_if_at_end, exit_block, body_block)?;

		self.builder.position_at_end(exit_block);

		let end_value = self.builder.build_phi(i32_type, "exit_value")?;

		end_value.add_incoming(&[
			(&i32_type.const_zero(), entry_block),
			(&putchar_value, body_block),
		]);

		self.builder
			.build_return(Some(&end_value.as_basic_value()))?;

		Ok(())
	}

	pub fn write_find_zero(&self) -> Result<(), LlvmAssemblyError> {
		let context = self.context();
		let i8_type = context.i8_type();
		let ptr_int_type = self.ptr_int_type;

		let entry_block = context.append_basic_block(self.functions.find_zero, "entry");
		let body_block = context.append_basic_block(self.functions.find_zero, "body");
		let exit_block = context.append_basic_block(self.functions.find_zero, "exit");

		self.builder.position_at_end(entry_block);

		let (tape_pointer, current_pointer_value, offset_value) = {
			let params = self.functions.find_zero.get_params();

			(params[0].into_pointer_value(), params[1].into_int_value(), params[2].into_int_value())
		};

		let header_phi_value = self.builder.build_phi(ptr_int_type, "find_zero_value")?;

		header_phi_value.add_incoming(&[(&current_pointer_value, entry_block)]);

		let gep = unsafe {
			self.builder.build_gep(
				i8_type,
				tape_pointer,
				&[header_phi_value.as_basic_value().into_int_value()],
				"find_zero_index_into_tape",
			)?
		};

        let value = self.builder.build_load(i8_type, gep, "find_zero_load")?.into_int_value();

        let i8_zero = i8_type.const_zero();

        let cmp  =self.builder.build_int_compare(IntPredicate::NE, value, i8_zero, "find_zero_cmp")?;

        self.builder.build_conditional_branch(cmp, body_block, exit_block)?;

        self.builder.position_at_end(body_block);

        let new_pointer_value = self.builder.build_int_add(header_phi_value.as_basic_value().into_int_value(), offset_value, "find_zero_add")?;

        let wrapped_pointer_value = {
            let tape_len = ptr_int_type.const_int(TAPE_SIZE as u64 - 1, false);

            self.builder.build_and(new_pointer_value, tape_len, "find_zero_wrap_pointer")?
        };

        self.builder.build_unconditional_branch(entry_block)?;

        header_phi_value.add_incoming(&[(&wrapped_pointer_value ,body_block)]);

        self.builder.position_at_end(exit_block);

        self.builder.build_return(Some(&header_phi_value.as_basic_value()))?;

		Ok(())
	}
}

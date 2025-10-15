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

use crate::{AssemblyError, ContextExt as _, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
	fn start_lifetime(
		&self,
		alloc_len: IntValue<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<impl Drop, AssemblyError> {
		struct LifetimeEnd<'builder, 'ctx> {
			builder: &'builder Builder<'ctx>,
			end: FunctionValue<'ctx>,
			pointer: PointerValue<'ctx>,
			alloc_len: IntValue<'ctx>,
		}

		impl Drop for LifetimeEnd<'_, '_> {
			fn drop(&mut self) {
				self.builder
					.build_call(
						self.end,
						&[self.alloc_len.into(), self.pointer.into()],
						"\0",
					)
					.unwrap();
			}
		}

		self.builder.build_call(
			self.functions.lifetime.start,
			&[alloc_len.into(), pointer.into()],
			"\0",
		)?;

		let lifetime_end = LifetimeEnd {
			builder: &self.builder,
			end: self.functions.lifetime.end,
			pointer,
			alloc_len,
		};

		Ok(lifetime_end)
	}

	#[tracing::instrument(skip_all)]
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

		let assert_ptr_not_null = self.builder.build_is_not_null(ptr_param, "\0")?;

		self.builder.build_direct_call(
			self.functions.assume,
			&[assert_ptr_not_null.into()],
			"\0",
		)?;

		let end_of_string = self.gep(i8_type, ptr_param, string_len, "end_of_string")?;

		let i64_zero = i64_type.const_zero();

		let is_string_len_zero =
			self.builder
				.build_int_compare(IntPredicate::EQ, string_len, i64_zero, "\0")?;

		self.builder
			.build_conditional_branch(is_string_len_zero, exit_block, try_block)?;

		self.builder.position_at_end(try_block);

		let try_block_phi = self.builder.build_phi(ptr_type, "\0")?;

		let i64_one = i64_type.const_int(1, false);

		let next_index_gep = self.gep(
			i8_type,
			try_block_phi.as_basic_value().into_pointer_value(),
			i64_one,
			"next_char_index",
		)?;

		try_block_phi.add_incoming(&[(&ptr_param, entry_block), (&next_index_gep, continue_block)]);

		let actual_value = self
			.builder
			.build_load(
				i8_type,
				try_block_phi.as_basic_value().into_pointer_value(),
				"\0",
			)?
			.into_int_value();

		let extended_character = self
			.builder
			.build_int_z_extend(actual_value, i32_type, "\0")?;

		let putchar_call = self.builder.build_direct_invoke(
			self.functions.putchar,
			&[extended_character.into()],
			continue_block,
			catch_block,
			"\0",
		)?;

		putchar_call.set_tail_call(true);

		self.builder.position_at_end(continue_block);

		let check_if_at_end = self.builder.build_int_compare(
			IntPredicate::EQ,
			next_index_gep,
			end_of_string,
			"\0",
		)?;

		self.builder
			.build_conditional_branch(check_if_at_end, exit_block, try_block)?;

		self.builder.position_at_end(exit_block);

		self.builder.build_return(None)?;

		self.builder.position_at_end(catch_block);

		let exception_type = context.struct_type(&[ptr_type.into(), i32_type.into()], false);

		let out = self.builder.build_landing_pad(
			exception_type,
			self.functions.eh_personality,
			&[],
			true,
			"\0",
		)?;

		self.builder.build_resume(out)?;

		Ok(())
	}
}

fn create_string(prefix: &str, suffix: &'static str) -> String {
	let mut out = String::with_capacity(prefix.len() + suffix.len());

	out.push_str(prefix);
	out.push_str(suffix);

	out
}

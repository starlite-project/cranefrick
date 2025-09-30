use std::slice;

use frick_assembler::AssemblyError;
use frick_ir::{BrainIr, CellChangeOptions, OutputOptions};
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	types::ArrayType,
	values::{CallSiteValue, InstructionValueError, IntValue},
};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn output(&self, options: &OutputOptions) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		match options {
			OutputOptions::Cell(options) => {
				self.output_cells(slice::from_ref(options))?;
			}
			OutputOptions::Cells(options) => self.output_cells(options)?,
			OutputOptions::Char(c) => self.output_char(*c)?,
			OutputOptions::Str(c) => self.output_chars(c)?,
			_ => {
				return Err(AssemblyError::NotImplemented(BrainIr::Output(
					options.clone(),
				)));
			}
		}

		Ok(())
	}

	fn output_cells(&self, options: &[CellChangeOptions<i8>]) -> Result<(), LlvmAssemblyError> {
		if options.len() < 8 {
			self.output_cells_puts(options)
		} else {
			self.output_cells_iterated(options)
		}
	}

	fn output_cells_puts(
		&self,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		assert!(options.len() < 8);

		let i8_type = self.context().i8_type();
		let i64_type = self.context().i64_type();
		let ptr_int_type = self.ptr_int_type;

		let lifetime_array_len = i64_type.const_int(options.len() as u64 + 1, false);

		let array_len = i64_type.const_int(options.len() as u64, false);

		let _lifetime = self.start_lifetime(lifetime_array_len, self.pointers.output)?;

		for (i, char) in options.iter().copied().enumerate() {
			let loaded_char = self.load(char.offset(), "output_cells_puts")?;

			let offset_value = if matches!(char.value(), 0) {
				loaded_char
			} else {
				let offset_value = i8_type.const_int(char.value() as u64, false);

				self.builder
					.build_int_add(loaded_char, offset_value, "output_cells_puts_add")?
			};

			let array_offset = ptr_int_type.const_int(i as u64, false);

			let output_array_gep = unsafe {
				self.builder.build_in_bounds_gep(
					i8_type,
					self.pointers.output,
					&[array_offset],
					"output_cells_puts_gep",
				)?
			};

			self.builder.build_store(output_array_gep, offset_value)?;
		}

		let zero = i8_type.const_zero();

		let last_index_gep = unsafe {
			self.builder.build_in_bounds_gep(
				i8_type,
				self.pointers.output,
				&[array_len],
				"output_cells_puts_gep",
			)?
		};

		self.builder.build_store(last_index_gep, zero)?;

		let puts_call = self.builder.build_call(
			self.functions.puts,
			&[self.pointers.output.into(), array_len.into()],
			"output_cells_puts_call",
		)?;

		let puts_value = puts_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_puts_io_attributes(
			puts_call,
			i8_type.array_type(options.len() as u32 + 1),
			options.len() as u64,
		);

		let last_cell = {
			let i32_type = self.context().i32_type();

			let last_options = options.last().copied().unwrap();

			let loaded_value = self.load(0, "output_cells_puts")?;

			let extended_value = self.builder.build_int_s_extend(
				loaded_value,
				i32_type,
				"output_cells_puts_extend",
			)?;

			let offset_to_add = i32_type.const_int(last_options.value() as u64, false);

			self.builder
				.build_int_add(extended_value, offset_to_add, "output_cells_puts_add")?
		};

		self.builder.build_call(
			self.functions.expect,
			&[puts_value.into(), last_cell.into()],
			"",
		)?;

		Ok(())
	}

	fn output_cells_iterated(
		&self,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		options
			.iter()
			.try_for_each(|x| self.output_cell(x.value(), x.offset()))
	}

	fn output_cell(&self, value_offset: i8, offset: i32) -> Result<(), LlvmAssemblyError> {
		let i32_type = self.context().i32_type();
		let loaded_value = self.load(offset, "output_current_cell")?;

		let extended_loaded_value = self.builder.build_int_z_extend(
			loaded_value,
			i32_type,
			"output_current_cell_extend",
		)?;

		if let Some(extended_instr) = extended_loaded_value.as_instruction() {
			extended_instr.set_non_negative_flag(true);
		}

		let offset_loaded_value = if matches!(value_offset, 0) {
			extended_loaded_value
		} else {
			let offset_value = i32_type.const_int(value_offset as u64, false);

			self.builder.build_int_add(
				extended_loaded_value,
				offset_value,
				"output_current_cell_add",
			)?
		};

		let putchar_call = self.builder.build_call(
			self.functions.putchar,
			&[offset_loaded_value.into()],
			"output_current_cell_call",
		)?;

		putchar_call.set_tail_call(true);

		let putchar_value = putchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(putchar_value, u8::MIN.into(), u8::MAX.into())?;

		self.builder.build_call(
			self.functions.expect,
			&[offset_loaded_value.into(), putchar_value.into()],
			"",
		)?;

		Ok(())
	}

	fn output_char(&self, c: u8) -> Result<(), LlvmAssemblyError> {
		let char_to_put = {
			let i32_type = self.context().i32_type();

			i32_type.const_int(c.into(), false)
		};

		let putchar_call = self.builder.build_call(
			self.functions.putchar,
			&[char_to_put.into()],
			"output_char_call",
		)?;

		putchar_call.set_tail_call(true);

		let putchar_value = putchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(putchar_value, c.into(), c.into())?;

		self.builder.build_call(
			self.functions.expect,
			&[putchar_value.into(), char_to_put.into()],
			"",
		)?;

		Ok(())
	}

	fn output_chars(&self, c: &[u8]) -> Result<(), LlvmAssemblyError> {
		let constant_initializer = self.context().const_string(c, true);

		let constant_s_ty = constant_initializer.get_type();

		let global_constant = self.module.add_global(constant_s_ty, None, "output_chars");

		global_constant.set_initializer(&constant_initializer);
		global_constant.set_constant(true);

		let global_constant_pointer = global_constant.as_pointer_value();

		let array_len = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(c.len() as u64, false)
		};

		let puts_call = self.builder.build_call(
			self.functions.puts,
			&[global_constant_pointer.into(), array_len.into()],
			"output_chars_call",
		)?;

		let puts_value = puts_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		let last = c.last().copied().unwrap();

		let last_value = {
			let i32_type = self.context().i32_type();

			i32_type.const_int(last.into(), false)
		};

		self.builder.build_call(
			self.functions.expect,
			&[puts_value.into(), last_value.into()],
			"",
		)?;

		self.add_range_io_metadata(puts_value, last.into(), last.into())?;

		self.add_puts_io_attributes(puts_call, constant_s_ty, c.len() as u64);

		Ok(())
	}

	fn add_range_io_metadata(
		&self,
		char_call: IntValue<'ctx>,
		min_inclusive: u64,
		max_inclusive: u64,
	) -> Result<(), InstructionValueError> {
		let Some(char_instr) = char_call.as_instruction() else {
			return Ok(());
		};

		let i32_type = self.context().i32_type();

		let i32_i8_min = i32_type.const_int(min_inclusive, false);

		let i32_i8_max = i32_type.const_int(max_inclusive + 1, false);

		let range_metadata_id = self.context().get_kind_id("range");

		let range_metadata_node = self
			.context()
			.metadata_node(&[i32_i8_min.into(), i32_i8_max.into()]);

		char_instr.set_metadata(range_metadata_node, range_metadata_id)
	}

	fn add_puts_io_attributes(
		&self,
		call: CallSiteValue<'ctx>,
		array_ty: ArrayType<'ctx>,
		array_len: u64,
	) {
		call.set_tail_call(true);

		let byref_attr = self
			.context()
			.create_type_attribute(Attribute::get_named_enum_kind_id("byref"), array_ty.into());

		let deref_attr = self.context().create_enum_attribute(
			Attribute::get_named_enum_kind_id("dereferenceable"),
			array_len + 1,
		);

		let align_attr = self
			.context()
			.create_enum_attribute(Attribute::get_named_enum_kind_id("align"), 1);

		for attribute in [byref_attr, deref_attr, align_attr] {
			call.add_attribute(AttributeLoc::Param(0), attribute);
		}
	}

	pub fn input_into_cell(&self) -> Result<(), LlvmAssemblyError> {
		let i8_type = self.context().i8_type();

		let getchar_call =
			self.builder
				.build_call(self.functions.getchar, &[], "input_into_cell_call")?;

		getchar_call.set_tail_call(true);

		let getchar_value = getchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(getchar_value, u8::MIN.into(), u8::MAX.into())?;

		let truncated_value =
			self.builder
				.build_int_truncate(getchar_value, i8_type, "input_into_cell_truncate")?;

		self.store(truncated_value, 0, "input_into_cell")
	}
}

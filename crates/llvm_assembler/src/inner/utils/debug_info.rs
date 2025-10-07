use std::ops::{Deref, DerefMut};

use frick_assembler::TAPE_SIZE;
use inkwell::{
	AddressSpace,
	builder::Builder,
	debug_info::{
		AsDIScope as _, DICompileUnit, DIFlagsConstants as _, DWARFEmissionKind,
		DWARFSourceLanguage, DebugInfoBuilder,
	},
	module::Module,
	values::{InstructionValue, PointerValue},
};

use super::{AssemblerFunctions, AssemblerPointers, OUTPUT_ARRAY_LEN};
use crate::LlvmAssemblyError;

pub struct AssemblerDebugBuilder<'ctx> {
	pub di_builder: DebugInfoBuilder<'ctx>,
	pub compile_unit: DICompileUnit<'ctx>,
}

impl<'ctx> AssemblerDebugBuilder<'ctx> {
	pub fn new(
		module: &Module<'ctx>,
		file_name: &str,
		directory: &str,
	) -> Result<Self, LlvmAssemblyError> {
		let (di_builder, compile_unit) = module.create_debug_info_builder(
			true,
			DWARFSourceLanguage::C,
			file_name,
			directory,
			"frick",
			false,
			"",
			0,
			"",
			DWARFEmissionKind::Full,
			0,
			false,
			false,
			"",
			"",
		);

		Ok(Self {
			di_builder,
			compile_unit,
		})
	}

	#[allow(clippy::single_range_in_vec_init)]
	pub fn setup(
		self,
		builder: &Builder<'ctx>,
		functions: AssemblerFunctions<'ctx>,
		pointers: AssemblerPointers<'ctx>,
	) -> Result<Self, LlvmAssemblyError> {
		let entry_block = functions.main.get_first_basic_block().unwrap();

		let main_subroutine_type =
			self.create_subroutine_type(self.compile_unit.get_file(), None, &[], i32::PUBLIC);

		let main_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"main",
			None,
			self.compile_unit.get_file(),
			0,
			main_subroutine_type,
			true,
			true,
			0,
			i32::PUBLIC,
			true,
		);

		functions.main.set_subprogram(main_subprogram);

		let context = functions.main.get_type().get_context();

		let debug_loc = self.create_debug_location(
			functions.main.get_type().get_context(),
			1,
			0,
			main_subprogram.as_debug_info_scope(),
			None,
		);

		let i8_di_type = self
			.di_builder
			.create_basic_type("u8", 8, 7, i32::ZERO)?
			.as_type();

		let i8_di_ptr_type = self
			.di_builder
			.create_pointer_type("ptr(u8)", i8_di_type, 64, 64, AddressSpace::default())
			.as_type();

		let i32_di_type = self
			.di_builder
			.create_basic_type("char", 32, 7, i32::ZERO)?
			.as_type();

		let puts_subroutine_type = self.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[i8_di_ptr_type],
			i32::ZERO,
		);

		let puts_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"puts",
			None,
			self.compile_unit.get_file(),
			0,
			puts_subroutine_type,
			true,
			true,
			0,
			i32::PRIVATE,
			true,
		);

		functions.puts.set_subprogram(puts_subprogram);

		let putchar_subroutine_type = self.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[i32_di_type],
			i32::ZERO,
		);

		let putchar_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"putchar",
			Some("rust_putchar"),
			self.compile_unit.get_file(),
			0,
			putchar_subroutine_type,
			false,
			false,
			0,
			i32::ZERO,
			true,
		);

		functions.putchar.set_subprogram(putchar_subprogram);

		let getchar_subroutine_type = self.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[],
			i32::ZERO,
		);

		let getchar_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"getchar",
			Some("getchar"),
			self.compile_unit.get_file(),
			0,
			getchar_subroutine_type,
			false,
			false,
			0,
			i32::ZERO,
			true,
		);

		functions.getchar.set_subprogram(getchar_subprogram);

		let i8_array_di_type = self
			.di_builder
			.create_array_type(
				i8_di_type,
				TAPE_SIZE as u64 * 8,
				1,
				&[0..(TAPE_SIZE as i64)],
			)
			.as_type();

		let tape_variable = self.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"tape",
			self.compile_unit.get_file(),
			0,
			i8_array_di_type,
			false,
			i32::ZERO,
			1,
		);

		let right_after_tape_alloca = get_instruction_after_alloca(pointers.tape)?;

		self.insert_declare_before_instruction(
			pointers.tape,
			Some(tape_variable),
			None,
			debug_loc,
			right_after_tape_alloca,
		);

		let i64_di_type = self
			.di_builder
			.create_basic_type("u64", 64, 7, i32::PUBLIC)?
			.as_type();

		let pointer_variable = self.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"pointer",
			self.compile_unit.get_file(),
			0,
			i64_di_type,
			false,
			i32::ZERO,
			8,
		);

		let right_after_pointer_alloca = get_instruction_after_alloca(pointers.pointer)?;

		self.insert_declare_before_instruction(
			pointers.pointer,
			Some(pointer_variable),
			None,
			debug_loc,
			right_after_pointer_alloca,
		);

		let i8_array_di_type = self
			.di_builder
			.create_array_type(i8_di_type, 8 * u64::from(OUTPUT_ARRAY_LEN), 1, &[0..8])
			.as_type();

		let output_variable = self.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"output",
			self.compile_unit.get_file(),
			0,
			i8_array_di_type,
			false,
			i32::ZERO,
			1,
		);

		// Need to do this as `setup` is called before any other instructions are added
		self.insert_declare_at_end(
			pointers.output,
			Some(output_variable),
			None,
			debug_loc,
			entry_block,
		);

		let debug_loc =
			self.create_debug_location(context, 1, 0, main_subprogram.as_debug_info_scope(), None);

		builder.set_current_debug_location(debug_loc);

		Ok(self)
	}
}

impl<'ctx> Deref for AssemblerDebugBuilder<'ctx> {
	type Target = DebugInfoBuilder<'ctx>;

	fn deref(&self) -> &Self::Target {
		&self.di_builder
	}
}

impl DerefMut for AssemblerDebugBuilder<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.di_builder
	}
}

fn get_instruction_after_alloca(
	alloca: PointerValue<'_>,
) -> Result<InstructionValue<'_>, LlvmAssemblyError> {
	let alloca_name = || alloca.get_name().to_string_lossy().into_owned();
	let alloca_instr =
		alloca
			.as_instruction()
			.ok_or_else(|| LlvmAssemblyError::MissingPointerInstruction {
				alloca_name: alloca_name(),
				looking_after: false,
			})?;

	alloca_instr.get_next_instruction().ok_or_else(|| {
		LlvmAssemblyError::MissingPointerInstruction {
			alloca_name: alloca_name(),
			looking_after: true,
		}
	})
}

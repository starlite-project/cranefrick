use frick_assembler::TAPE_SIZE;
use inkwell::{
	AddressSpace,
	debug_info::{
		AsDIScope as _, DICompileUnit, DIFlagsConstants as _, DWARFEmissionKind,
		DWARFSourceLanguage, DebugInfoBuilder,
	},
	module::Module,
};

use super::{AssemblerFunctions, AssemblerPointers};
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

	pub fn setup(
		self,
		functions: AssemblerFunctions<'ctx>,
		pointers: AssemblerPointers<'ctx>,
	) -> Result<Self, LlvmAssemblyError> {
		let entry_block = functions.main.get_first_basic_block().unwrap();

		let main_subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			None,
			&[],
			i32::PUBLIC,
		);

		let main_subprogram = self.di_builder.create_function(
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

		let i32_di_type = self
			.di_builder
			.create_basic_type("u32", 32, 7, i32::ZERO)?
			.as_type();

		let putchar_subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[i32_di_type],
			i32::ZERO,
		);

		let putchar_subprogram = self.di_builder.create_function(
			self.compile_unit.as_debug_info_scope(),
			"putchar",
			Some("putchar"),
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

		let getchar_subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[],
			i32::ZERO,
		);

		let getchar_subprogram = self.di_builder.create_function(
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

		let i8_di_type = self
			.di_builder
			.create_basic_type("u8", 8, 7, i32::ZERO)?
			.as_type();

		let i8_ptr_di_type = self
			.di_builder
			.create_pointer_type("*u8", i8_di_type, 64, 64, AddressSpace::default())
			.as_type();

		let puts_subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[i8_ptr_di_type],
			i32::ZERO,
		);

		let puts_subprogram = self.di_builder.create_function(
			self.compile_unit.as_debug_info_scope(),
			"puts",
			Some("puts"),
			self.compile_unit.get_file(),
			0,
			puts_subroutine_type,
			false,
			false,
			0,
			i32::ZERO,
			true,
		);

		functions.puts.set_subprogram(puts_subprogram);

		let debug_loc = self.di_builder.create_debug_location(
			functions.main.get_type().get_context(),
			0,
			0,
			main_subprogram.as_debug_info_scope(),
			None,
		);

		let i8_array_di_type = self
			.di_builder
			.create_array_type(i8_di_type, TAPE_SIZE as u64 * 8, 1, &[])
			.as_type();

		let tape_variable = self.di_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"tape",
			self.compile_unit.get_file(),
			0,
			i8_array_di_type,
			false,
			i32::ZERO,
			1,
		);

		self.di_builder.insert_declare_at_end(
			pointers.tape,
			Some(tape_variable),
			None,
			debug_loc,
			entry_block,
		);

		let i64_di_type = self
			.di_builder
			.create_basic_type("u64", 64, 7, i32::PUBLIC)?
			.as_type();

		let pointer_variable = self.di_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"pointer",
			self.compile_unit.get_file(),
			1,
			i64_di_type,
			false,
			i32::ZERO,
			8,
		);

		self.di_builder.insert_declare_at_end(
			pointers.pointer,
			Some(pointer_variable),
			None,
			debug_loc,
			entry_block,
		);

		let i8_array_di_type = self
			.di_builder
			.create_array_type(i8_di_type, 64, 1, &[])
			.as_type();

		let output_variable = self.di_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"output",
			self.compile_unit.get_file(),
			0,
			i8_array_di_type,
			false,
			i32::ZERO,
			1,
		);

		self.di_builder.insert_declare_at_end(
			pointers.output,
			Some(output_variable),
			None,
			debug_loc,
			entry_block,
		);

		Ok(self)
	}
}

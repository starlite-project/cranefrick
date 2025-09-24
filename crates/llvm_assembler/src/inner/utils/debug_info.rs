use frick_assembler::TAPE_SIZE;
use inkwell::{
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
			true,
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
			.create_basic_type("u32", 4, 7, i32::PUBLIC)?
			.as_type();

		let putchar_subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			None,
			&[i32_di_type],
			i32::PUBLIC,
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
			i32::PUBLIC,
			true,
		);

		functions.putchar.set_subprogram(putchar_subprogram);

		let getchar_subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[],
			i32::PUBLIC,
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
			i32::PUBLIC,
			true,
		);

		functions.getchar.set_subprogram(getchar_subprogram);

		let i8_di_type = self
			.di_builder
			.create_basic_type("u8", 1, 7, i32::PUBLIC)?
			.as_type();

		let i8_array_di_type = self
			.di_builder
			.create_array_type(i8_di_type, TAPE_SIZE as u64, 1, &[])
			.as_type();

		let tape_variable = self.di_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"tape",
			self.compile_unit.get_file(),
			0,
			i8_array_di_type,
			false,
			i32::PRIVATE,
			1,
		);

		let debug_loc = self.di_builder.create_debug_location(
			functions.main.get_type().get_context(),
			0,
			0,
			main_subprogram.as_debug_info_scope(),
			None,
		);

		self.di_builder.insert_declare_before_instruction(
			pointers.tape,
			Some(tape_variable),
			None,
			debug_loc,
			pointers.tape.as_instruction().unwrap(),
		);

		let i64_di_type = self
			.di_builder
			.create_basic_type("u64", 8, 7, i32::PUBLIC)?
			.as_type();

		let pointer_variable = self.di_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"pointer",
			self.compile_unit.get_file(),
			1,
			i64_di_type,
			false,
			i32::PRIVATE,
			8,
		);

		self.di_builder.insert_declare_before_instruction(
			pointers.pointer,
			Some(pointer_variable),
			None,
			debug_loc,
			pointers.pointer.as_instruction().unwrap(),
		);

		Ok(self)
	}
}

mod impls;
mod utils;

use std::path::Path;

use frick_assembler::{AssemblyError, TAPE_SIZE};
use frick_ir::BrainIr;
use frick_utils::GetOrZero as _;
use inkwell::{
	builder::Builder,
	context::{Context, ContextRef},
	debug_info::{
		AsDIScope as _, DICompileUnit, DIFlagsConstants as _, DWARFEmissionKind,
		DWARFSourceLanguage, DebugInfoBuilder,
	},
	module::{FlagBehavior, Module},
	targets::TargetMachine,
	types::IntType,
};

pub use self::utils::AssemblerFunctions;
use self::utils::AssemblerPointers;
use super::LlvmAssemblyError;

pub struct InnerAssembler<'ctx> {
	module: Module<'ctx>,
	builder: Builder<'ctx>,
	functions: AssemblerFunctions<'ctx>,
	pointers: AssemblerPointers<'ctx>,
	ptr_int_type: IntType<'ctx>,
	target_machine: TargetMachine,
	di_builder: DebugInfoBuilder<'ctx>,
	compile_unit: DICompileUnit<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(
		context: &'ctx Context,
		target_machine: TargetMachine,
		path: Option<&Path>,
	) -> Result<Self, LlvmAssemblyError> {
		let module = context.create_module("frick");
		let functions = AssemblerFunctions::new(context, &module)?;
		let builder = context.create_builder();

		let basic_block = context.append_basic_block(functions.main, "entry");
		builder.position_at_end(basic_block);

		let (pointers, ptr_int_type) = AssemblerPointers::new(&module, functions, &builder)?;

		let debug_metadata_version = {
			let i32_type = context.i32_type();

			i32_type.const_int(inkwell::debug_info::debug_metadata_version().into(), false)
		};

		module.add_basic_value_flag(
			"Debug Info Version",
			FlagBehavior::Warning,
			debug_metadata_version,
		);

		let (file_name, directory) = if let Some(path) = path {
			assert!(path.is_file());

			let file_name = path
				.file_name()
				.map(|s| s.to_string_lossy().into_owned())
				.unwrap_or_default();

			let directory = path
				.parent()
				.and_then(|s| s.canonicalize().ok())
				.map(|s| s.to_string_lossy().into_owned())
				.unwrap_or_default();

			(file_name, directory)
		} else {
			("frick_source_file.bf".to_owned(), "/".to_owned())
		};

		let (di_builder, compile_unit) = module.create_debug_info_builder(
			true,
			DWARFSourceLanguage::C,
			&file_name,
			&directory,
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

		let this = Self {
			module,
			builder,
			functions,
			pointers,
			ptr_int_type,
			target_machine,
			di_builder,
			compile_unit,
		};

		this.setup_debug_info()?;

		Ok(this)
	}

	fn setup_debug_info(&self) -> Result<(), LlvmAssemblyError> {
		let subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			None,
			&[],
			i32::PUBLIC,
		);

		let func_scope = self.di_builder.create_function(
			self.compile_unit.as_debug_info_scope(),
			"main",
			None,
			self.compile_unit.get_file(),
			0,
			subroutine_type,
			true,
			true,
			0,
			i32::PUBLIC,
			true,
		);

		self.functions.main.set_subprogram(func_scope);

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

		let putchar_func_scope = self.di_builder.create_function(
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

		self.functions.putchar.set_subprogram(putchar_func_scope);

		let getchar_subroutine_type = self.di_builder.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(i32_di_type),
			&[],
			i32::PUBLIC,
		);

		let getchar_func_scope = self.di_builder.create_function(
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

		self.functions.getchar.set_subprogram(getchar_func_scope);

		Ok(())
	}

	pub fn context(&self) -> ContextRef<'ctx> {
		self.module.get_context()
	}

	pub fn assemble(
		self,
		ops: &[BrainIr],
	) -> Result<
		(Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine),
		AssemblyError<LlvmAssemblyError>,
	> {
		self.ops(ops)?;

		let i64_size = {
			let i64_type = self.context().i64_type();

			i64_type.const_int(8, false)
		};

		let tape_size = {
			let ptr_int_type = self.ptr_int_type;

			ptr_int_type.const_int(TAPE_SIZE as u64, false)
		};

		self.builder
			.build_call(
				self.functions.lifetime.end,
				&[tape_size.into(), self.pointers.tape.into()],
				"",
			)
			.map_err(AssemblyError::backend)?;
		self.builder
			.build_call(
				self.functions.lifetime.end,
				&[i64_size.into(), self.pointers.pointer.into()],
				"",
			)
			.map_err(AssemblyError::backend)?;

		self.builder
			.build_return(None)
			.map_err(AssemblyError::backend)?;

		let data_layout = self.target_machine.get_target_data().get_data_layout();

		let target_triple = {
			let default_target = TargetMachine::get_default_triple();

			TargetMachine::normalize_triple(&default_target)
		};

		self.module.set_data_layout(&data_layout);
		self.module.set_triple(&target_triple);

		self.di_builder.finalize();

		Ok(self.into_parts())
	}

	fn ops(&self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		for op in ops {
			match op {
				BrainIr::MovePointer(offset) => self.move_pointer(*offset)?,
				BrainIr::SetCell(value, offset) => {
					self.set_cell(*value, offset.get_or_zero())?;
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.get_or_zero())?;
				}
				BrainIr::SubCellAt(options) => self.sub_cell_at(*options)?,
				BrainIr::SubFromCell(options) => self.sub_from_cell(*options)?,
				BrainIr::DuplicateCell { values } => self.duplicate_cell(values)?,
				BrainIr::Output(options) => self.output(options)?,
				BrainIr::InputIntoCell => self.input_into_cell()?,
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops)?,
				BrainIr::IfNotZero(ops) => self.if_not_zero(ops)?,
				BrainIr::FindZero(offset) => self.find_zero(*offset)?,
				BrainIr::MoveValueTo(options) => self.move_value_to(*options)?,
				BrainIr::CopyValueTo(options) => self.copy_value_to(*options)?,
				BrainIr::TakeValueTo(options) => self.take_value_to(*options)?,
				BrainIr::FetchValueFrom(options) => {
					self.fetch_value_from(*options)?;
				}
				BrainIr::ReplaceValueFrom(options) => {
					self.replace_value_from(*options)?;
				}
				BrainIr::ScaleValue(factor) => self.scale_value(*factor)?,
				BrainIr::SetRange { value, range } => self.set_range(*value, range.clone())?,
				BrainIr::SetManyCells { values, start } => {
					self.set_many_cells(values, start.get_or_zero())?;
				}
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}
		}

		Ok(())
	}

	fn into_parts(self) -> (Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine) {
		(self.module, self.functions, self.target_machine)
	}
}

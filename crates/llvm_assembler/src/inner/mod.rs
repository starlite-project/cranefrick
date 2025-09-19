mod impls;

use std::{iter, path::Path};

use frick_assembler::{AssemblyError, TAPE_SIZE};
use frick_ir::BrainIr;
use frick_utils::GetOrZero as _;
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	builder::Builder,
	context::{Context, ContextRef},
	debug_info::{
		AsDIScope as _, DICompileUnit, DIFlagsConstants as _, DWARFEmissionKind,
		DWARFSourceLanguage, DebugInfoBuilder,
	},
	intrinsics::Intrinsic,
	module::{FlagBehavior, Linkage, Module},
	targets::TargetMachine,
	types::IntType,
	values::{FunctionValue, PointerValue},
};

use super::{ContextExt, LlvmAssemblyError};

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

		this.setup_debug_info();

		Ok(this)
	}

	fn setup_debug_info(&self) {
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

#[derive(Clone, Copy)]
pub struct AssemblerFunctions<'ctx> {
	#[allow(dead_code)]
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
	pub lifetime: IntrinsicFunctionSet<'ctx>,
}

impl<'ctx> AssemblerFunctions<'ctx> {
	fn new(context: &'ctx Context, module: &Module<'ctx>) -> Result<Self, LlvmAssemblyError> {
		let ptr_type = context.default_ptr_type();
		let void_type = context.void_type();
		let i32_type = context.i32_type();

		let getchar_ty = void_type.fn_type(&[ptr_type.into()], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i32_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, Some(Linkage::External));

		let lifetime_start_intrinsic = Intrinsic::find("llvm.lifetime.start")
			.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.start"))?;
		let lifetime_end_intrinsic = Intrinsic::find("llvm.lifetime.end")
			.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.end"))?;

		let lifetime = {
			let context = module.get_context();
			let ptr_type = context.default_ptr_type();

			let lifetime_start = lifetime_start_intrinsic
				.get_declaration(module, &[ptr_type.into()])
				.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.start"))?;

			let lifetime_end = lifetime_end_intrinsic
				.get_declaration(module, &[ptr_type.into()])
				.ok_or_else(|| LlvmAssemblyError::intrinsic("llvm.lifetime.end"))?;

			IntrinsicFunctionSet::new(lifetime_start, lifetime_end)
		};

		let this = Self {
			getchar,
			putchar,
			main,
			lifetime,
		};

		Ok(this.setup(context))
	}

	fn setup(self, context: &'ctx Context) -> Self {
		self.setup_common_attributes(context)
			.setup_getchar_attributes(context)
			.setup_putchar_attributes(context)
	}

	fn setup_common_attributes(self, context: &'ctx Context) -> Self {
		let noundef_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noundef"), 0);

		for attribute in iter::once(noundef_attr) {
			self.putchar
				.add_attribute(AttributeLoc::Param(0), attribute);
			self.getchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
		let nonlazybind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonlazybind"), 0);
		let nocallback_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nocallback"), 0);
		let norecurse_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("norecurse"), 0);
		let willreturn_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("willreturn"), 0);
		let nosync_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nosync"), 0);

		for attribute in [
			nofree_attr,
			nonlazybind_attr,
			nocallback_attr,
			norecurse_attr,
			willreturn_attr,
			nosync_attr,
		] {
			self.getchar
				.add_attribute(AttributeLoc::Function, attribute);
			self.putchar
				.add_attribute(AttributeLoc::Function, attribute);
		}

		self
	}

	fn setup_getchar_attributes(self, context: &'ctx Context) -> Self {
		let memory_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("memory"), 6);

		self.getchar
			.add_attribute(AttributeLoc::Function, memory_attr);

		let writeonly_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("writeonly"), 0);
		let nocapture_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nocapture"), 0);
		let noalias_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noalias"), 0);
		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
		let nonnull_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonnull"), 0);
		let dead_on_unwind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("dead_on_unwind"), 0);

		for attribute in [
			writeonly_attr,
			nocapture_attr,
			noalias_attr,
			nofree_attr,
			nonnull_attr,
			dead_on_unwind_attr,
		] {
			self.getchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}

	fn setup_putchar_attributes(self, context: &'ctx Context) -> Self {
		let memory_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("memory"), 9);

		for attribute in iter::once(memory_attr) {
			self.putchar
				.add_attribute(AttributeLoc::Function, attribute);
		}

		let zeroext_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("zeroext"), 0);

		for attribute in iter::once(zeroext_attr) {
			self.putchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}
}

#[derive(Clone, Copy)]
pub struct IntrinsicFunctionSet<'ctx> {
	start: FunctionValue<'ctx>,
	end: FunctionValue<'ctx>,
}

impl<'ctx> IntrinsicFunctionSet<'ctx> {
	const fn new(start: FunctionValue<'ctx>, end: FunctionValue<'ctx>) -> Self {
		Self { start, end }
	}
}

#[derive(Clone, Copy)]
pub struct AssemblerPointers<'ctx> {
	pub tape: PointerValue<'ctx>,
	pub pointer: PointerValue<'ctx>,
	pub input: PointerValue<'ctx>,
}

impl<'ctx> AssemblerPointers<'ctx> {
	fn new(
		module: &Module<'ctx>,
		functions: AssemblerFunctions<'ctx>,
		builder: &Builder<'ctx>,
	) -> Result<(Self, IntType<'ctx>), LlvmAssemblyError> {
		let context = module.get_context();
		let i8_type = context.i8_type();
		let ptr_int_type = context.i64_type();

		let tape = {
			let i8_array_type = i8_type.array_type(TAPE_SIZE as u32);
			let array_size_value = ptr_int_type.const_int(TAPE_SIZE as u64, false);

			let tape_alloca = builder.build_alloca(i8_array_type, "tape")?;

			builder.build_call(
				functions.lifetime.start,
				&[array_size_value.into(), tape_alloca.into()],
				"",
			)?;

			builder.build_memset(tape_alloca, 1, i8_type.const_zero(), array_size_value)?;

			tape_alloca
		};

		let pointer = {
			let pointer_alloca = builder.build_alloca(ptr_int_type, "pointer")?;

			let i64_size = {
				let i64_type = context.i64_type();

				i64_type.const_int(8, false)
			};

			builder.build_call(
				functions.lifetime.start,
				&[i64_size.into(), pointer_alloca.into()],
				"",
			)?;

			builder.build_store(pointer_alloca, ptr_int_type.const_zero())?;

			pointer_alloca
		};

		let input = builder.build_alloca(i8_type, "input")?;

		Ok((
			Self {
				tape,
				pointer,
				input,
			},
			ptr_int_type,
		))
	}
}

mod impls;

use std::num::NonZero;

use frick_assembler::AssemblyError;
use frick_ir::BrainIr;
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	builder::Builder,
	context::Context,
	debug_info::{
		AsDIScope, DICompileUnit, DIFlagsConstants as _, DWARFEmissionKind, DWARFSourceLanguage,
		DebugInfoBuilder,
	},
	module::{FlagBehavior, Linkage, Module},
	targets::TargetMachine,
	types::IntType,
	values::{FunctionValue, PointerValue},
};

use super::{ContextExt, LlvmAssemblyError};

#[allow(dead_code)]
pub struct InnerAssembler<'ctx> {
	context: &'ctx Context,
	module: Module<'ctx>,
	builder: Builder<'ctx>,
	functions: Functions<'ctx>,
	tape: PointerValue<'ctx>,
	ptr: PointerValue<'ctx>,
	ptr_type: IntType<'ctx>,
	di_builder: DebugInfoBuilder<'ctx>,
	compile_unit: DICompileUnit<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(
		context: &'ctx Context,
		target_machine: &TargetMachine,
	) -> Result<Self, LlvmAssemblyError> {
		let module = context.create_module("frick");
		let functions = Functions::new(context, &module);
		let builder = context.create_builder();

		let triple = target_machine.get_triple();
		let target_data = target_machine.get_target_data();
		let data_layout = target_data.get_data_layout();

		module.set_data_layout(&data_layout);
		module.set_triple(&triple);

		let basic_block = context.append_basic_block(functions.main, "entry");
		builder.position_at_end(basic_block);

		let i64_type = context.i64_type();
		let tape = {
			let i8_type = context.i8_type();
			let i8_array_type = i8_type.array_type(30_000);
			let array_size = i64_type.const_int(30_000, false);

			let tape_alloca = builder.build_array_alloca(i8_type, array_size, "tape")?;

			builder.build_store(tape_alloca, i8_array_type.const_zero())?;

			tape_alloca
		};

		let ptr = {
			let ptr_alloca = builder.build_alloca(i64_type, "ptr")?;

			builder.build_store(ptr_alloca, i64_type.const_zero())?;

			ptr_alloca
		};

		let i32_type = context.i32_type();
		let debug_metadata_version = i32_type.const_int(3, false);
		module.add_basic_value_flag(
			"Debug Info Version",
			FlagBehavior::Warning,
			debug_metadata_version,
		);

		let (di_builder, compile_unit) = module.create_debug_info_builder(
			true,
			DWARFSourceLanguage::C,
			"file",
			".",
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

		let subroutine_type =
			di_builder.create_subroutine_type(compile_unit.get_file(), None, &[], i32::PUBLIC);

		let func_scope = di_builder.create_function(
			compile_unit.as_debug_info_scope(),
			"main",
			None,
			compile_unit.get_file(),
			0,
			subroutine_type,
			true,
			true,
			0,
			i32::PUBLIC,
			false,
		);

		functions.main.set_subprogram(func_scope);

		let i8_di_type = di_builder
			.create_basic_type("ty8", 1, 7, i32::PRIVATE)
			.unwrap();

		let i8_di_array_type = di_builder.create_array_type(i8_di_type.as_type(), 30_000, 1, &[]);

		let tape_variable = di_builder.create_auto_variable(
			func_scope.as_debug_info_scope(),
			"tape",
			compile_unit.get_file(),
			0,
			i8_di_array_type.as_type(),
			false,
			i32::PRIVATE,
			1,
		);

		let tape_alloca_instr = tape.as_instruction().unwrap();

		let tape_location =
			di_builder.create_debug_location(context, 0, 0, func_scope.as_debug_info_scope(), None);

		di_builder.insert_declare_before_instruction(
			tape,
			Some(tape_variable),
			None,
			tape_location,
			tape_alloca_instr,
		);

		let i64_di_type = di_builder
			.create_basic_type("ty64", 8, 7, i32::PRIVATE)
			.unwrap();

		let ptr_variable = di_builder.create_auto_variable(
			func_scope.as_debug_info_scope(),
			"ptr",
			compile_unit.get_file(),
			1,
			i64_di_type.as_type(),
			false,
			i32::PRIVATE,
			8,
		);

		let ptr_alloca_instr = ptr.as_instruction().unwrap();

		let ptr_location =
			di_builder.create_debug_location(context, 0, 0, func_scope.as_debug_info_scope(), None);

		di_builder.insert_declare_before_instruction(
			ptr,
			Some(ptr_variable),
			None,
			ptr_location,
			ptr_alloca_instr,
		);

		Ok(Self {
			context,
			module,
			builder,
			functions,
			tape,
			ptr,
			ptr_type: i64_type,
			di_builder,
			compile_unit,
		})
	}

	pub fn assemble(
		self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, Functions<'ctx>), AssemblyError<LlvmAssemblyError>> {
		self.ops(ops)?;

		self.builder
			.build_return(None)
			.map_err(AssemblyError::backend)?;

		self.di_builder.finalize();

		Ok(self.into_parts())
	}

	fn ops(&self, ops: &[BrainIr]) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		for op in ops {
			match op {
				BrainIr::MovePointer(offset) => self.move_pointer(*offset)?,
				BrainIr::SetCell(value, offset) => {
					self.set_cell(*value, offset.map_or(0, NonZero::get))?;
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.map_or(0, NonZero::get))?;
				}
				BrainIr::SubCell(offset) => self.sub_cell(*offset)?,
				BrainIr::OutputCurrentCell => self.output_current_cell()?,
				BrainIr::OutputChar(c) => self.output_char(*c)?,
				BrainIr::OutputChars(c) => self.output_chars(c)?,
				BrainIr::InputIntoCell => self.input_into_cell()?,
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops)?,
				BrainIr::IfNotZero(ops) => self.if_not_zero(ops)?,
				BrainIr::FindZero(offset) => self.find_zero(*offset)?,
				BrainIr::MoveValueTo(factor, offset) => self.move_value_to(*factor, *offset)?,
				BrainIr::TakeValueTo(factor, offset) => self.take_value_to(*factor, *offset)?,
				BrainIr::FetchValueFrom(factor, offset) => {
					self.fetch_value_from(*factor, *offset)?;
				}
				BrainIr::ReplaceValueFrom(factor, offset) => {
					self.replace_value_from(*factor, *offset)?;
				}
				BrainIr::ScaleValue(factor) => self.scale_value(*factor)?,
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}
		}

		Ok(())
	}

	fn into_parts(self) -> (Module<'ctx>, Functions<'ctx>) {
		(self.module, self.functions)
	}
}

#[derive(Clone, Copy)]
pub struct Functions<'ctx> {
	#[allow(dead_code)]
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
}

impl<'ctx> Functions<'ctx> {
	fn new(context: &'ctx Context, module: &Module<'ctx>) -> Self {
		let ptr_type = context.default_ptr_type();
		let void_type = context.void_type();
		let i32_type = context.i32_type();

		let getchar_ty = void_type.fn_type(&[ptr_type.into()], false);
		let getchar = module.add_function("getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i32_type.into()], false);
		let putchar = module.add_function("putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, Some(Linkage::External));

		let this = Self {
			getchar,
			putchar,
			main,
		};

		this.setup(context)
	}

	fn setup(self, context: &'ctx Context) -> Self {
		self.setup_common_attributes(context)
			.setup_getchar_attributes(context)
			.setup_putchar_attributes(context)
	}

	fn setup_common_attributes(self, context: &'ctx Context) -> Self {
		let noundef_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("noundef"), 0);

		self.putchar
			.add_attribute(AttributeLoc::Param(0), noundef_attr);
		self.getchar
			.add_attribute(AttributeLoc::Param(0), noundef_attr);

		let nofree_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
		let nonlazybind_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nonlazybind"), 0);
		let uwtable_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("uwtable"), 2);
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
			uwtable_attr,
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
		let writeonly_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("writeonly"), 0);
		let nocapture_attr =
			context.create_enum_attribute(Attribute::get_named_enum_kind_id("nocapture"), 0);

		for attribute in [writeonly_attr, nocapture_attr] {
			self.getchar
				.add_attribute(AttributeLoc::Param(0), attribute);
		}

		self
	}

	const fn setup_putchar_attributes(self, context: &'ctx Context) -> Self {
		let _ = context;
		self
	}
}

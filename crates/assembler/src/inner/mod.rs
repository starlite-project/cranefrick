mod impls;
mod utils;

use std::path::Path;

use frick_ir::{BrainIr, SubOptions};
use frick_spec::TAPE_SIZE;
use frick_utils::Convert as _;
use inkwell::{
	basic_block::BasicBlock,
	builder::Builder,
	context::{AsContextRef, Context},
	debug_info::AsDIScope,
	llvm_sys::prelude::LLVMContextRef,
	module::{FlagBehavior, Module},
	targets::{TargetMachine, TargetTriple},
	types::{BasicTypeEnum, VectorType},
	values::{BasicMetadataValueEnum, FunctionValue, VectorValue},
};

pub use self::utils::AssemblerFunctions;
use self::utils::{AssemblerDebugBuilder, AssemblerPointers};
use super::AssemblyError;
use crate::{ContextExt as _, ContextGetter as _, ModuleExt as _};

pub struct InnerAssembler<'ctx> {
	module: Module<'ctx>,
	builder: Builder<'ctx>,
	functions: AssemblerFunctions<'ctx>,
	pointers: AssemblerPointers<'ctx>,
	target_machine: TargetMachine,
	debug_builder: AssemblerDebugBuilder<'ctx>,
	catch_block: BasicBlock<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(
		context: &'ctx Context,
		target_machine: TargetMachine,
		target_triple: TargetTriple,
		cpu_name: &str,
		cpu_features: &str,
		path: Option<&Path>,
	) -> Result<Self, AssemblyError> {
		let module = context.create_module("frick\0");
		let functions = AssemblerFunctions::new(context, &module, cpu_name, cpu_features)?;
		let builder = context.create_builder();

		let target_data = target_machine.get_target_data();
		let data_layout = target_data.get_data_layout();

		module.set_new_debug_format(true);
		module.set_data_layout(&data_layout);
		module.set_triple(&target_triple);

		let basic_block = context.append_basic_block(functions.main, "entry\0");
		builder.position_at_end(basic_block);

		let catch_block = context.append_basic_block(functions.main, "lpad\0");

		let pointers = AssemblerPointers::new(&module, &builder, &target_data)?;

		pointers.setup(&builder, &functions)?;

		let debug_metadata_version = {
			let i32_type = context.i32_type();

			i32_type.const_int(
				inkwell::debug_info::debug_metadata_version().convert::<u64>(),
				false,
			)
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

		let debug_builder = AssemblerDebugBuilder::new(&module, &file_name, &directory)?;

		debug_builder.declare_subprograms(&functions)?;

		let debug_loc = debug_builder.create_debug_location(
			module.get_context(),
			1,
			0,
			functions
				.main
				.get_subprogram()
				.unwrap()
				.as_debug_info_scope(),
			None,
		);

		builder.set_current_debug_location(debug_loc);
		module.set_source_file_name(&file_name);

		Ok(Self {
			module,
			builder,
			functions,
			pointers,
			target_machine,
			debug_builder,
			catch_block,
		})
	}

	pub fn assemble(
		self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine), AssemblyError> {
		tracing::debug!("writing instructions");
		let mut op_count = 0;

		let ops_span = tracing::info_span!("ops").entered();
		self.ops(ops, &mut op_count)?;
		drop(ops_span);

		tracing::debug!("declaring variables");
		self.debug_builder
			.declare_variables(self.context(), self.pointers)?;

		let context = self.context();

		let i64_type = context.i64_type();
		let i32_type = context.i32_type();
		let ptr_type = context.default_ptr_type();

		let i64_size = i64_type.const_int(8, false);

		let tape_size = i64_type.const_int(TAPE_SIZE as u64, false);

		tracing::debug!("ending lifetimes in exit block");
		self.builder.build_call(
			self.functions.lifetime.end,
			&[
				tape_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.pointers.tape.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;
		self.builder.build_call(
			self.functions.lifetime.end,
			&[
				i64_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.pointers
					.pointer
					.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;

		self.builder.build_return(None)?;

		let debug_loc = self.debug_builder.create_debug_location(
			self.context(),
			1,
			op_count as u32,
			self.debug_builder.main_subprogram.as_debug_info_scope(),
			None,
		);

		self.builder.set_current_debug_location(debug_loc);

		tracing::debug!("setting up the landing pad");
		let last_basic_block = self.functions.main.get_last_basic_block().unwrap();

		if last_basic_block != self.catch_block {
			self.catch_block.move_after(last_basic_block).unwrap();
		}

		self.builder.position_at_end(self.catch_block);

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
			"exception\0",
		)?;

		tracing::debug!("ending the lifetimes in catch block");
		self.builder.build_call(
			self.functions.lifetime.end,
			&[
				tape_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.pointers.tape.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;
		self.builder.build_call(
			self.functions.lifetime.end,
			&[
				i64_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.pointers
					.pointer
					.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;

		self.builder.build_resume(exception)?;

		self.debug_builder.di_builder.finalize();

		Ok(self.into_parts())
	}

	fn ops(&self, ops: &[BrainIr], op_count: &mut usize) -> Result<(), AssemblyError> {
		for op in ops {
			tracing::trace!(%op_count, %op);
			let debug_loc = self.debug_builder.create_debug_location(
				self.context(),
				1,
				*op_count as u32,
				self.debug_builder.main_subprogram.as_debug_info_scope(),
				None,
			);

			self.builder.set_current_debug_location(debug_loc);

			match op {
				BrainIr::Boundary => {}
				&BrainIr::ChangeCell(options) => self.change_cell(options)?,
				&BrainIr::SetCell(options) => self.set_cell(options)?,
				&BrainIr::SubCell(SubOptions::CellAt(options)) => self.sub_cell_at(options)?,
				&BrainIr::SubCell(SubOptions::FromCell(options)) => self.sub_from_cell(options)?,
				&BrainIr::MovePointer(offset) => self.move_pointer(offset)?,
				&BrainIr::ScanTape(scan_tape_options) => self.scan_tape(scan_tape_options)?,
				&BrainIr::InputIntoCell(input_options) => self.input_into_cell(input_options)?,
				BrainIr::Output(options) => self.output(options)?,
				&BrainIr::MoveValueTo(options) => self.move_value_to(options)?,
				&BrainIr::TakeValueTo(options) => self.take_value_to(options)?,
				&BrainIr::FetchValueFrom(options) => self.fetch_value_from(options)?,
				&BrainIr::ReplaceValueFrom(options) => self.replace_value_from(options)?,
				&BrainIr::ScaleValue(factor) => self.scale_value(factor)?,
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops, op_count)?,
				BrainIr::IfNotZero(ops) => self.if_not_zero(ops, op_count)?,
				BrainIr::ChangeManyCells(options) => self.change_many_cells(options)?,
				&BrainIr::SetRange(options) => self.set_range(options)?,
				BrainIr::SetManyCells(options) => self.set_many_cells(options)?,
				BrainIr::DuplicateCell { values } => self.duplicate_cell(values)?,
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}

			*op_count += 1;
		}

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	fn call_vector_scatter(
		&self,
		vec_of_values: VectorValue<'ctx>,
		vec_of_pointers: VectorValue<'ctx>,
	) -> Result<(), AssemblyError> {
		assert!(
			vec_of_pointers
				.get_type()
				.get_element_type()
				.is_pointer_type()
		);
		assert_eq!(
			vec_of_values.get_type().get_size(),
			vec_of_pointers.get_type().get_size()
		);

		let context = self.context();

		let bool_type = context.bool_type();
		let i32_type = context.i32_type();

		let bool_vec_all_on = {
			let vec_of_trues =
				vec![bool_type.const_all_ones(); vec_of_values.get_type().get_size() as usize];

			VectorType::const_vector(&vec_of_trues)
		};

		let vec_store_alignment = i32_type.const_int(1, false);

		let vector_scatter = self.get_vector_scatter(vec_of_values.get_type())?;

		self.builder.build_direct_call(
			vector_scatter,
			&[
				vec_of_values.convert::<BasicMetadataValueEnum<'ctx>>(),
				vec_of_pointers.convert::<BasicMetadataValueEnum<'ctx>>(),
				vec_store_alignment.convert::<BasicMetadataValueEnum<'ctx>>(),
				bool_vec_all_on.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"call_vector_scatter\0",
		)?;

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	fn call_vector_gather(
		&self,
		res_type: VectorType<'ctx>,
		vec_of_pointers: VectorValue<'ctx>,
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		assert!(
			vec_of_pointers
				.get_type()
				.get_element_type()
				.is_pointer_type()
		);
		assert_eq!(res_type.get_size(), vec_of_pointers.get_type().get_size());

		let context = self.context();

		let bool_type = context.bool_type();
		let i32_type = context.i32_type();

		let bool_vec_all_on = {
			let vec_of_trues = vec![bool_type.const_all_ones(); res_type.get_size() as usize];

			VectorType::const_vector(&vec_of_trues)
		};

		let vec_load_alignment = i32_type.const_int(1, false);

		let vector_gather = self.get_vector_gather(res_type)?;

		Ok(self
			.builder
			.build_direct_call(
				vector_gather,
				&[
					vec_of_pointers.convert::<BasicMetadataValueEnum<'ctx>>(),
					vec_load_alignment.convert::<BasicMetadataValueEnum<'ctx>>(),
					bool_vec_all_on.convert::<BasicMetadataValueEnum<'ctx>>(),
					res_type
						.get_poison()
						.convert::<BasicMetadataValueEnum<'ctx>>(),
				],
				"call_vector_gather\0",
			)?
			.try_as_basic_value()
			.unwrap_basic()
			.into_vector_value())
	}

	fn get_vector_scatter(
		&self,
		vec_type: VectorType<'ctx>,
	) -> Result<FunctionValue<'ctx>, AssemblyError> {
		if let Some(known_fn) = self.functions.get_vector_scatter(vec_type) {
			return Ok(known_fn);
		}

		let size = vec_type.get_size();

		let ptr_vec_type = {
			let ptr_type = self.context().default_ptr_type();

			ptr_type.vec_type(size)
		};

		let fn_value = self::utils::get_intrinsic_function_from_name(
			"llvm.masked.scatter",
			&self.module,
			&[
				vec_type.convert::<BasicTypeEnum<'ctx>>(),
				ptr_vec_type.convert::<BasicTypeEnum<'ctx>>(),
			],
		)?;

		self.functions.insert_vector_scatter(vec_type, fn_value);

		Ok(fn_value)
	}

	fn get_vector_gather(
		&self,
		vec_type: VectorType<'ctx>,
	) -> Result<FunctionValue<'ctx>, AssemblyError> {
		if let Some(known_fn) = self.functions.get_vector_gather(vec_type) {
			return Ok(known_fn);
		}

		let size = vec_type.get_size();

		let ptr_vec_type = {
			let ptr_type = self.context().default_ptr_type();

			ptr_type.vec_type(size)
		};

		let fn_value = self::utils::get_intrinsic_function_from_name(
			"llvm.masked.gather",
			&self.module,
			&[
				vec_type.convert::<BasicTypeEnum<'ctx>>(),
				ptr_vec_type.convert::<BasicTypeEnum<'ctx>>(),
			],
		)?;

		self.functions.insert_vector_gather(vec_type, fn_value);

		Ok(fn_value)
	}

	fn into_parts(self) -> (Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine) {
		(self.module, self.functions, self.target_machine)
	}
}

unsafe impl<'ctx> AsContextRef<'ctx> for InnerAssembler<'ctx> {
	fn as_ctx_ref(&self) -> LLVMContextRef {
		self.module.get_context().as_ctx_ref()
	}
}

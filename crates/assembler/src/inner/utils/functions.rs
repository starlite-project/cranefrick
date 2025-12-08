use frick_spec::POINTER_SIZE;
use frick_utils::Convert as _;
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	context::{AsContextRef, Context},
	intrinsics::Intrinsic,
	llvm_sys::prelude::LLVMContextRef,
	module::{Linkage, Module},
	types::{BasicMetadataTypeEnum, BasicTypeEnum},
	values::FunctionValue,
};

use crate::{AssemblyError, ContextExt as _, ContextGetter as _};

#[derive(Debug, Clone, Copy)]
pub struct AssemblerFunctions<'ctx> {
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub alloc: FunctionValue<'ctx>,
	pub free: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
	pub lifetime: IntrinsicFunctionSet<'ctx>,
}

impl<'ctx> AssemblerFunctions<'ctx> {
	pub fn new(
		context: &'ctx Context,
		module: &Module<'ctx>,
		cpu_name: &str,
		cpu_features: &str,
	) -> Result<Self, AssemblyError> {
		let void_type = context.void_type();
		let i8_type = context.i8_type();
		let ptr_type = context.default_ptr_type();
		let ptr_int_type = context.custom_width_int_type(POINTER_SIZE as u32);

		let getchar_ty = i8_type.fn_type(&[], false);
		let getchar = module.add_function("rust_getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty =
			void_type.fn_type(&[i8_type.convert::<BasicMetadataTypeEnum<'ctx>>()], false);
		let putchar = module.add_function("rust_putchar", putchar_ty, Some(Linkage::External));

		let alloc_ty = ptr_type.fn_type(
			&[ptr_int_type.convert::<BasicMetadataTypeEnum<'ctx>>()],
			false,
		);
		let alloc = module.add_function("rust_alloc", alloc_ty, Some(Linkage::External));

		let free_ty =
			void_type.fn_type(&[ptr_type.convert::<BasicMetadataTypeEnum<'ctx>>()], false);
		let free = module.add_function("rust_free", free_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, None);

		let lifetime = {
			let lifetime_start = get_intrinsic_function_from_name(
				"llvm.lifetime.start",
				module,
				&[ptr_type.convert::<BasicTypeEnum<'ctx>>()],
			)?;
			let lifetime_end = get_intrinsic_function_from_name(
				"llvm.lifetime.end",
				module,
				&[ptr_type.convert::<BasicTypeEnum<'ctx>>()],
			)?;

			IntrinsicFunctionSet::new(lifetime_start, lifetime_end)
		};

		let this = Self {
			getchar,
			putchar,
			alloc,
			free,
			main,
			lifetime,
		};

		this.setup(cpu_name, cpu_features);

		Ok(this)
	}

	fn setup(self, cpu_name: &str, cpu_features: &str) {
		const ARG_READ: u64 = 0b0001;
		const ARG_WRITE: u64 = 0b0010;
		const INACCESSABLE_READ: u64 = 0b0100;
		const INACCESSABLE_WRITE: u64 = 0b1000;

		let context = self.context();

		let nocallback_attr = context.create_named_enum_attribute("nocallback", 0b0);
		let nofree_attr = context.create_named_enum_attribute("nofree", 0b0);
		let norecurse_attr = context.create_named_enum_attribute("norecurse", 0b0);
		let willreturn_attr = context.create_named_enum_attribute("willreturn", 0b0);
		let arg_none_inaccessable_read_memory_attr =
			context.create_named_enum_attribute("memory", INACCESSABLE_READ);
		let zeroext_attr = context.create_named_enum_attribute("zeroext", 0b0);
		let arg_read_inaccessable_write_memory_attr =
			context.create_named_enum_attribute("memory", ARG_READ | INACCESSABLE_WRITE);
		let noundef_attr = context.create_named_enum_attribute("noundef", 0b0);
		let nounwind_attr = context.create_named_enum_attribute("nounwind", 0b0);
		let target_cpu_attr = context.create_string_attribute("target-cpu", cpu_name);
		let target_cpu_features_attr =
			context.create_string_attribute("target-features", cpu_features);
		let probe_stack_attr = context.create_string_attribute("probe-stack", "inline-asm");
		let alloc_family_attr = context.create_string_attribute("alloc-family", "alloc");
		let allockind_alloc_attr = context.create_named_enum_attribute("allockind", 0b0001_0001);
		let allockind_free_attr = context.create_named_enum_attribute("allockind", 0b0100);
		let noalias_attr = context.create_named_enum_attribute("noalias", 0b0);
		let arg_none_inaccessable_readwrite_memory_attr =
			context.create_named_enum_attribute("memory", INACCESSABLE_READ | INACCESSABLE_WRITE);
		let allocptr_attr = context.create_named_enum_attribute("allocptr", 0b0);
		let arg_readwrite_inaccessable_readwrite_memory_attr = context.create_named_enum_attribute(
			"memory",
			ARG_READ | ARG_WRITE | INACCESSABLE_READ | INACCESSABLE_WRITE,
		);

		tracing::info!(?allockind_free_attr);

		add_attributes_to(
			self.putchar,
			[
				AppliedAttribute::Function(nocallback_attr),
				AppliedAttribute::Function(nofree_attr),
				AppliedAttribute::Function(norecurse_attr),
				AppliedAttribute::Function(willreturn_attr),
				AppliedAttribute::Function(arg_read_inaccessable_write_memory_attr),
				AppliedAttribute::Function(probe_stack_attr),
				AppliedAttribute::Function(target_cpu_attr),
				AppliedAttribute::Function(target_cpu_features_attr),
				AppliedAttribute::Function(nounwind_attr),
				AppliedAttribute::Param(0, zeroext_attr),
				AppliedAttribute::Param(0, noundef_attr),
			],
		);
		add_attributes_to(
			self.getchar,
			[
				AppliedAttribute::Function(nocallback_attr),
				AppliedAttribute::Function(nofree_attr),
				AppliedAttribute::Function(norecurse_attr),
				AppliedAttribute::Function(willreturn_attr),
				AppliedAttribute::Function(arg_none_inaccessable_read_memory_attr),
				AppliedAttribute::Function(probe_stack_attr),
				AppliedAttribute::Function(target_cpu_attr),
				AppliedAttribute::Function(target_cpu_features_attr),
				AppliedAttribute::Function(nounwind_attr),
				AppliedAttribute::Return(zeroext_attr),
				AppliedAttribute::Return(noundef_attr),
			],
		);
		add_attributes_to(
			self.alloc,
			[
				AppliedAttribute::Function(nounwind_attr),
				AppliedAttribute::Function(probe_stack_attr),
				AppliedAttribute::Function(target_cpu_attr),
				AppliedAttribute::Function(target_cpu_features_attr),
				AppliedAttribute::Function(alloc_family_attr),
				AppliedAttribute::Function(allockind_alloc_attr),
				AppliedAttribute::Function(willreturn_attr),
				AppliedAttribute::Function(nofree_attr),
				AppliedAttribute::Function(arg_none_inaccessable_readwrite_memory_attr),
				AppliedAttribute::Return(noalias_attr),
				AppliedAttribute::Return(noundef_attr),
			],
		);
		add_attributes_to(
			self.free,
			[
				AppliedAttribute::Function(nounwind_attr),
				AppliedAttribute::Function(probe_stack_attr),
				AppliedAttribute::Function(target_cpu_attr),
				AppliedAttribute::Function(target_cpu_features_attr),
				AppliedAttribute::Function(alloc_family_attr),
				AppliedAttribute::Function(allockind_free_attr),
				AppliedAttribute::Function(willreturn_attr),
				AppliedAttribute::Function(arg_readwrite_inaccessable_readwrite_memory_attr),
				AppliedAttribute::Param(0, allocptr_attr),
				AppliedAttribute::Param(0, noundef_attr),
			],
		);
		add_attributes_to(
			self.main,
			[
				nofree_attr,
				probe_stack_attr,
				target_cpu_attr,
				target_cpu_features_attr,
				nounwind_attr,
			]
			.map(AppliedAttribute::Function),
		);
	}
}

unsafe impl<'ctx> AsContextRef<'ctx> for AssemblerFunctions<'ctx> {
	fn as_ctx_ref(&self) -> LLVMContextRef {
		self.main.get_type().get_context().as_ctx_ref()
	}
}

#[derive(Debug, Clone, Copy)]
pub struct IntrinsicFunctionSet<'ctx> {
	pub start: FunctionValue<'ctx>,
	pub end: FunctionValue<'ctx>,
}

impl<'ctx> IntrinsicFunctionSet<'ctx> {
	const fn new(start: FunctionValue<'ctx>, end: FunctionValue<'ctx>) -> Self {
		Self { start, end }
	}
}

#[tracing::instrument(skip(module, types))]
pub fn get_intrinsic_function_from_name<'ctx>(
	name: &'static str,
	module: &Module<'ctx>,
	types: &[BasicTypeEnum<'ctx>],
) -> Result<FunctionValue<'ctx>, AssemblyError> {
	let intrinsic =
		Intrinsic::find(name).ok_or_else(|| AssemblyError::intrinsic_not_found(name))?;

	let declaration = intrinsic
		.get_declaration(module, types)
		.ok_or_else(|| AssemblyError::invalid_intrinsic_declaration(name))?;

	tracing::debug!(%declaration);

	Ok(declaration)
}

fn add_attributes_to<const N: usize>(func: FunctionValue<'_>, attrs: [AppliedAttribute; N]) {
	for attribute in attrs {
		match attribute {
			AppliedAttribute::Function(attr) => func.add_attribute(AttributeLoc::Function, attr),
			AppliedAttribute::Param(idx, attr) => {
				func.add_attribute(AttributeLoc::Param(idx), attr);
			}
			AppliedAttribute::Return(attr) => func.add_attribute(AttributeLoc::Return, attr),
		}
	}
}

enum AppliedAttribute {
	Function(Attribute),
	Param(u32, Attribute),
	Return(Attribute),
}

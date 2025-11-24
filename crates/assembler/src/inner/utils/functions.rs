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

#[derive(Debug, Clone)]
pub struct AssemblerFunctions<'ctx> {
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
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
		let i32_type = context.i32_type();
		let ptr_type = context.default_ptr_type();

		let getchar_ty = i32_type.fn_type(&[], false);
		let getchar = module.add_function("rust_getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty =
			void_type.fn_type(&[i8_type.convert::<BasicMetadataTypeEnum<'ctx>>()], false);
		let putchar = module.add_function("rust_putchar", putchar_ty, Some(Linkage::External));

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
			main,
			lifetime,
		};

		this.setup(cpu_name, cpu_features);

		Ok(this)
	}

	fn setup(&self, cpu_name: &str, cpu_features: &str) {
		let context = self.context();

		let nocallback_attr = context.create_named_enum_attribute("nocallback", 0);
		let nofree_attr = context.create_named_enum_attribute("nofree", 0);
		let norecurse_attr = context.create_named_enum_attribute("norecurse", 0);
		let willreturn_attr = context.create_named_enum_attribute("willreturn", 0);
		let arg_none_inaccessable_read_memory_attr =
			context.create_named_enum_attribute("memory", 4);
		let zeroext_attr = context.create_named_enum_attribute("zeroext", 0);
		let arg_read_inaccessable_write_memory_attr =
			context.create_named_enum_attribute("memory", 9);
		let noundef_attr = context.create_named_enum_attribute("noundef", 0);
		let nounwind_attr = context.create_named_enum_attribute("nounwind", 0);
		let target_cpu_attr = context.create_string_attribute("target-cpu", cpu_name);
		let target_cpu_features_attr =
			context.create_string_attribute("target-features", cpu_features);
		let probe_stack_attr = context.create_string_attribute("probe-stack", "inline-asm");

		add_attributes_to(
			self.putchar,
			[
				nocallback_attr,
				nofree_attr,
				norecurse_attr,
				willreturn_attr,
				arg_read_inaccessable_write_memory_attr,
				probe_stack_attr,
				target_cpu_attr,
				target_cpu_features_attr,
				nounwind_attr,
			],
			[(0, zeroext_attr), (0, noundef_attr)],
			[],
		);
		add_attributes_to(
			self.getchar,
			[
				nocallback_attr,
				nofree_attr,
				norecurse_attr,
				willreturn_attr,
				arg_none_inaccessable_read_memory_attr,
				probe_stack_attr,
				target_cpu_attr,
				target_cpu_features_attr,
				nounwind_attr,
			],
			[],
			[zeroext_attr, noundef_attr],
		);
		add_attributes_to(
			self.main,
			[
				nofree_attr,
				probe_stack_attr,
				target_cpu_attr,
				target_cpu_features_attr,
				nounwind_attr,
			],
			[],
			[],
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

fn add_attributes_to(
	func: FunctionValue<'_>,
	func_attrs: impl IntoIterator<Item = Attribute>,
	param_attrs: impl IntoIterator<Item = (u32, Attribute)>,
	return_attrs: impl IntoIterator<Item = Attribute>,
) {
	for attribute in func_attrs {
		func.add_attribute(AttributeLoc::Function, attribute);
	}

	for (param_idx, attribute) in param_attrs {
		func.add_attribute(AttributeLoc::Param(param_idx), attribute);
	}

	for attribute in return_attrs {
		func.add_attribute(AttributeLoc::Return, attribute);
	}
}

use inkwell::{
	attributes::{Attribute, AttributeLoc},
	context::{AsContextRef, Context},
	intrinsics::Intrinsic,
	llvm_sys::prelude::LLVMContextRef,
	module::{Linkage, Module},
	types::BasicTypeEnum,
	values::FunctionValue,
};

use crate::{AssemblyError, ContextExt as _, ContextGetter as _};

#[derive(Debug, Clone, Copy)]
pub struct AssemblerFunctions<'ctx> {
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
	pub puts: FunctionValue<'ctx>,
	pub lifetime: IntrinsicFunctionSet<'ctx>,
	pub invariant: IntrinsicFunctionSet<'ctx>,
	pub assume: FunctionValue<'ctx>,
	pub eh_personality: FunctionValue<'ctx>,
}

impl<'ctx> AssemblerFunctions<'ctx> {
	pub fn new(context: &'ctx Context, module: &Module<'ctx>) -> Result<Self, AssemblyError> {
		let void_type = context.void_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let ptr_type = context.default_ptr_type();

		let getchar_ty = i32_type.fn_type(&[], false);
		let getchar = module.add_function("rust_getchar", getchar_ty, Some(Linkage::External));

		let putchar_ty = void_type.fn_type(&[i32_type.into()], false);
		let putchar = module.add_function("rust_putchar", putchar_ty, Some(Linkage::External));

		let main_ty = void_type.fn_type(&[], false);
		let main = module.add_function("main", main_ty, None);

		let puts_ty = void_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
		let puts = module.add_function("frick_puts", puts_ty, Some(Linkage::Private));

		let lifetime = {
			let lifetime_start = get_intrinsic_function_from_name(
				"llvm.lifetime.start",
				module,
				&[ptr_type.into()],
			)?;
			let lifetime_end =
				get_intrinsic_function_from_name("llvm.lifetime.end", module, &[ptr_type.into()])?;

			IntrinsicFunctionSet::new(lifetime_start, lifetime_end)
		};

		let invariant = {
			let invariant_start = get_intrinsic_function_from_name(
				"llvm.invariant.start",
				module,
				&[ptr_type.into()],
			)?;

			let invariant_end =
				get_intrinsic_function_from_name("llvm.invariant.end", module, &[ptr_type.into()])?;

			IntrinsicFunctionSet::new(invariant_start, invariant_end)
		};

		let assume = get_intrinsic_function_from_name("llvm.assume", module, &[])?;

		let eh_personality_ty = i32_type.fn_type(
			&[
				i32_type.into(),
				i32_type.into(),
				i64_type.into(),
				ptr_type.into(),
				ptr_type.into(),
			],
			false,
		);
		let eh_personality = module.add_function(
			"rust_eh_personality",
			eh_personality_ty,
			Some(Linkage::External),
		);

		let this = Self {
			getchar,
			putchar,
			main,
			puts,
			lifetime,
			invariant,
			assume,
			eh_personality,
		};

		this.setup();

		Ok(this)
	}

	fn setup(self) {
		let context = self.context();

		self.main.set_personality_function(self.eh_personality);
		self.puts.set_personality_function(self.eh_personality);

		let nocallback_attr = context.create_named_enum_attribute("nocallback", 0);
		let nofree_attr = context.create_named_enum_attribute("nofree", 0);
		let norecurse_attr = context.create_named_enum_attribute("norecurse", 0);
		let willreturn_attr = context.create_named_enum_attribute("willreturn", 0);
		let nosync_attr = context.create_named_enum_attribute("nosync", 0);
		let arg_none_inaccessable_read_memory_attr =
			context.create_named_enum_attribute("memory", 4);
		let zeroext_attr = context.create_named_enum_attribute("zeroext", 0);
		let arg_read_inaccessable_write_memory_attr =
			context.create_named_enum_attribute("memory", 9);
		let uwtable_attr = context.create_named_enum_attribute("uwtable", 2);
		let noalias_attr = context.create_named_enum_attribute("noalias", 0);
		let nonnull_attr = context.create_named_enum_attribute("nonnull", 0);
		let readonly_attr = context.create_named_enum_attribute("readonly", 0);
		let align_1_attr = context.create_named_enum_attribute("align", 1);
		let noundef_attr = context.create_named_enum_attribute("noundef", 0);
		let nounwind_attr = context.create_named_enum_attribute("nounwind", 0);
		let nonlazybind_attr = context.create_named_enum_attribute("nonlazybind", 0);

		add_attributes_to(
			self.putchar,
			[
				nocallback_attr,
				nofree_attr,
				norecurse_attr,
				willreturn_attr,
				nosync_attr,
				arg_read_inaccessable_write_memory_attr,
				uwtable_attr,
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
				nosync_attr,
				arg_none_inaccessable_read_memory_attr,
			],
			[],
			[zeroext_attr],
		);
		add_attributes_to(
			self.puts,
			[
				nofree_attr,
				norecurse_attr,
				willreturn_attr,
				nosync_attr,
				arg_read_inaccessable_write_memory_attr,
				uwtable_attr,
			],
			[
				(0, noalias_attr),
				(0, nofree_attr),
				(0, nonnull_attr),
				(0, readonly_attr),
				(0, align_1_attr),
			],
			[],
		);
		add_attributes_to(self.main, [nosync_attr, nofree_attr, uwtable_attr], [], []);
		add_attributes_to(
			self.eh_personality,
			[nounwind_attr, nonlazybind_attr, uwtable_attr],
			(0..5).map(|i| (i, noundef_attr)),
			[noundef_attr],
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
fn get_intrinsic_function_from_name<'ctx>(
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

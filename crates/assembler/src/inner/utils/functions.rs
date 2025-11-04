use std::{cell::RefCell, collections::HashMap};

use frick_utils::Convert as _;
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	context::{AsContextRef, Context},
	intrinsics::Intrinsic,
	llvm_sys::prelude::LLVMContextRef,
	module::{Linkage, Module},
	types::{BasicMetadataTypeEnum, BasicTypeEnum, VectorType},
	values::FunctionValue,
};

use crate::{AssemblyError, ContextExt as _, ContextGetter as _};

#[derive(Debug, Clone)]
pub struct AssemblerFunctions<'ctx> {
	pub getchar: FunctionValue<'ctx>,
	pub putchar: FunctionValue<'ctx>,
	pub main: FunctionValue<'ctx>,
	pub lifetime: IntrinsicFunctionSet<'ctx>,
	pub invariant: IntrinsicFunctionSet<'ctx>,
	pub eh_personality: FunctionValue<'ctx>,
	masked_vector_functions: RefCell<HashMap<VectorKey, FunctionValue<'ctx>>>,
}

impl<'ctx> AssemblerFunctions<'ctx> {
	pub fn new(context: &'ctx Context, module: &Module<'ctx>) -> Result<Self, AssemblyError> {
		let void_type = context.void_type();
		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
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

		let invariant = {
			let invariant_start = get_intrinsic_function_from_name(
				"llvm.invariant.start",
				module,
				&[ptr_type.convert::<BasicTypeEnum<'ctx>>()],
			)?;

			let invariant_end = get_intrinsic_function_from_name(
				"llvm.invariant.end",
				module,
				&[ptr_type.convert::<BasicTypeEnum<'ctx>>()],
			)?;

			IntrinsicFunctionSet::new(invariant_start, invariant_end)
		};

		let eh_personality_ty = i32_type.fn_type(
			&[
				i32_type.convert::<BasicMetadataTypeEnum<'ctx>>(),
				i32_type.convert::<BasicMetadataTypeEnum<'ctx>>(),
				i64_type.convert::<BasicMetadataTypeEnum<'ctx>>(),
				ptr_type.convert::<BasicMetadataTypeEnum<'ctx>>(),
				ptr_type.convert::<BasicMetadataTypeEnum<'ctx>>(),
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
			lifetime,
			invariant,
			eh_personality,
			masked_vector_functions: RefCell::default(),
		};

		this.setup();

		Ok(this)
	}

	pub fn get_vector_scatter(&self, vec_type: VectorType<'ctx>) -> Option<FunctionValue<'ctx>> {
		self.get_vector_function(vec_type, VectorFunctionType::Scatter)
	}

	pub fn insert_vector_scatter(&self, vec_type: VectorType<'ctx>, fn_value: FunctionValue<'ctx>) {
		self.insert_vector_function(vec_type, VectorFunctionType::Scatter, fn_value);
	}

	pub fn get_vector_gather(&self, vec_type: VectorType<'ctx>) -> Option<FunctionValue<'ctx>> {
		self.get_vector_function(vec_type, VectorFunctionType::Gather)
	}

	pub fn insert_vector_gather(&self, vec_type: VectorType<'ctx>, fn_value: FunctionValue<'ctx>) {
		self.insert_vector_function(vec_type, VectorFunctionType::Gather, fn_value);
	}

	fn get_vector_function(
		&self,
		vec_type: VectorType<'ctx>,
		fn_type: VectorFunctionType,
	) -> Option<FunctionValue<'ctx>> {
		let key = VectorKey::new(vec_type, fn_type)?;

		self.masked_vector_functions.borrow().get(&key).copied()
	}

	fn insert_vector_function(
		&self,
		vec_type: VectorType<'ctx>,
		fn_type: VectorFunctionType,
		fn_value: FunctionValue<'ctx>,
	) {
		let Some(key) = VectorKey::new(vec_type, fn_type) else {
			return;
		};

		self.masked_vector_functions
			.borrow_mut()
			.insert(key, fn_value);
	}

	fn setup(&self) {
		let context = self.context();

		self.main.set_personality_function(self.eh_personality);

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
		let noundef_attr = context.create_named_enum_attribute("noundef", 0);
		let nounwind_attr = context.create_named_enum_attribute("nounwind", 0);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VectorKey {
	bit_width: u32,
	size_of_vec: u32,
	fn_type: VectorFunctionType,
}

impl VectorKey {
	fn new(vec: VectorType<'_>, fn_type: VectorFunctionType) -> Option<Self> {
		let bit_width = match vec.get_element_type() {
			BasicTypeEnum::IntType(ty) => ty.get_bit_width(),
			_ => return None,
		};

		let size_of_vec = vec.get_size();

		Some(Self {
			bit_width,
			size_of_vec,
			fn_type,
		})
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VectorFunctionType {
	Gather,
	Scatter,
}

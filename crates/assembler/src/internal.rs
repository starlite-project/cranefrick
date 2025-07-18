use cranelift::{
	codegen::ir::{Inst, immediates::Offset32},
	prelude::*,
};

pub trait InstBuilderExt<'f>: InstBuilder<'f> {
	fn store_imm<T1, T2, T3>(
		&mut self,
		mem_flags: T1,
		ty: Type,
		x: Value,
		p: T2,
		offset: T3,
	) -> Inst
	where
		T1: Into<MemFlags>,
		T2: Into<Imm64>,
		T3: Into<Offset32>;
}

impl<'f, T> InstBuilderExt<'f> for T
where
	T: InstBuilder<'f>,
{
	fn store_imm<T1, T2, T3>(
		&mut self,
		mem_flags: T1,
		ty: Type,
		x: Value,
		p: T2,
		offset: T3,
	) -> Inst
	where
		T1: Into<MemFlags>,
		T2: Into<Imm64>,
		T3: Into<Offset32>,
	{
		let immediate = self.iconst(ty, p);
		self.store(mem_flags, x, immediate, offset)
	}
}

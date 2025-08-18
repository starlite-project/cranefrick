use cranelift_codegen::{
	binemit::CodeOffset,
	ir::{SourceLoc, TrapCode},
};

#[derive(Debug, Clone)]
pub struct TrapSite {
	pub offset: CodeOffset,
	pub srcloc: SourceLoc,
	pub code: TrapCode,
}

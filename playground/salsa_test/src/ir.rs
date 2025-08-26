use annotate_snippets::{Level, Renderer, Snippet};
use ordered_float::OrderedFloat;

#[salsa::input(debug)]
pub struct SourceProgram {
	#[returns(ref)]
	pub text: String,
}

#[salsa::interned(debug)]
pub struct VariableId<'db> {
	#[returns(ref)]
	pub text: String,
}

#[salsa::interned(debug)]
pub struct FunctionId<'db> {
	#[returns(ref)]
	pub text: String,
}

#[salsa::tracked(debug)]
pub struct Program<'db> {
	#[tracked]
	#[returns(ref)]
	pub statements: Vec<Statement<'db>>,
}

#[derive(Debug, PartialEq, Eq, Hash, salsa::Update)]
pub struct Statement<'db> {
	pub span: Span<'db>,
	pub data: StatementData<'db>,
}

impl<'db> Statement<'db> {
	#[must_use]
	pub const fn new(span: Span<'db>, data: StatementData<'db>) -> Self {
		Self { span, data }
	}
}

#[derive(Debug, PartialEq, Eq, Hash, salsa::Update)]
pub enum StatementData<'db> {
	Function(Function<'db>),
	Print(Expression<'db>),
}

#[derive(Debug, PartialEq, Eq, Hash, salsa::Update)]
pub struct Expression<'db> {
	pub span: Span<'db>,
	pub data: ExpressionData<'db>,
}

impl<'db> Expression<'db> {
	#[must_use]
	pub const fn new(span: Span<'db>, data: ExpressionData<'db>) -> Self {
		Self { span, data }
	}
}

#[derive(Debug, PartialEq, Eq, Hash, salsa::Update)]
pub enum ExpressionData<'db> {
	Op(Box<Expression<'db>>, Op, Box<Expression<'db>>),
	Number(OrderedFloat<f64>),
	Variable(VariableId<'db>),
	Call(FunctionId<'db>, Vec<Expression<'db>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
	Add,
	Subtract,
	Multiply,
	Divide,
}

#[salsa::tracked(debug)]
pub struct Span<'db> {
	#[tracked]
	pub start: usize,
	#[tracked]
	pub end: usize,
}

#[salsa::tracked(debug)]
pub struct Function<'db> {
	pub name: FunctionId<'db>,

	name_span: Span<'db>,

	#[tracked]
	#[returns(ref)]
	pub args: Vec<VariableId<'db>>,

	#[tracked]
	#[returns(ref)]
	pub body: Expression<'db>,
}

#[salsa::accumulator]
#[derive(Debug)]
#[allow(dead_code)] // Debug impl uses them
pub struct Diagnostic {
	pub start: usize,
	pub end: usize,
	pub message: String,
}

impl Diagnostic {
	#[must_use]
	pub const fn new(start: usize, end: usize, message: String) -> Self {
		Self {
			start,
			end,
			message,
		}
	}

	pub fn render(&self, db: &dyn salsa::Database, src: SourceProgram) -> String {
		let line_start = src.text(db)[..self.start].lines().count() + 1;
		Renderer::plain()
			.render(
				Level::Error.title(&self.message).snippet(
					Snippet::source(src.text(db))
						.line_start(line_start)
						.origin("input")
						.fold(true)
						.annotation(Level::Error.span(self.start..self.end).label("here")),
				),
			)
			.to_string()
	}
}

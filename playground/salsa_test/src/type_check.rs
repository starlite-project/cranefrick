use salsa::Accumulator;

use super::ir::{
	Diagnostic, Expression, ExpressionData, Function, FunctionId, Program, Span, StatementData,
	VariableId,
};

#[salsa::tracked]
pub fn type_check_program<'db>(db: &'db dyn salsa::Database, program: Program<'db>) {
	for statement in program.statements(db) {
		match &statement.data {
			StatementData::Function(f) => type_check_function(db, *f, program),
			StatementData::Print(e) => CheckExpression::new(db, program, &[]).check(e),
		}
	}
}

#[salsa::tracked]
pub fn type_check_function<'db>(
	db: &'db dyn salsa::Database,
	function: Function<'db>,
	program: Program<'db>,
) {
	CheckExpression::new(db, program, function.args(db)).check(function.body(db));
}

#[salsa::tracked]
pub fn find_function<'db>(
	db: &'db dyn salsa::Database,
	program: Program<'db>,
	name: FunctionId<'db>,
) -> Option<Function<'db>> {
	program.statements(db).iter().find_map(|s| match &s.data {
		StatementData::Function(f) if f.name(db) == name => Some(*f),
		_ => None,
	})
}

struct CheckExpression<'input, 'db> {
	db: &'db dyn salsa::Database,
	program: Program<'db>,
	names_in_scope: &'input [VariableId<'db>],
}

impl<'input, 'db> CheckExpression<'input, 'db> {
	fn new(
		db: &'db dyn salsa::Database,
		program: Program<'db>,
		names_in_scope: &'input [VariableId<'db>],
	) -> Self {
		Self {
			db,
			program,
			names_in_scope,
		}
	}

	fn check(&self, expression: &Expression<'db>) {
		match &expression.data {
			ExpressionData::Op(left, .., right) => {
				self.check(left);
				self.check(right);
			}
			ExpressionData::Number(..) => {}
			ExpressionData::Variable(v) => {
				if !self.names_in_scope.contains(v) {
					self.report_error(
						expression.span,
						format!("the variable `{}` is not declared", v.text(self.db)),
					);
				}
			}
			ExpressionData::Call(f, args) => {
				if self.find_function(*f).is_none() {
					self.report_error(
						expression.span,
						format!("the function `{}` is not declared", f.text(self.db)),
					);
				}

				for arg in args {
					self.check(arg);
				}
			}
		}
	}

	fn find_function(&self, f: FunctionId<'db>) -> Option<Function<'db>> {
		find_function(self.db, self.program, f)
	}

	fn report_error(&self, span: Span<'db>, message: String) {
		Diagnostic::new(span.start(self.db), span.end(self.db), message).accumulate(self.db);
	}
}

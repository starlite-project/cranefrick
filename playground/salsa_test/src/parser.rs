use std::fmt::Debug;

use ordered_float::OrderedFloat;
use salsa::Accumulator;

use super::ir::{
	Diagnostic, Expression, ExpressionData, Function, FunctionId, Op, Program, SourceProgram, Span,
	Statement, StatementData, VariableId,
};

#[salsa::tracked]
pub fn parse_statements(db: &dyn salsa::Database, source: SourceProgram) -> Program<'_> {
	let source_text = source.text(db);

	let mut parser = Parser {
		db,
		source_text,
		position: 0,
	};

	let mut result = Vec::new();

	loop {
		parser.skip_whitespace();

		if parser.peek().is_none() {
			break;
		}

		if let Some(statement) = parser.parse_statement() {
			result.push(statement);
		} else {
			parser.report_error();
			break;
		}
	}

	Program::new(db, result)
}

struct Parser<'source, 'db> {
	db: &'db dyn salsa::Database,
	source_text: &'source str,
	position: usize,
}

impl<'db> Parser<'_, 'db> {
	fn probe<T: Debug>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
		let p = self.position;
		if let Some(v) = f(self) {
			Some(v)
		} else {
			self.position = p;
			None
		}
	}

	fn report_error(&self) {
		let next_position = match self.peek() {
			Some(ch) => self.position + ch.len_utf8(),
			None => self.position,
		};

		Diagnostic::new(
			self.position,
			next_position,
			"unexpected character".to_owned(),
		)
		.accumulate(self.db);
	}

	fn peek(&self) -> Option<char> {
		self.source_text[self.position..].chars().next()
	}

	fn span_from(&self, start_position: usize) -> Span<'db> {
		Span::new(self.db, start_position, self.position)
	}

	fn consume(&mut self, ch: char) {
		debug_assert_eq!(self.peek(), Some(ch));

		self.position += ch.len_utf8();
	}

	fn skip_whitespace(&mut self) -> usize {
		while let Some(ch) = self.peek() {
			if ch.is_whitespace() {
				self.consume(ch);
			} else {
				break;
			}
		}

		self.position
	}

	fn parse_statement(&mut self) -> Option<Statement<'db>> {
		let start_position = self.skip_whitespace();
		let word = self.word()?;
		match word.as_str() {
			"fn" => {
				let func = self.parse_function()?;
				Some(Statement::new(
					self.span_from(start_position),
					StatementData::Function(func),
				))
			}
			"print" => {
				let expr = self.parse_expression()?;
				Some(Statement::new(
					self.span_from(start_position),
					StatementData::Print(expr),
				))
			}
			_ => None,
		}
	}

	fn parse_function(&mut self) -> Option<Function<'db>> {
		let start_position = self.skip_whitespace();
		let name = self.word()?;
		let name_span = self.span_from(start_position);
		let name = FunctionId::new(self.db, name);
		self.ch('(')?;
		let args = self.parameters()?;
		self.ch(')')?;
		self.ch('=')?;
		let body = self.parse_expression()?;
		Some(Function::new(self.db, name, name_span, args, body))
	}

	fn parse_expression(&mut self) -> Option<Expression<'db>> {
		self.parse_op_expression(Self::parse_expression1, Self::low_op)
	}

	fn parse_expression1(&mut self) -> Option<Expression<'db>> {
		self.parse_op_expression(Self::parse_expression2, Self::high_op)
	}

	fn parse_op_expression(
		&mut self,
		mut parse_expr: impl FnMut(&mut Self) -> Option<Expression<'db>>,
		mut op: impl FnMut(&mut Self) -> Option<Op>,
	) -> Option<Expression<'db>> {
		let start_position = self.skip_whitespace();
		let mut expr1 = parse_expr(self)?;

		while let Some(op) = op(self) {
			let expr2 = parse_expr(self)?;
			expr1 = Expression::new(
				self.span_from(start_position),
				ExpressionData::Op(Box::new(expr1), op, Box::new(expr2)),
			);
		}

		Some(expr1)
	}

	fn parse_expression2(&mut self) -> Option<Expression<'db>> {
		let start_position = self.skip_whitespace();
		if let Some(w) = self.word() {
			if self.ch('(').is_some() {
				let f = FunctionId::new(self.db, w);
				let args = self.parse_expressions()?;
				self.ch(')')?;
				return Some(Expression::new(
					self.span_from(start_position),
					ExpressionData::Call(f, args),
				));
			}

			let v = VariableId::new(self.db, w);
			Some(Expression::new(
				self.span_from(start_position),
				ExpressionData::Variable(v),
			))
		} else if let Some(n) = self.number() {
			Some(Expression::new(
				self.span_from(start_position),
				ExpressionData::Number(OrderedFloat::from(n)),
			))
		} else if self.ch('(').is_some() {
			let expr = self.parse_expression()?;
			self.ch(')')?;
			Some(expr)
		} else {
			None
		}
	}

	fn low_op(&mut self) -> Option<Op> {
		if self.ch('+').is_some() {
			Some(Op::Add)
		} else if self.ch('-').is_some() {
			Some(Op::Subtract)
		} else {
			None
		}
	}

	fn high_op(&mut self) -> Option<Op> {
		if self.ch('*').is_some() {
			Some(Op::Multiply)
		} else if self.ch('/').is_some() {
			Some(Op::Divide)
		} else {
			None
		}
	}

	fn parse_expressions(&mut self) -> Option<Vec<Expression<'db>>> {
		let mut r = Vec::new();

		loop {
			let expr = self.parse_expression()?;
			r.push(expr);
			if self.ch(',').is_none() {
				break Some(r);
			}
		}
	}

	fn parameters(&mut self) -> Option<Vec<VariableId<'db>>> {
		let mut r = Vec::new();

		loop {
			let name = self.word()?;
			let vid = VariableId::new(self.db, name);
			r.push(vid);

			if self.ch(',').is_none() {
				break Some(r);
			}
		}
	}

	fn ch(&mut self, c: char) -> Option<Span<'db>> {
		let start_position = self.skip_whitespace();
		let p = self.peek()?;

		if c == p {
			self.consume(c);
			Some(self.span_from(start_position))
		} else {
			None
		}
	}

	fn word(&mut self) -> Option<String> {
		self.skip_whitespace();

		let mut s = String::new();
		while let Some(ch) = self.peek() {
			if ch.is_alphabetic() || matches!(ch, '_') || (!s.is_empty() && ch.is_numeric()) {
				s.push(ch);
			} else {
				break;
			}
			self.consume(ch);
		}

		if s.is_empty() { None } else { Some(s) }
	}

	fn number(&mut self) -> Option<f64> {
		self.skip_whitespace();

		self.probe(|this| {
			let mut s = String::new();
			while let Some(ch) = this.peek() {
				if ch.is_numeric() || matches!(ch, '.') {
					s.push(ch);
				} else {
					break;
				}

				this.consume(ch);
			}

			if s.is_empty() {
				None
			} else {
				str::parse(&s).ok()
			}
		})
	}
}

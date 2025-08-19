use std::{
	collections::HashMap,
	fmt::{Display, Formatter, Result as FmtResult},
};

use chumsky::{input::ValueInput, prelude::*};

pub type Span = SimpleSpan;
pub type Spanned<T> = (T, Span);

#[derive(Debug)]
pub struct Error {
	pub span: Span,
	pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Func<'src> {
	pub args: Vec<&'src str>,
	pub span: Span,
	pub body: Spanned<Expr<'src>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token<'src> {
	Null,
	Bool(bool),
	Num(f64),
	Str(&'src str),
	Op(&'src str),
	Ctrl(char),
	Ident(&'src str),
	Fn,
	Let,
	Print,
	If,
	Else,
}

impl Display for Token<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Null => write!(f, "null"),
			Self::Bool(x) => write!(f, "{x}"),
			Self::Num(n) => write!(f, "{n}"),
			Self::Str(s) | Self::Op(s) | Self::Ident(s) => write!(f, "{s}"),
			Self::Ctrl(c) => write!(f, "{c}"),
			Self::Fn => write!(f, "fn"),
			Self::Let => write!(f, "let"),
			Self::Print => write!(f, "print"),
			Self::If => write!(f, "if"),
			Self::Else => write!(f, "else"),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value<'src> {
	Null,
	Bool(bool),
	Num(f64),
	Str(&'src str),
	List(Vec<Self>),
	Func(&'src str),
}

impl Value<'_> {
	fn num(self, span: Span) -> Result<f64, Error> {
		if let Self::Num(x) = self {
			Ok(x)
		} else {
			Err(Error {
				span,
				message: format!("'{self}' is not a number"),
			})
		}
	}
}

impl Display for Value<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Null => write!(f, "null"),
			Self::Bool(x) => write!(f, "{x}"),
			Self::Num(x) => write!(f, "{x}"),
			Self::Str(x) => write!(f, "{x}"),
			Self::List(xs) => write!(
				f,
				"[{}]",
				xs.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>()
					.join(", ")
			),
			Self::Func(name) => write!(f, "<function: {name}>"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
	Add,
	Sub,
	Mul,
	Div,
	Eq,
	NotEq,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
	Error,
	Value(Value<'src>),
	List(Vec<Spanned<Self>>),
	Local(&'src str),
	Let(&'src str, Box<Spanned<Self>>, Box<Spanned<Self>>),
	Then(Box<Spanned<Self>>, Box<Spanned<Self>>),
	Binary(Box<Spanned<Self>>, BinaryOp, Box<Spanned<Self>>),
	Call(Box<Spanned<Self>>, Spanned<Vec<Spanned<Self>>>),
	If(Box<Spanned<Self>>, Box<Spanned<Self>>, Box<Spanned<Self>>),
	Print(Box<Spanned<Self>>),
}

pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
	let num = text::int(10)
		.then(just('.').then(text::digits(10)).or_not())
		.to_slice()
		.from_str()
		.unwrapped()
		.map(Token::Num);

	let str = just('"')
		.ignore_then(none_of('"').repeated().to_slice())
		.then_ignore(just('"'))
		.map(Token::Str);

	let op = one_of("+*-/!=")
		.repeated()
		.at_least(1)
		.to_slice()
		.map(Token::Op);

	let ctrl = one_of("()[]{};,").map(Token::Ctrl);

	let ident = text::ascii::ident().map(|ident: &str| match ident {
		"fn" => Token::Fn,
		"let" => Token::Let,
		"print" => Token::Print,
		"if" => Token::If,
		"else" => Token::Else,
		"true" => Token::Bool(true),
		"false" => Token::Bool(false),
		"null" => Token::Null,
		_ => Token::Ident(ident),
	});

	let token = num.or(str).or(op).or(ctrl).or(ident);

	let comment = just("//")
		.then(any().and_is(just('\n').not()).repeated())
		.padded();

	token
		.map_with(|tok, e| (tok, e.span()))
		.padded_by(comment.repeated())
		.recover_with(skip_then_retry_until(any().ignored(), end()))
		.repeated()
		.collect()
}

#[must_use]
pub fn expr_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Expr<'src>>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	recursive(|expr| {
		let inline_expr = recursive(|inline_expr| {
			let value = select! {
				Token::Null => Expr::Value(Value::Null),
				Token::Bool(x) => Expr::Value(Value::Bool(x)),
				Token::Num(x) => Expr::Value(Value::Num(x)),
				Token::Str(x) => Expr::Value(Value::Str(x)),
			}
			.labelled("value");

			let ident = select! { Token::Ident(ident) => ident }.labelled("identifier");

			let items = expr
				.clone()
				.separated_by(just(Token::Ctrl(',')))
				.allow_trailing()
				.collect::<Vec<_>>();

			let let_ = just(Token::Let)
				.ignore_then(ident)
				.then_ignore(just(Token::Op("=")))
				.then(inline_expr)
				.then_ignore(just(Token::Ctrl(';')))
				.then(expr.clone())
				.map(|((name, val), body)| Expr::Let(name, Box::new(val), Box::new(body)));

			let list = items
				.clone()
				.map(Expr::List)
				.delimited_by(just(Token::Ctrl('[')), just(Token::Ctrl(']')));

			let atom = value
				.or(ident.map(Expr::Local))
				.or(let_)
				.or(list)
				.or(just(Token::Print)
					.ignore_then(
						expr.clone()
							.delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')'))),
					)
					.map(|expr| Expr::Print(Box::new(expr))))
				.map_with(|expr, e| (expr, e.span()))
				.or(expr
					.clone()
					.delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')'))))
				.recover_with(via_parser(nested_delimiters(
					Token::Ctrl('('),
					Token::Ctrl(')'),
					[
						(Token::Ctrl('['), Token::Ctrl(']')),
						(Token::Ctrl('{'), Token::Ctrl('}')),
					],
					|span| (Expr::Error, span),
				)))
				.recover_with(via_parser(nested_delimiters(
					Token::Ctrl('['),
					Token::Ctrl(']'),
					[
						(Token::Ctrl('('), Token::Ctrl(')')),
						(Token::Ctrl('{'), Token::Ctrl('}')),
					],
					|span| (Expr::Error, span),
				)))
				.boxed();

			let call = atom.foldl_with(
				items
					.delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
					.map_with(|args, e| (args, e.span()))
					.repeated(),
				|f, args, e| (Expr::Call(Box::new(f), args), e.span()),
			);

			let op = just(Token::Op("*"))
				.to(BinaryOp::Mul)
				.or(just(Token::Op("/")).to(BinaryOp::Div));

			let product = call
				.clone()
				.foldl_with(op.then(call).repeated(), |a, (op, b), e| {
					(Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
				});

			let op = just(Token::Op("+"))
				.to(BinaryOp::Add)
				.or(just(Token::Op("-")).to(BinaryOp::Sub));

			let sum = product
				.clone()
				.foldl_with(op.then(product).repeated(), |a, (op, b), e| {
					(Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
				});

			let op = just(Token::Op("=="))
				.to(BinaryOp::Eq)
				.or(just(Token::Op("!=")).to(BinaryOp::NotEq));

			let compare = sum
				.clone()
				.foldl_with(op.then(sum).repeated(), |a, (op, b), e| {
					(Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
				});

			compare.labelled("expression").as_context()
		});

		let block = expr
			.clone()
			.delimited_by(just(Token::Ctrl('{')), just(Token::Ctrl('}')))
			.recover_with(via_parser(nested_delimiters(
				Token::Ctrl('{'),
				Token::Ctrl('}'),
				[
					(Token::Ctrl('('), Token::Ctrl(')')),
					(Token::Ctrl('['), Token::Ctrl(']')),
				],
				|span| (Expr::Error, span),
			)));

		let if_ = recursive(|if_| {
			just(Token::If)
				.ignore_then(expr.clone())
				.then(block.clone())
				.then(
					just(Token::Else)
						.ignore_then(block.clone().or(if_))
						.or_not(),
				)
				.map_with(|((cond, a), b), e| {
					(
						Expr::If(
							Box::new(cond),
							Box::new(a),
							Box::new(b.unwrap_or_else(|| (Expr::Value(Value::Null), e.span()))),
						),
						e.span(),
					)
				})
		});

		let block_expr = block.or(if_);

		let block_chain = block_expr
			.clone()
			.foldl_with(block_expr.clone().repeated(), |a, b, e| {
				(Expr::Then(Box::new(a), Box::new(b)), e.span())
			});

		let block_recovery = nested_delimiters(
			Token::Ctrl('{'),
			Token::Ctrl('}'),
			[
				(Token::Ctrl('('), Token::Ctrl(')')),
				(Token::Ctrl('['), Token::Ctrl(']')),
			],
			|span| (Expr::Error, span),
		);

		block_chain
			.labelled("block")
			.or(inline_expr.clone())
			.recover_with(skip_then_retry_until(
				block_recovery.ignored().or(any().ignored()),
				one_of([
					Token::Ctrl(';'),
					Token::Ctrl('}'),
					Token::Ctrl(')'),
					Token::Ctrl(']'),
				])
				.ignored(),
			))
			.foldl_with(
				just(Token::Ctrl(';')).ignore_then(expr.or_not()).repeated(),
				|a, b, e| {
					let span: Span = e.span();
					(
						Expr::Then(
							Box::new(a),
							Box::new(
								b.unwrap_or_else(|| (Expr::Value(Value::Null), span.to_end())),
							),
						),
						span,
					)
				},
			)
	})
}

#[must_use]
pub fn funcs_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, HashMap<&'src str, Func<'src>>, extra::Err<Rich<'tokens, Token<'src>, Span>>>
+ Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	let ident = select! { Token::Ident(ident) => ident };

	let args = ident
		.separated_by(just(Token::Ctrl(',')))
		.allow_trailing()
		.collect()
		.delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
		.labelled("function args");

	let func = just(Token::Fn)
		.ignore_then(
			ident
				.map_with(|name, e| (name, e.span()))
				.labelled("function name"),
		)
		.then(args)
		.map_with(|start, e| (start, e.span()))
		.then(
			expr_parser()
				.delimited_by(just(Token::Ctrl('{')), just(Token::Ctrl('}')))
				.recover_with(via_parser(nested_delimiters(
					Token::Ctrl('{'),
					Token::Ctrl('}'),
					[
						(Token::Ctrl('('), Token::Ctrl(')')),
						(Token::Ctrl('['), Token::Ctrl(']')),
					],
					|span| (Expr::Error, span),
				))),
		)
		.map(|(((name, args), span), body)| (name, Func { args, span, body }))
		.labelled("function");

	func.repeated()
		.collect::<Vec<_>>()
		.validate(|fs, _, emitter| {
			let mut funcs = HashMap::new();
			for ((name, name_span), f) in fs {
				if funcs.insert(name, f).is_some() {
					emitter.emit(Rich::custom(
						name_span,
						format!("Function '{name}' already exists"),
					));
				}
			}

			funcs
		})
}

pub fn eval_expr<'src>(
	expr: &Spanned<Expr<'src>>,
	funcs: &HashMap<&'src str, Func<'src>>,
	stack: &mut Vec<(&'src str, Value<'src>)>,
) -> Result<Value<'src>, Error> {
	Ok(match &expr.0 {
		Expr::Error => unreachable!(),
		Expr::Value(val) => val.clone(),
		Expr::List(items) => Value::List(
			items
				.iter()
				.map(|item| eval_expr(item, funcs, stack))
				.collect::<Result<_, _>>()?,
		),
		Expr::Local(name) => stack
			.iter()
			.rev()
			.find(|(l, _)| l == name)
			.map(|(_, v)| v.clone())
			.or_else(|| Some(Value::Func(name)).filter(|_| funcs.contains_key(name)))
			.ok_or_else(|| Error {
				span: expr.1,
				message: format!("No such variable '{name}' in scope"),
			})?,
		Expr::Let(local, val, body) => {
			let val = eval_expr(val, funcs, stack)?;
			stack.push((local, val));
			let res = eval_expr(body, funcs, stack)?;
			stack.pop();
			res
		}
		Expr::Then(a, b) => {
			eval_expr(a, funcs, stack)?;
			eval_expr(b, funcs, stack)?
		}
		Expr::Binary(a, BinaryOp::Add, b) => Value::Num(
			eval_expr(a, funcs, stack)?.num(a.1)? + eval_expr(b, funcs, stack)?.num(b.1)?,
		),
		Expr::Binary(a, BinaryOp::Sub, b) => Value::Num(
			eval_expr(a, funcs, stack)?.num(a.1)? - eval_expr(b, funcs, stack)?.num(b.1)?,
		),
		Expr::Binary(a, BinaryOp::Mul, b) => Value::Num(
			eval_expr(a, funcs, stack)?.num(a.1)? * eval_expr(b, funcs, stack)?.num(b.1)?,
		),
		Expr::Binary(a, BinaryOp::Div, b) => Value::Num(
			eval_expr(a, funcs, stack)?.num(a.1)? / eval_expr(b, funcs, stack)?.num(b.1)?,
		),
		Expr::Binary(a, BinaryOp::Eq, b) => {
			Value::Bool(eval_expr(a, funcs, stack)? == eval_expr(b, funcs, stack)?)
		}
		Expr::Binary(a, BinaryOp::NotEq, b) => {
			Value::Bool(eval_expr(a, funcs, stack)? != eval_expr(b, funcs, stack)?)
		}
		Expr::Call(func, args) => {
			let f = eval_expr(func, funcs, stack)?;
			match f {
				Value::Func(name) => {
					let f = &funcs[&name];
					let mut stack = if f.args.len() == args.0.len() {
						f.args
							.iter()
							.zip(args.0.iter())
							.map(|(name, arg)| Ok((*name, eval_expr(arg, funcs, stack)?)))
							.collect::<Result<_, _>>()?
					} else {
						return Err(Error {
							span: expr.1,
							message: format!(
								"'{}' called with wrong number of arguments (expected {name}, found {})",
								f.args.len(),
								args.0.len()
							),
						});
					};
					eval_expr(&f.body, funcs, &mut stack)?
				}
				f => {
					return Err(Error {
						span: func.1,
						message: format!("'{f:?}' is not callable"),
					});
				}
			}
		}
		Expr::If(cond, a, b) => {
			let c = eval_expr(cond, funcs, stack)?;
			match c {
				Value::Bool(true) => eval_expr(a, funcs, stack)?,
				Value::Bool(false) => eval_expr(b, funcs, stack)?,
				c => {
					return Err(Error {
						span: cond.1,
						message: format!("Conditions must be booleans, found '{c:?}'"),
					});
				}
			}
		}
		Expr::Print(a) => {
			let val = eval_expr(a, funcs, stack)?;
			println!("{val}");
			val
		}
	})
}

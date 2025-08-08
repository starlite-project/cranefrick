pub mod polynomial;

use std::{
	collections::HashMap,
	fmt::{Debug, Display},
	ops::{Add, Div, Mul, Neg, RangeInclusive, Sub},
	sync::{Arc, LazyLock, RwLock},
};

use num_traits::{Float, One};

#[derive(Clone, Copy)]
pub struct DirectEval;

impl DirectEval {
	pub const fn var<T: NumericType>(_: &str, value: T) -> T {
		value
	}

	pub const fn var_by_index<T: NumericType>(_: usize, value: T) -> T {
		value
	}

	pub fn eval_with_vars<T>(expr: &ASTRepr<T>, variables: &[T]) -> T
	where
		T: Copy + Float + NumericType,
	{
		Self::eval_vars_optimized(expr, variables)
	}

	pub fn eval_vars_optimized<T>(expr: &ASTRepr<T>, variables: &[T]) -> T
	where
		T: Copy + Float + NumericType,
	{
		match expr {
			ASTRepr::Constant(value) => *value,
			ASTRepr::Variable(index) => variables.get(*index).copied().unwrap_or_else(|| T::zero()),
			ASTRepr::Add(left, right) => {
				Self::eval_vars_optimized(left, variables)
					+ Self::eval_vars_optimized(right, variables)
			}
			ASTRepr::Sub(left, right) => {
				Self::eval_vars_optimized(left, variables)
					- Self::eval_vars_optimized(right, variables)
			}
			ASTRepr::Mul(left, right) => {
				Self::eval_vars_optimized(left, variables)
					* Self::eval_vars_optimized(right, variables)
			}
			ASTRepr::Div(left, right) => {
				Self::eval_vars_optimized(left, variables)
					/ Self::eval_vars_optimized(right, variables)
			}
			ASTRepr::Pow(base, exp) => Self::eval_vars_optimized(base, variables)
				.powf(Self::eval_vars_optimized(exp, variables)),
			ASTRepr::Neg(inner) => -Self::eval_vars_optimized(inner, variables),
			ASTRepr::Ln(inner) => Self::eval_vars_optimized(inner, variables).ln(),
			ASTRepr::Exp(inner) => Self::eval_vars_optimized(inner, variables).exp(),
			ASTRepr::Sin(inner) => Self::eval_vars_optimized(inner, variables).sin(),
			ASTRepr::Cos(inner) => Self::eval_vars_optimized(inner, variables).cos(),
			ASTRepr::Sqrt(inner) => Self::eval_vars_optimized(inner, variables).sqrt(),
		}
	}

	#[must_use]
	pub fn eval_two_vars(expr: &ASTRepr<f64>, x: f64, y: f64) -> f64 {
		Self::eval_two_vars_fast(expr, x, y)
	}

	#[must_use]
	pub fn eval_two_vars_fast(expr: &ASTRepr<f64>, x: f64, y: f64) -> f64 {
		match expr {
			ASTRepr::Constant(value) => *value,
			ASTRepr::Variable(index) => match *index {
				0 => x,
				1 => y,
				_ => 0.0,
			},
			ASTRepr::Add(left, right) => {
				Self::eval_two_vars_fast(left, x, y) + Self::eval_two_vars_fast(right, x, y)
			}
			ASTRepr::Sub(left, right) => {
				Self::eval_two_vars_fast(left, x, y) - Self::eval_two_vars_fast(right, x, y)
			}
			ASTRepr::Mul(left, right) => {
				Self::eval_two_vars_fast(left, x, y) * Self::eval_two_vars_fast(right, x, y)
			}
			ASTRepr::Div(left, right) => {
				Self::eval_two_vars_fast(left, x, y) / Self::eval_two_vars_fast(right, x, y)
			}
			ASTRepr::Pow(base, exp) => {
				Self::eval_two_vars_fast(base, x, y).powf(Self::eval_two_vars_fast(exp, x, y))
			}
			ASTRepr::Neg(inner) => -Self::eval_two_vars_fast(inner, x, y),
			ASTRepr::Ln(inner) => Self::eval_two_vars_fast(inner, x, y).ln(),
			ASTRepr::Exp(inner) => Self::eval_two_vars_fast(inner, x, y).exp(),
			ASTRepr::Sin(inner) => Self::eval_two_vars_fast(inner, x, y).sin(),
			ASTRepr::Cos(inner) => Self::eval_two_vars_fast(inner, x, y).cos(),
			ASTRepr::Sqrt(inner) => Self::eval_two_vars_fast(inner, x, y).sqrt(),
		}
	}
}

impl MathExpr for DirectEval {
	type Repr<T> = T;

	fn constant<T: NumericType>(value: T) -> Self::Repr<T> {
		value
	}

	fn var<T: NumericType>(_: &str) -> Self::Repr<T> {
		T::default()
	}

	fn var_by_index<T: NumericType>(_: usize) -> Self::Repr<T> {
		T::default()
	}

	fn add<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Add<R, Output = Output>,
	{
		left + right
	}

	fn sub<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Sub<R, Output = Output>,
	{
		left - right
	}

	fn mul<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Mul<R, Output = Output>,
	{
		left * right
	}

	fn div<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Div<R, Output = Output>,
	{
		left / right
	}

	fn pow<T>(base: Self::Repr<T>, exp: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		base.powf(exp)
	}

	fn neg<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Neg<Output = T> + NumericType,
	{
		-expr
	}

	fn ln<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		expr.ln()
	}

	fn exp<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		expr.exp()
	}

	fn sqrt<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		expr.sqrt()
	}

	fn sin<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		expr.sin()
	}

	fn cos<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		expr.cos()
	}
}

impl StatisticalExpr for DirectEval {}

#[derive(Clone, Copy)]
pub struct PrettyPrint;

impl PrettyPrint {
	#[must_use]
	pub fn var(name: &str) -> String {
		name.to_owned()
	}
}

impl MathExpr for PrettyPrint {
	type Repr<T> = String;

	fn constant<T: NumericType>(value: T) -> Self::Repr<T> {
		format!("{value}")
	}

	fn var<T: NumericType>(name: &str) -> Self::Repr<T> {
		Self::var(name)
	}

	fn var_by_index<T: NumericType>(_: usize) -> Self::Repr<T> {
		T::default().to_string()
	}

	fn add<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Add<R, Output = Output>,
	{
		format!("({left} + {right})")
	}

	fn sub<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Sub<R, Output = Output>,
	{
		format!("({left} - {right})")
	}

	fn mul<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Mul<R, Output = Output>,
	{
		format!("({left} * {right})")
	}

	fn div<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Div<R, Output = Output>,
	{
		format!("({left} / {right})")
	}

	fn pow<T>(base: Self::Repr<T>, exp: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		format!("({base} ^ {exp})")
	}

	fn neg<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Neg<Output = T> + NumericType,
	{
		format!("(-{expr})")
	}

	fn ln<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		format!("ln({expr})")
	}

	fn exp<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		format!("exp({expr})")
	}

	fn sqrt<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		format!("sqrt({expr})")
	}

	fn sin<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		format!("sin({expr})")
	}

	fn cos<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		format!("cos({expr})")
	}
}

impl StatisticalExpr for PrettyPrint {}

#[derive(Clone, Copy)]
pub struct ASTEval;

impl ASTEval {
	#[must_use]
	pub const fn var<T: NumericType>(index: usize) -> ASTRepr<T> {
		ASTRepr::Variable(index)
	}

	#[must_use]
	pub const fn var_by_name(_: &str) -> ASTRepr<f64> {
		Self::var(0)
	}
}

impl ASTMathExpr for ASTEval {
	type Repr = ASTRepr<f64>;

	fn constant(value: f64) -> Self::Repr {
		ASTRepr::Constant(value)
	}

	fn var(index: usize) -> Self::Repr {
		ASTRepr::Variable(index)
	}

	fn add(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Add(Box::new(left), Box::new(right))
	}

	fn sub(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Sub(Box::new(left), Box::new(right))
	}

	fn mul(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Mul(Box::new(left), Box::new(right))
	}

	fn div(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Div(Box::new(left), Box::new(right))
	}

	fn pow(base: Self::Repr, exp: Self::Repr) -> Self::Repr {
		ASTRepr::Pow(Box::new(base), Box::new(exp))
	}

	fn neg(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Neg(Box::new(expr))
	}

	fn ln(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Ln(Box::new(expr))
	}

	fn exp(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Exp(Box::new(expr))
	}

	fn cos(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Cos(Box::new(expr))
	}

	fn sin(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Sin(Box::new(expr))
	}

	fn sqrt(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Sqrt(Box::new(expr))
	}
}

impl ASTMathExprf64 for ASTEval {
	type Repr = ASTRepr<f64>;

	fn constant(value: f64) -> Self::Repr {
		ASTRepr::Constant(value)
	}

	fn var(index: usize) -> Self::Repr {
		ASTRepr::Variable(index)
	}

	fn add(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Add(Box::new(left), Box::new(right))
	}

	fn sub(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Sub(Box::new(left), Box::new(right))
	}

	fn mul(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Mul(Box::new(left), Box::new(right))
	}

	fn div(left: Self::Repr, right: Self::Repr) -> Self::Repr {
		ASTRepr::Div(Box::new(left), Box::new(right))
	}

	fn pow(base: Self::Repr, exp: Self::Repr) -> Self::Repr {
		ASTRepr::Pow(Box::new(base), Box::new(exp))
	}

	fn neg(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Neg(Box::new(expr))
	}

	fn ln(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Ln(Box::new(expr))
	}

	fn exp(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Exp(Box::new(expr))
	}

	fn sqrt(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Sqrt(Box::new(expr))
	}

	fn sin(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Sin(Box::new(expr))
	}

	fn cos(expr: Self::Repr) -> Self::Repr {
		ASTRepr::Cos(Box::new(expr))
	}
}

impl MathExpr for ASTEval {
	type Repr<T> = ASTRepr<T>;

	fn constant<T: NumericType>(value: T) -> Self::Repr<T> {
		ASTRepr::Constant(value)
	}

	fn var<T: NumericType>(_: &str) -> Self::Repr<T> {
		ASTRepr::Variable(0)
	}

	fn var_by_index<T: NumericType>(index: usize) -> Self::Repr<T> {
		ASTRepr::Variable(index)
	}

	fn add<L, R: NumericType, Output: NumericType>(
		_: Self::Repr<L>,
		_: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Add<R, Output = Output>,
	{
		unimplemented!("use ASTMathExpr or ASTMathExprf64 for concrete implementations")
	}

	fn sub<L, R: NumericType, Output: NumericType>(
		_: Self::Repr<L>,
		_: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Sub<R, Output = Output>,
	{
		unimplemented!("use ASTMathExpr or ASTMathExprf64 for concrete implementations")
	}

	fn mul<L, R: NumericType, Output: NumericType>(
		_: Self::Repr<L>,
		_: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Mul<R, Output = Output>,
	{
		unimplemented!("use ASTMathExpr or ASTMathExprf64 for concrete implementations")
	}

	fn div<L, R: NumericType, Output: NumericType>(
		_: Self::Repr<L>,
		_: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Div<R, Output = Output>,
	{
		unimplemented!("use ASTMathExpr or ASTMathExprf64 for concrete implementations")
	}

	fn pow<T>(base: Self::Repr<T>, exp: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		ASTRepr::Pow(Box::new(base), Box::new(exp))
	}

	fn neg<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Neg<Output = T> + NumericType,
	{
		ASTRepr::Neg(Box::new(expr))
	}

	fn ln<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		ASTRepr::Ln(Box::new(expr))
	}

	fn exp<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		ASTRepr::Exp(Box::new(expr))
	}

	fn cos<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		ASTRepr::Cos(Box::new(expr))
	}

	fn sin<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		ASTRepr::Sin(Box::new(expr))
	}

	fn sqrt<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		ASTRepr::Sqrt(Box::new(expr))
	}
}

impl StatisticalExpr for ASTEval {}

#[expect(missing_copy_implementations)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntRange {
	pub start: i64,
	pub end: i64,
}

impl IntRange {
	#[must_use]
	pub const fn new(start: i64, end: i64) -> Self {
		Self { start, end }
	}

	#[must_use]
	pub const fn one_to_n(n: i64) -> Self {
		Self::new(1, n)
	}

	#[must_use]
	pub const fn zero_to_n_minus_one(n: i64) -> Self {
		Self::new(0, n - 1)
	}

	#[expect(clippy::iter_without_into_iter)]
	#[must_use]
	pub const fn iter(&self) -> RangeInclusive<i64> {
		self.start..=self.end
	}
}

impl RangeType for IntRange {
	type IndexType = i64;

	fn start(&self) -> Self::IndexType {
		self.start
	}

	fn end(&self) -> Self::IndexType {
		self.end
	}

	fn contains(&self, value: &Self::IndexType) -> bool {
		self.iter().contains(value)
	}

	fn len(&self) -> Self::IndexType {
		if self.end >= self.start {
			self.end - self.start + 1
		} else {
			0
		}
	}

	fn is_empty(&self) -> bool {
		self.end < self.start
	}
}

#[derive(Debug, Clone, PartialEq)]
#[expect(missing_copy_implementations)]
pub struct FloatRange {
	pub start: f64,
	pub end: f64,
	pub step: f64,
}

impl FloatRange {
	#[must_use]
	pub const fn new(start: f64, end: f64, step: f64) -> Self {
		Self { start, end, step }
	}

	#[must_use]
	pub const fn unit_step(start: f64, end: f64) -> Self {
		Self::new(start, end, 1.0)
	}
}

impl RangeType for FloatRange {
	type IndexType = f64;

	fn start(&self) -> Self::IndexType {
		self.start
	}

	fn end(&self) -> Self::IndexType {
		self.end
	}

	fn contains(&self, value: &Self::IndexType) -> bool {
		(self.start..=self.end).contains(value)
	}

	fn len(&self) -> Self::IndexType {
		if self.end >= self.start && self.step > 0.0 {
			((self.end - self.start) / self.step).floor() + 1.0
		} else {
			0.0
		}
	}

	fn is_empty(&self) -> bool {
		self.end < self.start || self.step <= 0.0
	}
}

#[derive(Debug, Clone)]
pub struct SymbolicRange<T> {
	pub start: Box<ASTRepr<T>>,
	pub end: Box<ASTRepr<T>>,
}

impl<T: NumericType> SymbolicRange<T> {
	pub fn new(start: ASTRepr<T>, end: ASTRepr<T>) -> Self {
		Self {
			start: Box::new(start),
			end: Box::new(end),
		}
	}

	pub fn one_to_expr(end: ASTRepr<T>) -> Self
	where
		T: One,
	{
		Self::new(ASTRepr::Constant(T::one()), end)
	}

	pub fn evaluate_bounds(&self, variables: &[T]) -> Option<(T, T)>
	where
		T: Copy + Float,
	{
		let start_val = DirectEval::eval_with_vars(&self.start, variables);
		let end_val = DirectEval::eval_with_vars(&self.end, variables);
		Some((start_val, end_val))
	}
}

#[derive(Debug, Clone)]
pub struct ASTFunction<T> {
	pub index_var: String,
	pub body: ASTRepr<T>,
}

impl<T: NumericType> ASTFunction<T> {
	pub fn new(index_var: impl Into<String>, body: ASTRepr<T>) -> Self {
		Self {
			index_var: index_var.into(),
			body,
		}
	}

	pub fn linear(index_var: impl Into<String>, coefficient: T, constant: T) -> Self {
		let body = ASTRepr::Add(
			Box::new(ASTRepr::Mul(
				Box::new(ASTRepr::Constant(coefficient)),
				Box::new(ASTRepr::Variable(0)),
			)),
			Box::new(ASTRepr::Constant(constant)),
		);

		Self::new(index_var, body)
	}

	pub fn power(index_var: impl Into<String>, exponent: T) -> Self {
		let body = ASTRepr::Pow(
			Box::new(ASTRepr::Variable(0)),
			Box::new(ASTRepr::Constant(exponent)),
		);
		Self::new(index_var, body)
	}

	pub fn constant_func(index_var: impl Into<String>, value: T) -> Self {
		let body = ASTRepr::Constant(value);
		Self::new(index_var, body)
	}
}

#[allow(clippy::only_used_in_recursion)]
impl<T> ASTFunction<T>
where
	T: Copy + NumericType,
{
	fn substitute_variable(&self, var_name: &str, value: T) -> ASTRepr<T> {
		self.substitute_in_expr(&self.body, var_name, value)
	}

	fn substitute_in_expr(&self, expr: &ASTRepr<T>, var_name: &str, value: T) -> ASTRepr<T> {
		match expr {
			ASTRepr::Constant(c) => ASTRepr::Constant(*c),
			ASTRepr::Variable(index) => {
				let expected_index = match var_name {
					"i" | "x" => 0,
					"j" | "y" => 1,
					"k" | "z" => 2,
					_ => {
						if let Some(idx) = get_variable_index(var_name) {
							idx
						} else {
							return expr.clone();
						}
					}
				};

				if *index == expected_index {
					ASTRepr::Constant(value)
				} else {
					expr.clone()
				}
			}
			ASTRepr::Add(left, right) => ASTRepr::Add(
				Box::new(self.substitute_in_expr(left, var_name, value)),
				Box::new(self.substitute_in_expr(right, var_name, value)),
			),
			ASTRepr::Sub(left, right) => ASTRepr::Sub(
				Box::new(self.substitute_in_expr(left, var_name, value)),
				Box::new(self.substitute_in_expr(right, var_name, value)),
			),
			ASTRepr::Mul(left, right) => ASTRepr::Mul(
				Box::new(self.substitute_in_expr(left, var_name, value)),
				Box::new(self.substitute_in_expr(right, var_name, value)),
			),
			ASTRepr::Div(left, right) => ASTRepr::Div(
				Box::new(self.substitute_in_expr(left, var_name, value)),
				Box::new(self.substitute_in_expr(right, var_name, value)),
			),
			ASTRepr::Pow(base, exp) => ASTRepr::Pow(
				Box::new(self.substitute_in_expr(base, var_name, value)),
				Box::new(self.substitute_in_expr(exp, var_name, value)),
			),
			ASTRepr::Neg(inner) => {
				ASTRepr::Neg(Box::new(self.substitute_in_expr(inner, var_name, value)))
			}
			ASTRepr::Ln(inner) => {
				ASTRepr::Ln(Box::new(self.substitute_in_expr(inner, var_name, value)))
			}
			ASTRepr::Exp(inner) => {
				ASTRepr::Exp(Box::new(self.substitute_in_expr(inner, var_name, value)))
			}
			ASTRepr::Sin(inner) => {
				ASTRepr::Sin(Box::new(self.substitute_in_expr(inner, var_name, value)))
			}
			ASTRepr::Cos(inner) => {
				ASTRepr::Cos(Box::new(self.substitute_in_expr(inner, var_name, value)))
			}
			ASTRepr::Sqrt(inner) => {
				ASTRepr::Sqrt(Box::new(self.substitute_in_expr(inner, var_name, value)))
			}
		}
	}

	fn contains_variable(&self, expr: &ASTRepr<T>, var_name: &str) -> bool {
		match expr {
			ASTRepr::Constant(_) => false,
			ASTRepr::Variable(index) => {
				let expected_index = match var_name {
					"i" | "x" => 0,
					"j" | "y" => 1,
					"k" | "z" => 2,
					_ => {
						if let Some(idx) = get_variable_index(var_name) {
							idx
						} else {
							return false;
						}
					}
				};

				*index == expected_index
			}
			ASTRepr::Add(left, right)
			| ASTRepr::Sub(left, right)
			| ASTRepr::Mul(left, right)
			| ASTRepr::Div(left, right)
			| ASTRepr::Pow(left, right) => {
				self.contains_variable(left, var_name) || self.contains_variable(right, var_name)
			}
			ASTRepr::Neg(inner)
			| ASTRepr::Ln(inner)
			| ASTRepr::Exp(inner)
			| ASTRepr::Sin(inner)
			| ASTRepr::Cos(inner)
			| ASTRepr::Sqrt(inner) => self.contains_variable(inner, var_name),
		}
	}

	fn extract_factors_recursive(&self, expr: &ASTRepr<T>) -> (Vec<ASTRepr<T>>, ASTRepr<T>)
	where
		T: One,
	{
		match expr {
			ASTRepr::Mul(left, right) => {
				let left_depends = self.contains_variable(left, &self.index_var);
				let right_depends = self.contains_variable(right, &self.index_var);

				match (left_depends, right_depends) {
					(false, false) => (vec![expr.clone()], ASTRepr::Constant(T::one())),
					(false, true) => (vec![(**left).clone()], (**right).clone()),
					(true, false) => (vec![(**right).clone()], (**left).clone()),
					(true, true) => (vec![], expr.clone()),
				}
			}
			_ => {
				if self.contains_variable(expr, &self.index_var) {
					(vec![], expr.clone())
				} else {
					(vec![expr.clone()], ASTRepr::Constant(T::one()))
				}
			}
		}
	}
}

impl<T> SummandFunction<T> for ASTFunction<T>
where
	T: Copy + Float + NumericType,
{
	type Body = ASTRepr<T>;

	fn index_var(&self) -> &str {
		&self.index_var
	}

	fn body(&self) -> &Self::Body {
		&self.body
	}

	fn apply(&self, index: T) -> Self::Body {
		self.substitute_variable(&self.index_var, index)
	}

	fn depends_on_index(&self) -> bool {
		self.contains_variable(&self.body, &self.index_var)
	}

	fn extract_independent_factors(&self) -> (Vec<Self::Body>, Self::Body) {
		self.extract_factors_recursive(&self.body)
	}
}

#[derive(Debug, Clone)]
pub struct VariableRegistry {
	name_to_index: HashMap<String, usize>,
	index_to_name: Vec<String>,
}

impl VariableRegistry {
	#[must_use]
	pub fn new() -> Self {
		Self {
			name_to_index: HashMap::new(),
			index_to_name: Vec::new(),
		}
	}

	pub fn register_variable(&mut self, name: &str) -> usize {
		if let Some(&index) = self.name_to_index.get(name) {
			index
		} else {
			let index = self.index_to_name.len();
			self.name_to_index.insert(name.to_owned(), index);
			self.index_to_name.push(name.to_owned());
			index
		}
	}

	#[must_use]
	pub fn get_index(&self, name: &str) -> Option<usize> {
		self.name_to_index.get(name).copied()
	}

	pub fn get_name(&self, index: usize) -> Option<&str> {
		self.index_to_name.get(index).map(String::as_str)
	}

	#[must_use]
	pub fn get_all_names(&self) -> &[String] {
		&self.index_to_name
	}

	#[must_use]
	pub const fn len(&self) -> usize {
		self.index_to_name.len()
	}

	#[must_use]
	pub const fn is_empty(&self) -> bool {
		self.index_to_name.is_empty()
	}

	pub fn clear(&mut self) {
		self.name_to_index.clear();
		self.index_to_name.clear();
	}

	#[must_use]
	pub fn create_variable_map(&self, values: &[(String, f64)]) -> Vec<f64> {
		let mut result = vec![0.0; self.len()];
		for (name, value) in values {
			if let Some(index) = self.get_index(name) {
				result[index] = *value;
			}
		}

		result
	}

	#[must_use]
	pub fn create_ordered_variable_map(&self, values: &[f64]) -> Vec<f64> {
		let mut result = vec![0.0; self.len()];
		for (i, &value) in values.iter().enumerate() {
			if i < result.len() {
				result[i] = value;
			}
		}

		result
	}
}

impl Default for VariableRegistry {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ExpressionBuilder {
	registry: VariableRegistry,
}

#[allow(clippy::unused_self)]
impl ExpressionBuilder {
	#[must_use]
	pub fn new() -> Self {
		Self {
			registry: VariableRegistry::new(),
		}
	}

	pub fn register_variable(&mut self, name: &str) -> usize {
		self.registry.register_variable(name)
	}

	pub fn var(&mut self, name: &str) -> ASTRepr<f64> {
		let index = self.register_variable(name);
		ASTRepr::Variable(index)
	}

	#[must_use]
	pub const fn var_by_index(&self, index: usize) -> ASTRepr<f64> {
		ASTRepr::Variable(index)
	}

	#[must_use]
	pub const fn constant(&self, value: f64) -> ASTRepr<f64> {
		ASTRepr::Constant(value)
	}

	#[must_use]
	pub const fn registry(&self) -> &VariableRegistry {
		&self.registry
	}

	pub const fn registry_mut(&mut self) -> &mut VariableRegistry {
		&mut self.registry
	}

	#[must_use]
	pub fn eval_with_named_vars(&self, expr: &ASTRepr<f64>, named_vars: &[(String, f64)]) -> f64 {
		let var_array = self.registry.create_variable_map(named_vars);
		self.eval_with_vars(expr, &var_array)
	}

	#[must_use]
	pub fn eval_with_vars(&self, expr: &ASTRepr<f64>, variables: &[f64]) -> f64 {
		DirectEval::eval_with_vars(expr, variables)
	}

	#[must_use]
	pub const fn num_variables(&self) -> usize {
		self.registry.len()
	}

	#[must_use]
	pub fn variable_names(&self) -> &[String] {
		self.registry.get_all_names()
	}

	#[must_use]
	pub fn get_variable_index(&self, name: &str) -> Option<usize> {
		self.registry.get_index(name)
	}

	#[must_use]
	pub fn get_variable_name(&self, index: usize) -> Option<&str> {
		self.registry.get_name(index)
	}
}

impl Default for ExpressionBuilder {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ASTRepr<T> {
	Constant(T),
	Variable(usize),
	Add(Box<Self>, Box<Self>),
	Sub(Box<Self>, Box<Self>),
	Mul(Box<Self>, Box<Self>),
	Div(Box<Self>, Box<Self>),
	Pow(Box<Self>, Box<Self>),
	Neg(Box<Self>),
	Ln(Box<Self>),
	Exp(Box<Self>),
	Sqrt(Box<Self>),
	Sin(Box<Self>),
	Cos(Box<Self>),
}

impl<T> ASTRepr<T> {
	pub const fn count_operations(&self) -> usize {
		match self {
			Self::Constant(..) | Self::Variable(..) => 0,
			Self::Add(left, right)
			| Self::Sub(left, right)
			| Self::Mul(left, right)
			| Self::Div(left, right)
			| Self::Pow(left, right) => 1 + left.count_operations() + right.count_operations(),
			Self::Neg(inner)
			| Self::Ln(inner)
			| Self::Exp(inner)
			| Self::Sin(inner)
			| Self::Cos(inner)
			| Self::Sqrt(inner) => 1 + inner.count_operations(),
		}
	}

	#[allow(clippy::unused_self)]
	pub const fn count_summation_operations(&self) -> usize {
		0
	}

	pub const fn variable_index(&self) -> Option<usize> {
		let Self::Variable(v) = self else {
			return None;
		};

		Some(*v)
	}
}

impl<T> ASTRepr<T>
where
	T: Float + NumericType,
{
	#[must_use]
	pub fn pow(self, exp: Self) -> Self {
		Self::Pow(Box::new(self), Box::new(exp))
	}

	#[must_use]
	pub fn pow_ref(&self, exp: &Self) -> Self {
		self.clone().pow(exp.clone())
	}

	#[must_use]
	pub fn ln(self) -> Self {
		Self::Ln(Box::new(self))
	}

	#[must_use]
	pub fn ln_ref(&self) -> Self {
		self.clone().ln()
	}

	#[must_use]
	pub fn exp(self) -> Self {
		Self::Exp(Box::new(self))
	}

	#[must_use]
	pub fn exp_ref(&self) -> Self {
		self.clone().exp()
	}

	#[must_use]
	pub fn sqrt(self) -> Self {
		Self::Sqrt(Box::new(self))
	}

	#[must_use]
	pub fn sqrt_ref(&self) -> Self {
		self.clone().sqrt()
	}

	#[must_use]
	pub fn sin(self) -> Self {
		Self::Sin(Box::new(self))
	}

	#[must_use]
	pub fn sin_ref(&self) -> Self {
		self.clone().sin()
	}

	#[must_use]
	pub fn cos(self) -> Self {
		Self::Cos(Box::new(self))
	}

	#[must_use]
	pub fn cos_ref(&self) -> Self {
		self.clone().cos()
	}
}

impl<T> Add for ASTRepr<T>
where
	T: Add<Output = T> + NumericType,
{
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Self::Add(Box::new(self), Box::new(rhs))
	}
}

impl<T> Add<&Self> for ASTRepr<T>
where
	T: Add<Output = T> + NumericType,
{
	type Output = Self;

	fn add(self, rhs: &Self) -> Self::Output {
		Self::Add(Box::new(self), Box::new(rhs.clone()))
	}
}

impl<T> Add<ASTRepr<T>> for &ASTRepr<T>
where
	T: Add<Output = T> + NumericType,
{
	type Output = ASTRepr<T>;

	fn add(self, rhs: ASTRepr<T>) -> Self::Output {
		ASTRepr::Add(Box::new(self.clone()), Box::new(rhs))
	}
}

impl<T> Add for &ASTRepr<T>
where
	T: Add<Output = T> + NumericType,
{
	type Output = ASTRepr<T>;

	fn add(self, rhs: Self) -> Self::Output {
		ASTRepr::Add(Box::new(self.clone()), Box::new(rhs.clone()))
	}
}

impl<T> Div for ASTRepr<T>
where
	T: Div<Output = T> + NumericType,
{
	type Output = Self;

	fn div(self, rhs: Self) -> Self::Output {
		Self::Div(Box::new(self), Box::new(rhs))
	}
}

impl<T> Div<&Self> for ASTRepr<T>
where
	T: Div<Output = T> + NumericType,
{
	type Output = Self;

	fn div(self, rhs: &Self) -> Self::Output {
		Self::Div(Box::new(self), Box::new(rhs.clone()))
	}
}

impl<T> Div<ASTRepr<T>> for &ASTRepr<T>
where
	T: Div<Output = T> + NumericType,
{
	type Output = ASTRepr<T>;

	fn div(self, rhs: ASTRepr<T>) -> Self::Output {
		ASTRepr::Div(Box::new(self.clone()), Box::new(rhs))
	}
}

impl<T> Div for &ASTRepr<T>
where
	T: Div<Output = T> + NumericType,
{
	type Output = ASTRepr<T>;

	fn div(self, rhs: Self) -> Self::Output {
		ASTRepr::Div(Box::new(self.clone()), Box::new(rhs.clone()))
	}
}

impl<T> Mul for ASTRepr<T>
where
	T: Mul<Output = T> + NumericType,
{
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		Self::Mul(Box::new(self), Box::new(rhs))
	}
}

impl<T> Mul<&Self> for ASTRepr<T>
where
	T: Mul<Output = T> + NumericType,
{
	type Output = Self;

	fn mul(self, rhs: &Self) -> Self::Output {
		Self::Mul(Box::new(self), Box::new(rhs.clone()))
	}
}

impl<T> Mul<ASTRepr<T>> for &ASTRepr<T>
where
	T: Mul<Output = T> + NumericType,
{
	type Output = ASTRepr<T>;

	fn mul(self, rhs: ASTRepr<T>) -> Self::Output {
		ASTRepr::Mul(Box::new(self.clone()), Box::new(rhs))
	}
}

impl<T> Mul for &ASTRepr<T>
where
	T: Mul<Output = T> + NumericType,
{
	type Output = ASTRepr<T>;

	fn mul(self, rhs: Self) -> Self::Output {
		ASTRepr::Mul(Box::new(self.clone()), Box::new(rhs.clone()))
	}
}

impl<T> Neg for ASTRepr<T>
where
	T: Neg<Output = T> + NumericType,
{
	type Output = Self;

	fn neg(self) -> Self::Output {
		Self::Neg(Box::new(self))
	}
}

impl<T> Neg for &ASTRepr<T>
where
	T: Neg<Output = T> + NumericType,
{
	type Output = ASTRepr<T>;

	fn neg(self) -> Self::Output {
		ASTRepr::Neg(Box::new(self.clone()))
	}
}

impl<T> Sub for ASTRepr<T>
where
	T: NumericType + Sub<Output = T>,
{
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Self::Sub(Box::new(self), Box::new(rhs))
	}
}

impl<T> Sub<&Self> for ASTRepr<T>
where
	T: NumericType + Sub<Output = T>,
{
	type Output = Self;

	fn sub(self, rhs: &Self) -> Self::Output {
		Self::Sub(Box::new(self), Box::new(rhs.clone()))
	}
}

impl<T> Sub<ASTRepr<T>> for &ASTRepr<T>
where
	T: NumericType + Sub<Output = T>,
{
	type Output = ASTRepr<T>;

	fn sub(self, rhs: ASTRepr<T>) -> Self::Output {
		ASTRepr::Sub(Box::new(self.clone()), Box::new(rhs))
	}
}

impl<T> Sub for &ASTRepr<T>
where
	T: NumericType + Sub<Output = T>,
{
	type Output = ASTRepr<T>;

	fn sub(self, rhs: Self) -> Self::Output {
		ASTRepr::Sub(Box::new(self.clone()), Box::new(rhs.clone()))
	}
}

pub trait NumericType: Clone + Debug + Default + Display + Send + Sync + 'static {}

impl<T> NumericType for T where T: Clone + Debug + Default + Display + Send + Sync + 'static {}

pub trait MathExpr {
	type Repr<T>;

	fn constant<T: NumericType>(value: T) -> Self::Repr<T>;

	fn var<T: NumericType>(name: &str) -> Self::Repr<T>;

	fn var_by_index<T: NumericType>(index: usize) -> Self::Repr<T>;

	fn add<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Add<R, Output = Output>;

	fn sub<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Sub<R, Output = Output>;

	fn mul<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Mul<R, Output = Output>;

	fn div<L, R: NumericType, Output: NumericType>(
		left: Self::Repr<L>,
		right: Self::Repr<R>,
	) -> Self::Repr<Output>
	where
		L: Div<R, Output = Output>;

	fn pow<T>(base: Self::Repr<T>, exp: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType;

	fn neg<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Neg<Output = T> + NumericType;

	fn ln<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType;

	fn exp<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType;

	fn sqrt<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType;

	fn sin<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType;

	fn cos<T>(expr: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType;
}

pub trait StatisticalExpr: MathExpr {
	fn logistic<T>(x: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		let one = Self::constant(T::one());
		let neg_x = Self::neg(x);
		let exp_neg_x = Self::exp(neg_x);
		let denominator = Self::add(one, exp_neg_x);
		Self::div(Self::constant(T::one()), denominator)
	}

	fn softplus<T>(x: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		let one = Self::constant(T::one());
		let exp_x = Self::exp(x);
		let one_plus_exp_x = Self::add(one, exp_x);
		Self::ln(one_plus_exp_x)
	}

	fn sigmoid<T>(x: Self::Repr<T>) -> Self::Repr<T>
	where
		T: Float + NumericType,
	{
		Self::logistic(x)
	}
}

pub trait ASTMathExpr {
	type Repr;

	fn constant(value: f64) -> Self::Repr;

	fn var(index: usize) -> Self::Repr;

	fn add(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn sub(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn mul(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn div(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn pow(base: Self::Repr, exp: Self::Repr) -> Self::Repr;

	fn neg(expr: Self::Repr) -> Self::Repr;

	fn ln(expr: Self::Repr) -> Self::Repr;

	fn exp(expr: Self::Repr) -> Self::Repr;

	fn sqrt(expr: Self::Repr) -> Self::Repr;

	fn sin(expr: Self::Repr) -> Self::Repr;

	fn cos(expr: Self::Repr) -> Self::Repr;
}

pub trait ASTMathExprf64 {
	type Repr;

	fn constant(value: f64) -> Self::Repr;

	fn var(index: usize) -> Self::Repr;

	fn add(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn sub(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn mul(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn div(left: Self::Repr, right: Self::Repr) -> Self::Repr;

	fn pow(base: Self::Repr, exp: Self::Repr) -> Self::Repr;

	fn neg(expr: Self::Repr) -> Self::Repr;

	fn ln(expr: Self::Repr) -> Self::Repr;

	fn exp(expr: Self::Repr) -> Self::Repr;

	fn sqrt(expr: Self::Repr) -> Self::Repr;

	fn sin(expr: Self::Repr) -> Self::Repr;

	fn cos(expr: Self::Repr) -> Self::Repr;
}

pub trait RangeType: Clone + Debug + Send + Sync + 'static {
	type IndexType: NumericType;

	fn start(&self) -> Self::IndexType;

	fn end(&self) -> Self::IndexType;

	fn contains(&self, value: &Self::IndexType) -> bool;

	fn len(&self) -> Self::IndexType;

	fn is_empty(&self) -> bool;
}

pub trait SummandFunction<T>: Clone + Debug {
	type Body: Clone;

	fn index_var(&self) -> &str;

	fn body(&self) -> &Self::Body;

	fn apply(&self, index: T) -> Self::Body;

	fn depends_on_index(&self) -> bool;

	fn extract_independent_factors(&self) -> (Vec<Self::Body>, Self::Body);
}

pub trait SummationExpr: MathExpr {
	fn sum_finite<T: NumericType, R: RangeType, F>(
		range: Self::Repr<R>,
		function: Self::Repr<F>,
	) -> Self::Repr<T>
	where
		F: SummandFunction<T>,
		Self::Repr<T>: Clone;

	fn sum_infinite<T: NumericType, F>(
		start: Self::Repr<T>,
		function: Self::Repr<F>,
	) -> Self::Repr<T>
	where
		F: SummandFunction<T>,
		Self::Repr<T>: Clone;

	fn sum_telescoping<T: NumericType, F>(
		range: Self::Repr<IntRange>,
		function: Self::Repr<F>,
	) -> Self::Repr<T>
	where
		F: SummandFunction<T>;

	fn range_to<T: NumericType>(start: Self::Repr<T>, end: Self::Repr<T>) -> Self::Repr<IntRange>;

	fn function<T: NumericType>(index_var: &str, body: Self::Repr<T>)
	-> Self::Repr<ASTFunction<T>>;
}

pub fn global_registry() -> Arc<RwLock<VariableRegistry>> {
	static GLOBAL_REGISTRY: LazyLock<Arc<RwLock<VariableRegistry>>> =
		LazyLock::new(|| Arc::new(RwLock::new(VariableRegistry::new())));

	GLOBAL_REGISTRY.clone()
}

#[must_use]
pub fn register_variable(name: &str) -> usize {
	let registry = global_registry();
	let mut guard = registry.write().unwrap();

	guard.register_variable(name)
}

#[must_use]
pub fn get_variable_index(name: &str) -> Option<usize> {
	let registry = global_registry();
	let guard = registry.read().unwrap();

	guard.get_index(name)
}

#[must_use]
pub fn get_variable_name(index: usize) -> Option<String> {
	let registry = global_registry();
	let guard = registry.read().unwrap();

	guard.get_name(index).map(ToString::to_string)
}

#[must_use]
pub fn create_variable_map(values: &[(String, f64)]) -> Vec<f64> {
	let registry = global_registry();
	let guard = registry.read().unwrap();

	guard.create_variable_map(values)
}

pub fn clear_global_registry() {
	let registry = global_registry();
	let mut guard = registry.write().unwrap();

	guard.clear();
}

#[cfg(test)]
mod tests {
	use std::f64;

	use super::*;

	#[test]
	fn direct_eval() {
		fn linear<E: MathExpr>(x: E::Repr<f64>) -> E::Repr<f64> {
			E::add(E::mul(E::constant(2.0), x), E::constant(1.0))
		}

		let result = linear::<DirectEval>(DirectEval::var("x", 5.0));
		assert_eq!(result, 11.0);
	}

	#[test]
	fn statistical_extension() {
		fn logistic_expr<E: StatisticalExpr>(x: E::Repr<f64>) -> E::Repr<f64> {
			E::logistic(x)
		}

		let result = logistic_expr::<DirectEval>(DirectEval::var("x", 0.0));
		assert!((result - 0.5).abs() < 1e-10);
	}

	#[test]
	fn pretty_print() {
		fn quadratic<E: MathExpr>(x: E::Repr<f64>) -> E::Repr<f64>
		where
			E::Repr<f64>: Clone,
		{
			let a = E::constant(2.0);
			let b = E::constant(3.0);
			let c = E::constant(1.0);

			E::add(
				E::add(E::mul(a, E::pow(x.clone(), E::constant(2.0))), E::mul(b, x)),
				c,
			)
		}

		let expr = quadratic::<PrettyPrint>(PrettyPrint::var("x"));

		assert_eq!(expr, "(((2 * (x ^ 2)) + (3 * x)) + 1)");
	}

	#[test]
	fn horner_polynomal() {
		let coeffs = [1.0, 2.0, 3.0];
		let x = DirectEval::var("x", 2.0);
		let result = polynomial::horner::<DirectEval, f64>(&coeffs, x);
		assert_eq!(result, 17.0);
	}

	#[test]
	fn horner_pretty_print() {
		let coeffs = [1.0, 2.0, 3.0];
		let x = PrettyPrint::var("x");
		let result = polynomial::horner::<PrettyPrint, f64>(&coeffs, x);
		assert_eq!(result, "((((3 * x) + 2) * x) + 1)");
	}

	#[test]
	fn polynomial_from_roots() {
		let roots = [1.0, 2.0];
		let x = DirectEval::var("x", 0.0);
		let result = polynomial::from_roots::<DirectEval, f64>(&roots, x);
		assert_eq!(result, 2.0);

		let x = DirectEval::var("x", 3.0);
		let result = polynomial::from_roots::<DirectEval, f64>(&roots, x);
		assert_eq!(result, 2.0);
	}

	#[test]
	fn division_operations() {
		let div_1_3: f64 = DirectEval::div(DirectEval::constant(1.0), DirectEval::constant(3.0));
		assert!((div_1_3 - 1.0 / 3.0).abs() < 1e-10);

		let div_10_2: f64 = DirectEval::div(DirectEval::constant(10.0), DirectEval::constant(2.0));
		assert!((div_10_2 - 5.0).abs() < 1e-10);

		let div_by_one: f64 =
			DirectEval::div(DirectEval::constant(42.0), DirectEval::constant(1.0));
		assert!((div_by_one - 42.0).abs() < 1e-10);
	}

	#[test]
	fn transcendental_functions() {
		let ln_e: f64 = DirectEval::ln(DirectEval::constant(f64::consts::E));
		assert!((ln_e - 1.0).abs() < 1e-10);

		let exp_1: f64 = DirectEval::exp(DirectEval::constant(1.0));
		assert!((exp_1 - std::f64::consts::E).abs() < 1e-10);

		let sqrt_4: f64 = DirectEval::sqrt(DirectEval::constant(4.0));
		assert!((sqrt_4 - 2.0).abs() < 1e-10);

		let sin_pi_2: f64 = DirectEval::sin(DirectEval::constant(f64::consts::PI / 2.0));
		assert!((sin_pi_2 - 1.0).abs() < 1e-10);

		let cos_0: f64 = DirectEval::cos(DirectEval::constant(0.0));
		assert!((cos_0 - 1.0).abs() < 1e-10);
	}

	#[test]
	fn pretty_print_basic() {
		let var_x = PrettyPrint::var("x");
		assert_eq!(var_x, "x");

		let const_5 = PrettyPrint::constant::<f64>(5.0);
		assert_eq!(const_5, "5");

		let add_expr =
			PrettyPrint::add::<f64, f64, f64>(PrettyPrint::var("x"), PrettyPrint::constant(1.0));

		assert_eq!(add_expr, "(x + 1)");
	}

	#[test]
	fn efficient_variable_indexing() {
		let expr = ASTRepr::Add(
			Box::new(ASTRepr::Variable(0)),
			Box::new(ASTRepr::Variable(1)),
		);
		let result = DirectEval::eval_with_vars(&expr, &[2.0, 3.0]);
		assert_eq!(result, 5.0);

		let expr = ASTRepr::Mul(
			Box::new(ASTRepr::Variable(0)),
			Box::new(ASTRepr::Variable(1)),
		);
		let result = DirectEval::eval_with_vars(&expr, &[4.0, 5.0]);
		assert_eq!(result, 20.0);
	}

	#[test]
	fn mixed_variable_types() {
		let expr = ASTRepr::Add(
			Box::new(ASTRepr::Variable(0)),
			Box::new(ASTRepr::Variable(1)),
		);

		let result = DirectEval::eval_with_vars(&expr, &[2.0, 3.0]);
		assert_eq!(result, 5.0);
	}

	#[test]
	fn variable_index_access() {
		let expr = ASTRepr::<f64>::Variable(5);
		assert_eq!(expr.variable_index(), Some(5));
		let expr = ASTRepr::Constant(42.0);
		assert_eq!(expr.variable_index(), None);
	}

	#[test]
	fn out_of_bounds_variable_index() {
		let expr = ASTRepr::Variable(10);
		let result = DirectEval::eval_with_vars(&expr, &[1.0, 2.0]);
		assert_eq!(result, 0.0);
	}

	#[test]
	fn int_range() {
		let range = IntRange::new(1, 10);
		assert_eq!(range.start(), 1);
		assert_eq!(range.end(), 10);
		assert_eq!(range.len(), 10);
		assert!(range.contains(&5));
		assert!(!range.contains(&15));
		assert!(!range.is_empty());

		let empty_range = IntRange::new(5, 3);
		assert!(empty_range.is_empty());
		assert_eq!(empty_range.len(), 0);
	}

	#[test]
	fn float_range() {
		let range = FloatRange::new(1.0, 10.0, 1.0);
		assert_eq!(range.start(), 1.0);
		assert_eq!(range.end(), 10.0);
		assert_eq!(range.len(), 10.0);
		assert!(range.contains(&5.5));
		assert!(!range.contains(&15.0));

		let empty_range = FloatRange::new(5.0, 3.0, 1.0);
		assert!(empty_range.is_empty());
	}

	#[test]
	fn symbolic_range() {
		let range = SymbolicRange::new(ASTRepr::Constant(1.0), ASTRepr::Variable(0));

		let bounds = range.evaluate_bounds(&[10.0]);

		assert_eq!(bounds, Some((1.0, 10.0)));

		let range2 = SymbolicRange::new(ASTRepr::Variable(0), ASTRepr::Variable(1));

		let bounds2 = range2.evaluate_bounds(&[2.0, 8.0]);
		assert_eq!(bounds2, Some((2.0, 8.0)));
	}

	#[test]
	fn ast_function_creation() {
		let func = ASTFunction::linear("i", 2.0, 3.0);
		assert_eq!(func.index_var(), "i");
		assert!(func.depends_on_index());

		let const_func = ASTFunction::constant_func("i", 42.0);
		assert!(!const_func.depends_on_index());
	}

	#[test]
	fn ast_function_substitution() {
		let func = ASTFunction::new(
			"i",
			ASTRepr::Mul(
				Box::new(ASTRepr::Constant(3.0)),
				Box::new(ASTRepr::Variable(0)),
			),
		);

		let (factors, ..) = func.extract_independent_factors();
		assert_eq!(factors.len(), 1);

		if let Some(ASTRepr::Constant(value)) = factors.first() {
			assert_eq!(*value, 3.0);
		} else {
			panic!("expected constant factor");
		}
	}

	#[test]
	fn range_convenience_methods() {
		let range_1_to_n = IntRange::one_to_n(10);
		assert_eq!(range_1_to_n.start(), 1);
		assert_eq!(range_1_to_n.end(), 10);

		let range_0_to_n_minus_1 = IntRange::zero_to_n_minus_one(10);
		assert_eq!(range_0_to_n_minus_1.start(), 0);
		assert_eq!(range_0_to_n_minus_1.end(), 9);
	}

	#[test]
	fn power_function() {
		let func = ASTFunction::power("i", 2.0);
		assert!(func.depends_on_index());

		let result = func.apply(3.0);
		let evaluated = DirectEval::eval_with_vars(&result, &[]);
		assert_eq!(evaluated, 9.0);
	}

	#[test]
	fn variable_registry() {
		let mut builder = ExpressionBuilder::new();

		let x_index = builder.register_variable("x");
		let y_index = builder.register_variable("y");
		let x_index_again = builder.register_variable("x");

		assert_ne!(x_index, y_index);
		assert_eq!(x_index, x_index_again);

		assert_eq!(builder.get_variable_index("x"), Some(x_index));
		assert_eq!(builder.get_variable_index("y"), Some(y_index));
		assert_eq!(builder.get_variable_index("z"), None);

		assert_eq!(builder.get_variable_name(x_index), Some("x"));
		assert_eq!(builder.get_variable_name(y_index), Some("y"));

		let max_index = x_index.max(y_index);
		assert_eq!(builder.get_variable_name(max_index + 10), None);
	}

	#[test]
	fn named_variable_evaluation() {
		let mut builder = ExpressionBuilder::new();

		let expr = ASTRepr::Add(Box::new(builder.var("x")), Box::new(builder.var("y")));

		let named_vars = [("x".to_owned(), 3.0), ("y".to_owned(), 4.0)];
		let result = builder.eval_with_named_vars(&expr, &named_vars);
		assert_eq!(result, 7.0);
	}

	#[test]
	fn mixed_variable_access() {
		let mut builder = ExpressionBuilder::new();

		let x_idx = builder.register_variable("x");
		let y_idx = builder.register_variable("y");

		let expr = ASTRepr::Mul(
			Box::new(ASTRepr::Variable(x_idx)),
			Box::new(ASTRepr::Variable(y_idx)),
		);

		let result = builder.eval_with_vars(&expr, &[2.0, 5.0]);
		assert_eq!(result, 10.0);

		let named_vars = [("x".to_owned(), 2.0), ("y".to_owned(), 5.0)];
		let result = builder.eval_with_named_vars(&expr, &named_vars);
		assert_eq!(result, 10.0);
	}

	#[test]
	#[expect(clippy::collection_is_never_read)]
	fn variable_registry_performance() {
		let mut builder = ExpressionBuilder::new();
		let start_count = builder.num_variables();

		assert_eq!(start_count, 0);

		let mut indices = Vec::new();
		for i in 0..1000 {
			let var_name = format!("perf_test_var_{i}");
			let index = builder.register_variable(&var_name);
			indices.push(index);
			assert_eq!(index, i);
		}

		for i in 0..1000 {
			let var_name = format!("perf_test_var_{i}");
			let found_index = builder.get_variable_index(&var_name);
			assert_eq!(found_index, Some(i));

			let found_name = builder.get_variable_name(i);
			assert_eq!(found_name, Some(var_name.as_str()));
		}

		let final_count = builder.num_variables();
		assert_eq!(final_count, 1000);
	}
}

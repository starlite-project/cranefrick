use std::ops::{Add, Mul, Sub};

use num_traits::{FromPrimitive, One};

use super::{MathExpr, NumericType};

pub fn horner<E: MathExpr, T>(coeffs: &[T], x: E::Repr<T>) -> E::Repr<T>
where
	E::Repr<T>: Clone,
	T: Add<Output = T> + Clone + Mul<Output = T> + NumericType,
{
	if coeffs.is_empty() {
		return E::constant(T::default());
	}

	if matches!(coeffs.len(), 1) {
		return E::constant(coeffs[0].clone());
	}

	let mut result = E::constant(coeffs[coeffs.len() - 1].clone());

	for coeff in coeffs.iter().rev().skip(1) {
		result = E::add(E::mul(result, x.clone()), E::constant(coeff.clone()));
	}

	result
}

pub fn horner_expr<E: MathExpr, T>(coeffs: &[E::Repr<T>], x: E::Repr<T>) -> E::Repr<T>
where
	E::Repr<T>: Clone,
	T: Add<Output = T> + Mul<Output = T> + NumericType,
{
	if coeffs.is_empty() {
		return E::constant(T::default());
	}

	if matches!(coeffs.len(), 1) {
		return coeffs[0].clone();
	}

	let mut result = coeffs[coeffs.len() - 1].clone();

	for coeff in coeffs.iter().rev().skip(1) {
		result = E::add(E::mul(result, x.clone()), coeff.clone());
	}

	result
}

pub fn from_roots<E: MathExpr, T>(roots: &[T], x: E::Repr<T>) -> E::Repr<T>
where
	E::Repr<T>: Clone,
	T: Clone + One + NumericType + Sub<Output = T>,
{
	if roots.is_empty() {
		return E::constant(One::one());
	}

	let mut result = E::sub(x.clone(), E::constant(roots[0].clone()));

	for root in roots.iter().skip(1) {
		let factor = E::sub(x.clone(), E::constant(root.clone()));
		result = E::mul(result, factor);
	}

	result
}

pub fn horner_derivative<E: MathExpr, T>(coeffs: &[T], x: E::Repr<T>) -> E::Repr<T>
where
	E::Repr<T>: Clone,
	T: Add<Output = T> + Clone + FromPrimitive + Mul<Output = T> + NumericType,
{
	if coeffs.len() <= 1 {
		return E::constant(T::default());
	}

	let mut deriv_coeffs = Vec::with_capacity(coeffs.len() - 1);
	for (i, coeff) in coeffs.iter().enumerate().skip(1) {
		let power = FromPrimitive::from_usize(i).unwrap_or_default();
		deriv_coeffs.push(coeff.clone() * power);
	}

	horner::<E, T>(&deriv_coeffs, x)
}

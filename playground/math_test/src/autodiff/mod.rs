#![allow(clippy::unused_self, missing_copy_implementations)]

pub mod convenience;

use ad_trait::forward_ad::adfn::adfn;

use super::{Error, Result};

#[repr(transparent)]
pub struct ForwardAD(());

impl ForwardAD {
	#[must_use]
	pub const fn new() -> Self {
		Self(())
	}

	pub fn differentiate(&self, f: impl FnOnce(adfn<1>) -> adfn<1>, x: f64) -> Result<(f64, f64)> {
		let ad_x = adfn::new(x, [1.0]);
		let result = f(ad_x);

		let value = result.value();
		let derivative = result.tangent()[0];

		Ok((value, derivative))
	}

	pub fn differentiate_multi(
		&self,
		f: impl FnOnce(&[adfn<8>]) -> adfn<8>,
		inputs: &[f64],
	) -> Result<(f64, Vec<f64>)> {
		if inputs.len() > 8 {
			return Err(Error::InvalidInput(
				"forward AD supports up to 8 variables".into(),
			));
		}

		let mut ad_inputs = Vec::new();
		for (i, &input) in inputs.iter().enumerate() {
			let mut tangent = [0.0; 8];
			tangent[i] = 1.0;
			ad_inputs.push(adfn::new(input, tangent));
		}

		let result = f(&ad_inputs);

		let value = result.value();
		let derivatives = result.tangent()[..inputs.len()].to_owned();

		Ok((value, derivatives))
	}
}

impl Default for ForwardAD {
	fn default() -> Self {
		Self::new()
	}
}

#[repr(transparent)]
pub struct ReverseAD(());

impl ReverseAD {
	#[must_use]
	pub const fn new() -> Self {
		Self(())
	}

	pub fn differentiate(&self, mut f: impl FnMut(f64) -> f64, x: f64) -> Result<(f64, f64)> {
		let h = 1e-8;
		let value = f(x);
		let derivative = (f(x + h) - f(x - h)) / (2.0 * h);

		Ok((value, derivative))
	}

	pub fn differentiate_multi(
		&self,
		mut f: impl FnMut(&[f64]) -> f64,
		inputs: &[f64],
	) -> Result<(f64, Vec<f64>)> {
		let value = f(inputs);
		let mut derivatives = Vec::new();
		let h = 1e-8;

		for i in 0..inputs.len() {
			let mut inputs_plus = inputs.to_owned();
			let mut inputs_minus = inputs.to_owned();
			inputs_plus[i] += h;
			inputs_minus[i] -= h;

			let derivative = (f(&inputs_plus) - f(&inputs_minus)) / (2.0 * h);
			derivatives.push(derivative);
		}

		Ok((value, derivatives))
	}
}

impl Default for ReverseAD {
	fn default() -> Self {
		Self::new()
	}
}

#[repr(transparent)]
pub struct HigherOrderAD(());

impl HigherOrderAD {
	#[must_use]
	pub const fn new() -> Self {
		Self(())
	}

	pub fn second_derivative<F>(&self, f: F, x: f64) -> Result<(f64, f64, f64)>
	where
		F: FnOnce(adfn<1>) -> adfn<1> + Clone,
	{
		let df_dx = |x_val: f64| -> f64 {
			let ad_x = adfn::new(x_val, [1.0]);
			let result = f.clone()(ad_x);
			result.tangent()[0]
		};

		let h = 1e-8;
		let first_deriv = df_dx(x);
		let second_deriv = (df_dx(x + h) - df_dx(x - h)) / (2.0 * h);

		let ad_x = adfn::new(x, [1.0]);
		let result = f(ad_x);
		let value = result.value();

		Ok((value, first_deriv, second_deriv))
	}
}

impl Default for HigherOrderAD {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forward_ad_simple() -> Result<()> {
		let forward_ad = ForwardAD::new();

		let quadratic = |x: adfn<1>| x * x;
		let (value, derivative) = forward_ad.differentiate(quadratic, 3.0)?;

		assert!((value - 9.0).abs() < 1e-10);
		assert!((derivative - 6.0).abs() < 1e-10);

		Ok(())
	}

	#[test]
	fn reverse_ad_simple() -> Result<()> {
		let reverse_ad = ReverseAD::new();

		let cubic = |x: f64| x * x * x;
		let (value, derivative) = reverse_ad.differentiate(cubic, 2.0)?;

		assert!((value - 8.0).abs() < 1e-10);
		assert!((derivative - 12.0).abs() < 1e-6);

		Ok(())
	}

	#[test]
	fn polynomal_functions() -> Result<()> {
		let forward_ad = ForwardAD::new();

		let polynomial = |x: adfn<1>| {
			let x2 = x * x;
			let x3 = x2 * x;
			let two = adfn::new(2.0, [0.0]);
			let one = adfn::new(1.0, [0.0]);
			x3 + two * x2 + x + one
		};

		let (value, derivative) = forward_ad.differentiate(polynomial, 1.0)?;

		assert!((value - 5.0).abs() < 1e-10);
		assert!((derivative - 8.0).abs() < 1e-10);

		Ok(())
	}

	#[test]
	fn multi_variable_gradient() -> Result<()> {
		let func = |vars: &[f64]| vars[0].mul_add(vars[0], vars[1] * vars[1]);
		let grad = convenience::gradient(func, &[1.0, 2.0])?;

		assert!((grad[0] - 2.0).abs() < 1e-6);
		assert!((grad[1] - 4.0).abs() < 1e-6);

		Ok(())
	}
}

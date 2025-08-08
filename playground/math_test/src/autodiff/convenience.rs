use crate::{Result, ReverseAD};

pub fn gradient(f: impl FnMut(&[f64]) -> f64, inputs: &[f64]) -> Result<Vec<f64>> {
	let reverse_ad = ReverseAD::new();

	reverse_ad.differentiate_multi(f, inputs).map(|f| f.1)
}

pub fn jacobian(
	mut f: impl FnMut(&[f64]) -> Vec<f64>,
	inputs: &[f64],
	num_outputs: usize,
) -> Result<Vec<Vec<f64>>> {
	let mut jacobian = Vec::new();

	for output_idx in 0..num_outputs {
		let output_function = |x: &[f64]| f(x)[output_idx];
		let gradient = gradient(output_function, inputs)?;
		jacobian.push(gradient);
	}

	Ok(jacobian)
}

pub fn hessian<F>(f: F, inputs: &[f64]) -> Result<Vec<Vec<f64>>>
where
	F: FnOnce(&[f64]) -> f64 + Clone,
{
	let n = inputs.len();
	let mut hessian = vec![vec![0.0; n]; n];
	let h = 1e-6;

	for i in 0..n {
		for j in 0..n {
			let mut x_pp = inputs.to_owned();
			let mut x_pm = inputs.to_owned();
			let mut x_mp = inputs.to_owned();
			let mut x_mm = inputs.to_owned();

			x_pp[i] += h;
			x_pp[j] += h;
			x_pm[i] += h;
			x_pm[j] -= h;
			x_mp[i] -= h;
			x_mp[j] += h;
			x_mm[i] -= h;
			x_mm[j] -= h;

			let second_deriv = (f.clone()(&x_pp) - f.clone()(&x_pm) - f.clone()(&x_mp)
				+ f.clone()(&x_mm))
				/ (4.0 * h * h);
			hessian[i][j] = second_deriv;
		}
	}

	Ok(hessian)
}

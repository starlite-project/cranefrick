use core::fmt::{Debug, Display, Formatter, Result as FmtResult, Write as _};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanTapeOptions {
	#[serde(skip_serializing_if = "is_zero")]
	initial_move: i32,
	scan_step: i32,
	#[serde(skip_serializing_if = "is_zero")]
	post_scan_move: i32,
}

impl ScanTapeOptions {
	#[must_use]
	pub const fn new(initial_move: i32, scan_step: i32, post_scan_move: i32) -> Self {
		Self {
			initial_move,
			scan_step,
			post_scan_move,
		}
	}

	#[must_use]
	pub const fn initial_move(self) -> i32 {
		self.initial_move
	}

	#[must_use]
	pub const fn only_scans_tape(self) -> bool {
		self.needs_nonzero_cell() && self.is_zeroing_cell()
	}

	#[must_use]
	pub const fn needs_nonzero_cell(self) -> bool {
		matches!(self.initial_move, 0)
	}

	#[must_use]
	pub const fn scan_step(self) -> i32 {
		self.scan_step
	}

	#[must_use]
	pub const fn post_scan_move(self) -> i32 {
		self.post_scan_move
	}

	#[must_use]
	pub const fn into_parts(self) -> (i32, i32, i32) {
		(self.initial_move, self.scan_step, self.post_scan_move)
	}

	#[must_use]
	pub const fn is_zeroing_cell(self) -> bool {
		matches!(self.post_scan_move, 0)
	}
}

impl Debug for ScanTapeOptions {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Display::fmt(&self, f)
	}
}

impl Display for ScanTapeOptions {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char('(')?;
		Display::fmt(&self.initial_move, f)?;
		f.write_str(", ")?;
		Display::fmt(&self.scan_step, f)?;
		f.write_str(", ")?;
		Display::fmt(&self.post_scan_move, f)?;
		f.write_char(')')
	}
}

#[expect(clippy::trivially_copy_pass_by_ref)]
const fn is_zero(x: &i32) -> bool {
	matches!(x, 0)
}

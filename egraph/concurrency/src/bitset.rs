use std::{
	mem,
	sync::atomic::{AtomicU64, Ordering},
};

use super::ReadOptimizedLock;

#[repr(transparent)]
pub struct BitSet {
	data: ReadOptimizedLock<Vec<AtomicU64>>,
}

impl BitSet {
	pub fn with_capacity(n: usize) -> Self {
		let n = n.next_multiple_of(64).next_power_of_two();
		let cells = n / 64;
		let mut data = Vec::with_capacity(cells);
		data.resize_with(cells, AtomicU64::default);
		Self {
			data: ReadOptimizedLock::new(data),
		}
	}

	pub fn get(&self, i: usize) -> bool {
		let cell = i / 64;
		let bit = i % 64;
		let reader = self.data.read();

		reader
			.get(cell)
			.is_some_and(|x| x.load(Ordering::Acquire) & (1 << bit) != 0)
	}

	pub fn set(&self, i: usize, value: bool) {
		let cell = i / 64;
		let bit = i % 64;
		let handle = self.data.read();
		if let Some(cell) = handle.get(cell) {
			if value {
				cell.fetch_or(1 << bit, Ordering::Release);
			} else {
				cell.fetch_and(!(1 << bit), Ordering::Release);
			}
			return;
		}

		mem::drop(handle);
		let mut writer = self.data.lock();
		if cell >= writer.len() {
			writer.resize_with((cell + 1).next_power_of_two(), AtomicU64::default);
		}

		mem::drop(writer);
		self.set(i, value);
	}
}

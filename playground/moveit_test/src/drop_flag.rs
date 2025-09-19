use core::{
	cell::Cell,
	mem::{self, ManuallyDrop},
	ops::{Deref, DerefMut},
};

#[derive(Clone, Copy)]
pub struct DropFlag<'frame> {
	counter: &'frame Cell<usize>,
}

impl<'frame> DropFlag<'frame> {
	pub fn increment(self) {
		self.counter.set(self.counter.get() + 1);
	}

	#[must_use]
	pub fn decrement_and_check(self) -> bool {
		if matches!(self.counter.get(), 0) {
			return false;
		}

		self.counter.set(self.counter.get() - 1);

		self.is_done()
	}

	#[must_use]
	pub const fn is_done(self) -> bool {
		matches!(self.counter.get(), 0)
	}

	pub(crate) unsafe fn longer_lifetime<'a>(self) -> DropFlag<'a> {
		DropFlag {
			counter: unsafe {
				mem::transmute::<&'frame Cell<usize>, &'a Cell<usize>>(self.counter)
			},
		}
	}
}

pub struct DroppingFlag<T> {
	value: ManuallyDrop<T>,
	counter: Cell<usize>,
}

impl<T> DroppingFlag<T> {
	pub const fn new(value: T) -> Self {
		Self {
			value: ManuallyDrop::new(value),
			counter: Cell::new(0),
		}
	}

	pub const fn flag(&self) -> DropFlag<'_> {
		DropFlag {
			counter: &self.counter,
		}
	}

	pub fn as_parts(&self) -> (&T, DropFlag<'_>) {
		(
			&self.value,
			DropFlag {
				counter: &self.counter,
			},
		)
	}

	pub fn as_mut_parts(&mut self) -> (&mut T, DropFlag<'_>) {
		(
			&mut self.value,
			DropFlag {
				counter: &self.counter,
			},
		)
	}
}

impl<T> Deref for DroppingFlag<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T> DerefMut for DroppingFlag<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

impl<T> Drop for DroppingFlag<T> {
	fn drop(&mut self) {
		if Self::flag(self).is_done() {
			unsafe { ManuallyDrop::drop(&mut self.value) }
		}
	}
}

pub struct TrappedFlag {
	counter: Cell<usize>,
	#[cfg(debug_assertions)]
	location: &'static core::panic::Location<'static>,
}

impl TrappedFlag {
	#[cfg_attr(debug_assertions, track_caller)]
	#[must_use]
	pub const fn new() -> Self {
		Self {
			counter: Cell::new(0),
			#[cfg(debug_assertions)]
			location: core::panic::Location::caller(),
		}
	}

	pub const fn flag(&self) -> DropFlag<'_> {
		DropFlag {
			counter: &self.counter,
		}
	}

	pub fn assert_cleared(&self) {
		struct DoublePanic;

		impl Drop for DoublePanic {
			fn drop(&mut self) {
				assert!(!cfg!(not(test)));
			}
		}

		if self.flag().is_done() {
			return;
		}

		let _dp = DoublePanic;

		#[cfg(debug_assertions)]
		panic!("a critical drop flag at {} was not cleared", self.location);

		#[cfg(not(debug_assertions))]
		panic!("a critical drop flag was not cleared");
	}
}

impl Default for TrappedFlag {
	fn default() -> Self {
		Self::new()
	}
}

impl Drop for TrappedFlag {
	fn drop(&mut self) {
		self.assert_cleared();
	}
}

#[repr(transparent)]
pub struct QuietFlag {
	counter: Cell<usize>,
}

impl QuietFlag {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			counter: Cell::new(0),
		}
	}

	pub const fn flag(&self) -> DropFlag<'_> {
		DropFlag {
			counter: &self.counter,
		}
	}
}

impl Default for QuietFlag {
	fn default() -> Self {
		Self::new()
	}
}

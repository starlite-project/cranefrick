use std::{
	alloc::{Layout, alloc, dealloc, handle_alloc_error},
	mem,
	ptr::{self, NonNull},
};

#[repr(transparent)]
pub struct BoxAllocation<T>(NonNull<T>);

impl<T> BoxAllocation<T> {
	pub fn init(self, value: T) -> Box<T> {
		if matches!(mem::size_of::<T>(), 0) {
			return Box::new(value);
		}

		unsafe {
			let ptr = self.0.as_ptr();
			mem::forget(self);
			ptr::write(ptr, value);
			Box::from_raw(ptr)
		}
	}
}

impl<T> Drop for BoxAllocation<T> {
	fn drop(&mut self) {
		if matches!(mem::size_of::<T>(), 0) {
			return;
		}

		let layout = Layout::new::<T>();
		unsafe {
			dealloc(self.0.as_ptr().cast(), layout);
		}
	}
}

pub trait BoxHelper<T> {
	fn alloc() -> BoxAllocation<T>;
}

impl<T> BoxHelper<T> for Box<T> {
	fn alloc() -> BoxAllocation<T> {
		if matches!(mem::size_of::<T>(), 0) {
			return BoxAllocation(NonNull::dangling());
		}

		let layout = Layout::new::<T>();
		BoxAllocation(
			NonNull::new(unsafe { alloc(layout).cast::<T>() })
				.unwrap_or_else(|| handle_alloc_error(layout)),
		)
	}
}

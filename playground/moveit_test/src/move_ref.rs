use core::{
	mem,
	ops::{Deref, DerefMut},
	pin::Pin,
	ptr,
};

use super::DropFlag;

pub struct MoveRef<'a, T: ?Sized> {
	ptr: &'a mut T,
	drop_flag: DropFlag<'a>,
}

impl<'a, T: ?Sized> MoveRef<'a, T> {
	pub const unsafe fn new_unchecked(ptr: &'a mut T, drop_flag: DropFlag<'a>) -> Self {
		Self { ptr, drop_flag }
	}

	#[must_use]
	pub const fn into_pin(this: Self) -> Pin<Self> {
		unsafe { Pin::new_unchecked(this) }
	}

	#[must_use]
	pub const fn as_ptr(&self) -> *const T {
		self.ptr
	}

	pub const fn as_mut_ptr(&mut self) -> *mut T {
		self.ptr
	}

	pub(crate) const fn drop_flag(&self) -> DropFlag<'a> {
		self.drop_flag
	}
}

impl<'a, T> MoveRef<'a, T> {
	pub(crate) unsafe fn cast<U>(mut self) -> MoveRef<'a, U> {
		let mr = MoveRef {
			ptr: unsafe { &mut *Self::as_mut_ptr(&mut self).cast() },
			drop_flag: self.drop_flag,
		};

		mem::forget(self);
		mr
	}

    #[must_use]
    pub fn into_inner(self) -> T {
        unsafe {
            let value = ptr::read(self.ptr);
            let _ = self.cast::<()>();
            value
        }
    }
}

impl<T: ?Sized> Deref for MoveRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.ptr
	}
}

impl<T: ?Sized> DerefMut for MoveRef<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.ptr
	}
}

impl<T: ?Sized> Drop for MoveRef<'_, T> {
	fn drop(&mut self) {
		_ = self.drop_flag.decrement_and_check();
		unsafe { ptr::drop_in_place(self.ptr) }
	}
}

impl<'a, T> From<MoveRef<'a, T>> for Pin<MoveRef<'a, T>> {
	fn from(value: MoveRef<'a, T>) -> Self {
		MoveRef::into_pin(value)
	}
}

pub trait AsMove: Sized + Deref {
    type Storage: Sized;
}

#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use core::ptr::NonNull;

pub trait PointerExt<T: ?Sized> {
	#[cfg(feature = "alloc")]
	unsafe fn into_boxed(self) -> Option<Box<T>>;
}

impl<T: ?Sized> PointerExt<T> for *mut T {
	#[cfg(feature = "alloc")]
	unsafe fn into_boxed(self) -> Option<Box<T>> {
		if self.is_null() {
			None
		} else {
			Some(unsafe { Box::from_raw(self) })
		}
	}
}

impl<T: ?Sized> PointerExt<T> for NonNull<T> {
	#[cfg(feature = "alloc")]
	unsafe fn into_boxed(self) -> Option<Box<T>> {
		unsafe { self.as_ptr().into_boxed() }
	}
}

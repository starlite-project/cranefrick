#[cfg(feature = "alloc")]
use alloc::boxed::Box;

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

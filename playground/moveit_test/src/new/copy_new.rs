use core::{
	mem::{self, MaybeUninit},
	ops::Deref,
	pin::Pin,
};

use super::{New, by_raw};

pub unsafe trait CopyNew: Sized {
	unsafe fn copy_new(src: &Self, this: Pin<&mut MaybeUninit<Self>>);
}

pub fn copy<P>(ptr: P) -> impl New<Output = P::Target>
where
	P: Deref,
	P::Target: CopyNew,
{
	unsafe {
		by_raw(move |this| {
			CopyNew::copy_new(&*ptr, this);

			mem::drop(ptr);
		})
	}
}

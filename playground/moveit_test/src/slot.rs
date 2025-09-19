use core::{
	mem::{self, MaybeUninit},
	pin::Pin,
	ptr,
};

use super::{DropFlag, MoveRef, New, TryNew, of};

pub struct Slot<'frame, T> {
	ptr: &'frame mut MaybeUninit<T>,
	drop_flag: DropFlag<'frame>,
}

impl<'frame, T> Slot<'frame, T> {
	pub const unsafe fn new_unchecked(
		ptr: &'frame mut MaybeUninit<T>,
		drop_flag: DropFlag<'frame>,
	) -> Self {
		Self { ptr, drop_flag }
	}

	pub fn put(self, value: T) -> MoveRef<'frame, T> {
		unsafe { Pin::into_inner_unchecked(self.pin(value)) }
	}

	pub fn pin(self, value: T) -> Pin<MoveRef<'frame, T>> {
		self.emplace(of(value))
	}

	pub fn emplace<N>(self, new: N) -> Pin<MoveRef<'frame, T>>
	where
		N: New<Output = T>,
	{
		match self.try_emplace(new) {
			Ok(x) => x,
			Err(e) => match e {},
		}
	}

	pub fn try_emplace<N>(self, new: N) -> Result<Pin<MoveRef<'frame, T>>, N::Error>
	where
		N: TryNew<Output = T>,
	{
		unsafe {
			self.drop_flag.increment();
			new.try_new(Pin::new_unchecked(self.ptr))?;
			Ok(MoveRef::into_pin(MoveRef::new_unchecked(
				self.ptr.assume_init_mut(),
				self.drop_flag,
			)))
		}
	}

	#[must_use]
	pub fn into_pinned(self) -> Slot<'frame, Pin<T>> {
		unsafe { self.cast() }
	}

	#[must_use]
	pub unsafe fn cast<U>(self) -> Slot<'frame, U> {
		debug_assert!(mem::size_of::<T>() >= mem::size_of::<U>());
		debug_assert!(mem::align_of::<T>() >= mem::align_of::<U>());

		Slot {
			ptr: unsafe { &mut *self.ptr.as_mut_ptr().cast() },
			drop_flag: self.drop_flag,
		}
	}
}

impl<'frame, T> Slot<'frame, Pin<T>> {
	#[must_use]
	pub fn into_unpinned(self) -> Slot<'frame, T> {
		unsafe { self.cast() }
	}
}

pub struct DroppingSlot<'frame, T> {
	ptr: &'frame mut MaybeUninit<T>,
	drop_flag: DropFlag<'frame>,
}

impl<'frame, T> DroppingSlot<'frame, T> {
	pub unsafe fn new_unchecked(
		ptr: &'frame mut MaybeUninit<T>,
		drop_flag: DropFlag<'frame>,
	) -> Self {
		drop_flag.increment();
		Self { ptr, drop_flag }
	}

	pub const fn put(self, value: T) -> (&'frame mut T, DropFlag<'frame>) {
		({ self.ptr }.write(value), self.drop_flag)
	}
}

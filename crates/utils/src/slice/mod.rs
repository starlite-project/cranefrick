mod windows_n;

pub use self::windows_n::*;

pub trait SliceExt<T> {
	fn windows_n<const N: usize>(&self) -> WindowsN<'_, T, N>;
}

impl<T> SliceExt<T> for [T] {
	fn windows_n<const N: usize>(&self) -> WindowsN<'_, T, N> {
		WindowsN::new(self)
	}
}

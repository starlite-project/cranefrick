pub trait IntoSigned {
	type Signed;

	fn into_signed(self) -> Self::Signed;
}

macro_rules! impl_into_signed {
	($unsign:ty => $sign:ty) => {
		impl $crate::ints::signed::IntoSigned for $unsign {
			type Signed = $sign;

			fn into_signed(self) -> Self::Signed {
				self as _
			}
		}

		impl $crate::ints::signed::IntoSigned for $sign {
			type Signed = Self;

			fn into_signed(self) -> Self {
				self
			}
		}
	};
}

impl_into_signed!(u8 => i8);
impl_into_signed!(u16 => i16);
impl_into_signed!(u32 => i32);
impl_into_signed!(u64 => i64);
impl_into_signed!(u128 => i128);
impl_into_signed!(usize => isize);

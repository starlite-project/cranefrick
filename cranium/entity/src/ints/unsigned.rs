pub trait IntoUnsigned {
	type Unsigned;

	fn into_unsigned(self) -> Self::Unsigned;
}

macro_rules! impl_into_unsigned {
	($sign:ty => $unsign:ty) => {
		impl $crate::ints::unsigned::IntoUnsigned for $sign {
			type Unsigned = $unsign;

			fn into_unsigned(self) -> Self::Unsigned {
				self as _
			}
		}

		impl $crate::ints::unsigned::IntoUnsigned for $unsign {
			type Unsigned = Self;

			fn into_unsigned(self) -> Self {
				self
			}
		}
	};
}

impl_into_unsigned!(i8 => u8);
impl_into_unsigned!(i16 => u16);
impl_into_unsigned!(i32 => u32);
impl_into_unsigned!(i64 => u64);
impl_into_unsigned!(i128 => u128);
impl_into_unsigned!(isize => usize);

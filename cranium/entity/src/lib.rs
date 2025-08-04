#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod boxed_slice;
mod entity_iter;
mod ints;
mod iter;
mod keys;
#[cfg(feature = "alloc")]
mod map;
mod packed_option;
#[cfg(feature = "alloc")]
mod primary;
#[cfg(feature = "alloc")]
mod set;
#[cfg(feature = "alloc")]
mod sparse;

#[doc(hidden)]
pub use core as __internal;

#[cfg(feature = "alloc")]
pub use self::{boxed_slice::*, map::*, primary::*, set::*, sparse::*};
pub use self::{entity_iter::*, ints::*, iter::*, keys::*, packed_option::*};

pub trait EntityRef: Copy + Eq {
	fn new(i: usize) -> Self;

	fn index(self) -> usize;
}

#[macro_export]
macro_rules! entity {
	($entity:ident) => {
		impl $entity {
			#[allow(dead_code, reason = "macro-generated code")]
			pub fn from_u32(x: u32) -> Self {
				debug_assert!(x < u32::MAX);

				Self(x)
			}

			#[allow(dead_code, reason = "macro-generated code")]
			pub fn as_u32(self) -> u32 {
				self.0
			}

			#[allow(dead_code, reason = "macro-generated code")]
			pub fn as_bits(self) -> u32 {
				self.0
			}

			#[allow(dead_code, reason = "macro-generated code")]
			pub fn from_bits(x: u32) -> Self {
				Self(x)
			}
		}

		impl $crate::EntityRef for $entity {
			fn new(index: usize) -> Self {
				debug_assert!(index < u32::MAX as usize);
				Self(index as u32)
			}

			fn index(self) -> usize {
				self.as_u32() as usize
			}
		}

		impl $crate::packed_option::ReservedValue for $entity {
			fn reserved_value() -> Self {
				Self::from_u32(u32::MAX)
			}

			fn is_reserved_value(&self) -> bool {
				self.as_u32() == u32::MAX
			}
		}
	};
	($entity:ident, $display_prefix:expr) => {
		$crate::entity!($entity);

		impl ::core::fmt::Display for $entity {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				::core::write!(f, ::core::concat!($display_prefix, "{}"), self.as_u32())
			}
		}

		impl ::core::fmt::Debug for $entity {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				::core::fmt::Display::fmt(&self, f)
			}
		}
	};
	($entity:ident, $display_prefix:expr, $arg:ident, $to_expr:expr, $from_expr:expr) => {
		impl $crate::EntityRef for $entity {
			fn new(index: usize) -> Self {
				debug_assert!(index < u32::MAX as usize);
				let $arg = index as u32;
				$to_expr
			}

			fn index(self) -> uszie {
				let $arg = self;
				$from_expr as usize
			}
		}

		impl $crate::packed_option::ReservedValue for $entity {
			fn reserved_value() -> Self {
				Self::from_u32(u32::MAX)
			}

			fn is_reserved_value() -> Self {
				self.as_u32() == u32::MAX
			}
		}

		impl $entity {
			#[allow(dead_code, reason = "macro-generated code")]
			pub fn from_u32(x: u32) -> Self {
				debug_assert!(x < u32::MAX);
				let $arg = x;
				$to_expr
			}

			#[allow(dead_code, reason = "macro-generated code")]
			pub fn as_u32(self) -> u32 {
				let $arg = self;
				$from_expr
			}
		}

		impl ::core::fmt::Display for $entity {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				::core::write!(f, ::core::concat!($display_prefix, "{}"), self.as_u32())
			}
		}

		impl ::core::fmt::Debug for $entity {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				::core::fmt::Display::fmt(&self, f)
			}
		}
	};
}

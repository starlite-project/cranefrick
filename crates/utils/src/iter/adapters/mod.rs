#[cfg(feature = "alloc")]
mod sorted;

#[cfg(feature = "alloc")]
pub use self::sorted::*;

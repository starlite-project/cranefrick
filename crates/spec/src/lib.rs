#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

pub const TAPE_SIZE: usize = 0x8000;

const _: () = const { assert!(TAPE_SIZE.is_power_of_two()) };

#[cfg(target_pointer_width = "64")]
pub const POINTER_SIZE: usize = 64;
#[cfg(target_pointer_width = "32")]
pub const POINTER_SIZE: usize = 32;

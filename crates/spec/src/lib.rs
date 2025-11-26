#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

pub const POINTER_SIZE: usize = core::mem::size_of::<usize>() * 8;
pub const TAPE_SIZE: usize = 0x8000;

const _: () = const { assert!(TAPE_SIZE.is_power_of_two()) };

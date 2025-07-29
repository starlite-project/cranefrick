#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod brainfuck;
mod opts;

pub use self::{brainfuck::*, opts::*};

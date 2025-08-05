#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

pub mod ast;
pub mod codegen;
pub mod compile;
mod disjoint_sets;
pub mod error;
pub mod files;
pub mod lexer;
pub mod overlap;
pub mod parser;
pub mod printer;
pub mod sema;
pub mod serialize;
mod stable_mapset;
pub mod trie_again;

pub use self::{disjoint_sets::*, stable_mapset::*};

macro_rules! declare_id {
    (
        $(#[$attr:meta])*
        $name:ident
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(pub usize);

        impl $name {
            #[must_use]
            pub const fn index(self) -> usize {
                self.0
            }
        }
    };
}

pub(crate) use declare_id;

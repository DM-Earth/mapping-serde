//! A collection of utilities for mapping-serde.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod ext;
mod pipe;
#[cfg(feature = "translate")]
mod translate;

mod ref_visitor;

pub use ext::DeserializerExt;
pub use ref_visitor::RefVisitor;

#[cfg(feature = "std")]
pub use translate::flat2tree::Nest;
#[cfg(feature = "std")]
pub use translate::tree2flat::Flatten;

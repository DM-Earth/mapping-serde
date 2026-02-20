//! A collection of utilities for mapping-serde.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod ext;
mod pipe;
#[cfg(feature = "translate")]
mod translate;

pub use ext::DeserializerExt;
pub use translate::flat2tree::Nest;

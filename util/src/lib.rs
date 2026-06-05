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

#[cfg(feature = "translate")]
pub use translate::flat2tree::Nest;
#[cfg(feature = "translate")]
pub use translate::tree2flat::Flatten;

#[cfg(test)]
mod tests {
    #[cfg(feature = "translate")]
    mod translate;
}

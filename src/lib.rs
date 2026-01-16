//! Framework for Java deobfuscation mapping serialization and deserialization.

#![no_std]
#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod de;
pub mod ser;

#[doc(inline)]
pub use de::{Deserialize, Deserializer};
#[doc(inline)]
pub use ser::{Serialize, Serializer};

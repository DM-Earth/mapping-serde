//! Framework for Java deobfuscation mapping serialization and deserialization.

#![no_std]
#![allow(clippy::missing_errors_doc)]

pub mod de;
pub mod ser;

pub use de::Deserializer;
pub use ser::Serializer;

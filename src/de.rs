//! Deserialization based on visitors.

use std::fmt::Display;

mod visit;

pub use visit::*;

/// Error type used by a deserializer.
pub trait Error: std::error::Error + Sized {
    /// A general error message during deserialization.
    fn custom<T>(msg: T) -> Self
    where
        T: Display;

    /// Receives a type different from what it was expecting when visiting through a deserialization.
    fn invalid_type(unexp: impl Display, exp: impl Display) -> Self {
        Self::custom(format_args!("invalid type: {unexp}, expected {exp}"))
    }
}

/// Deserializer of a mapping file.
pub trait Deserializer<'de> {
    /// The error type.
    type Error: Error;

    /// Seeks for the next entry and passes it into the given `visitor`.
    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>;
}

impl<'de, T> Deserializer<'de> for &mut T
where
    T: Deserializer<'de>,
{
    type Error = T::Error;

    #[inline]
    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        T::deserialize_any(self, visitor)
    }
}

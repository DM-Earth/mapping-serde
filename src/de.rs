//! Deserialization based on visitors.

use core::fmt::Display;

mod visit;

pub use visit::*;

/// Error type used by a deserializer.
pub trait Error: core::error::Error + Sized {
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

    /// Hints the count of remaining top-level entries of this deserializer.
    ///
    /// See [`Iterator::size_hint`] for more information.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
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

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        T::size_hint(self)
    }
}

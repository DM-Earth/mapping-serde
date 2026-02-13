//! Deserialization based on visitors.

use core::{convert::Infallible, fmt::Display};

mod r#impl;
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

    /// One or more fields are missing in the provided arguments during deserializer.
    fn missing_field(field: impl Display) -> Self {
        Self::custom(format_args!("missing field for deserializer: {field}"))
    }
}

/// A type that can be deserialized from a [`Deserializer`].
pub trait Deserialize<'de>: Sized {
    /// Whether the implementation returns `Some` or `None` conditionally.
    const IS_CONDITIONAL: bool = true;

    /// Deserializes elements from the given deserializer.
    fn deserialize<D>(deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: Deserializer<'de>;
}

/// Deserializer of a mapping file.
pub trait Deserializer<'de> {
    /// The error type.
    type Error: Error;

    /// Returns the source namespace of this mapping.
    fn src_namespace(&self) -> &str;

    /// Returns the destination namespaces of this mapping.
    fn dst_namespaces(&self) -> impl Iterator<Item = &str>;

    /// Seeks for the next entry and passes it into the given `visitor`.
    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>;

    /// Hints the deserializer to deserialize a class.
    #[inline]
    fn deserialize_class<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    /// Hints the deserializer to deserialize a field.
    #[inline]
    fn deserialize_field<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    /// Hints the deserializer to deserialize a method.
    #[inline]
    fn deserialize_method<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    /// Hints the deserializer to deserialize a method argument.
    #[inline]
    fn deserialize_method_arg<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    /// Hints the deserializer to deserialize a method variable.
    #[inline]
    fn deserialize_method_var<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

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
    fn deserialize_class<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        T::deserialize_class(self, visitor)
    }

    #[inline]
    fn deserialize_field<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        T::deserialize_field(self, visitor)
    }

    #[inline]
    fn deserialize_method<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        T::deserialize_method(self, visitor)
    }

    #[inline]
    fn deserialize_method_arg<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        T::deserialize_method_arg(self, visitor)
    }

    #[inline]
    fn deserialize_method_var<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        T::deserialize_method_var(self, visitor)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        T::size_hint(self)
    }

    #[inline]
    fn src_namespace(&self) -> &str {
        T::src_namespace(self)
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        T::dst_namespaces(self)
    }
}

impl Error for Infallible {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        unreachable!()
    }
}

//! Deserializer extensions.

use mapping_serde::{Deserializer, Serializer};

#[cfg(feature = "translate")]
use crate::Flatten;
#[cfg(feature = "translate")]
use crate::Nest;

/// Extension trait for a [`Deserializer`].
pub trait DeserializerExt<'de>: Deserializer<'de> {
    /// Pipes this deserializer's elements into the given serializer.
    ///
    /// # Panics
    ///
    /// Panics if the serializer doesn't accept this deserializer's class layout.
    #[allow(clippy::missing_errors_doc)]
    #[inline]
    fn pipe_into<S>(self, serializer: S) -> Result<(), Self::Error>
    where
        Self: Sized,
        S: Serializer,
    {
        assert!(S::layout_matches::<Self>(), "layout mismatch");
        crate::pipe::pipe_into(self, serializer)
    }

    /// Makes this deserializer nested in its class layout.
    #[cfg(feature = "translate")]
    #[inline]
    fn nest(self) -> Nest<'de, Self>
    where
        Self: Sized,
    {
        Nest::new(self)
    }

    /// Makes this deserializer flattened in its class layout.
    #[cfg(feature = "translate")]
    #[inline]
    fn flatten(self) -> Flatten<'de, Self>
    where
        Self: Sized,
    {
        Flatten::new(self)
    }
}

impl<'de, D> DeserializerExt<'de> for D where D: Deserializer<'de> {}

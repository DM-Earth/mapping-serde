//! Serialization.

use core::{
    convert::Infallible,
    fmt::{Debug, Display},
    marker::PhantomData,
};

mod r#impl;

/// Error type used by a serializer.
pub trait Error: core::error::Error + Sized {
    /// A general error message during serialization.
    fn custom<T>(msg: T) -> Self
    where
        T: Display;

    /// Ran into an element type that is not supported by the serializer.
    fn unsupported_type(ty: impl Display) -> Self {
        Self::custom(format_args!("unsupported type for serializer: {ty}"))
    }

    /// One or more fields are missing in the provided arguments during serialization.
    fn missing_field(field: impl Display) -> Self {
        Self::custom(format_args!("missing field for serializer: {field}"))
    }
}

/// Type that could be serialized into a mapping file through [`Serializer`].
pub trait Serialize {
    /// Serializes this value.
    fn serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer;
}

impl<T: ?Sized> Serialize for &T
where
    T: Serialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        T::serialize(self, serializer)
    }
}

/// Serializer of a mapping file.
pub trait Serializer {
    /// The error type.
    type Error: Error;

    /// Whether inner classes should be flattened.
    const FLAT_CLASSES: bool;

    /// Type returned from [`Serializer::serialize_class`] for class content serialization.
    type SerializeClass<'a>: Serializer<Error = Self::Error>
    where
        Self: 'a;

    /// Type returned from [`Serializer::serialize_field`] for field comment serialization.
    type SerializeField<'a>: Serializer<Error = Self::Error>
    where
        Self: 'a;

    /// Type returned from [`Serializer::serialize_method`] for method arguments, variables and comment serialization.
    type SerializeMethod<'a>: Serializer<Error = Self::Error>
    where
        Self: 'a;

    /// Type returned from [`Serializer::serialize_method_arg`] for method argument comment serialization.
    type SerializeMethodArg<'a>: Serializer<Error = Self::Error>
    where
        Self: 'a;

    /// Type returned from [`Serializer::serialize_method_var`] for method variable comment serialization.
    type SerializeMethodVar<'a>: Serializer<Error = Self::Error>
    where
        Self: 'a;

    /// Serializes a comment literal.
    fn serialize_comment(&mut self, value: &str) -> Result<(), Self::Error>;

    /// Serializes a class.
    fn serialize_class<Dst>(
        &mut self,
        src: &str,
        dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>;

    /// Serializes a field.
    fn serialize_field<Dst, DstDesc>(
        &mut self,
        src: &str,
        desc: Option<&str>,
        dst: Dst,
        dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeField<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>;

    /// Serializes a method.
    fn serialize_method<Dst, DstDesc>(
        &mut self,
        src: &str,
        desc: Option<&str>,
        dst: Dst,
        dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeMethod<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>;

    /// Serializes a method argument.
    fn serialize_method_arg<Dst>(
        &mut self,
        src: Option<&str>,
        dst: Option<Dst>,
        pos: Option<usize>,
        lv_index: Option<usize>,
    ) -> Result<Self::SerializeMethodArg<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>;

    /// Serializes a method variable.
    fn serialize_method_var<Dst>(
        &mut self,
        src: Option<&str>,
        dst: Option<Dst>,
        lv_index: Option<usize>,
        lvt_row_index: Option<usize>,
        op_idx: Option<(usize, Option<usize>)>,
    ) -> Result<Self::SerializeMethodVar<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>;
}

impl<T> Serializer for &mut T
where
    T: Serializer,
{
    type Error = T::Error;

    const FLAT_CLASSES: bool = T::FLAT_CLASSES;

    type SerializeClass<'a>
        = T::SerializeClass<'a>
    where
        Self: 'a;

    type SerializeField<'a>
        = T::SerializeField<'a>
    where
        Self: 'a;

    type SerializeMethod<'a>
        = T::SerializeMethod<'a>
    where
        Self: 'a;

    type SerializeMethodArg<'a>
        = T::SerializeMethodArg<'a>
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = T::SerializeMethodVar<'a>
    where
        Self: 'a;

    #[inline]
    fn serialize_comment(&mut self, value: &str) -> Result<(), Self::Error> {
        T::serialize_comment(self, value)
    }

    #[inline]
    fn serialize_class<Dst>(
        &mut self,
        src: &str,
        dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        T::serialize_class(self, src, dst)
    }

    #[inline]
    fn serialize_field<Dst, DstDesc>(
        &mut self,
        src: &str,
        desc: Option<&str>,
        dst: Dst,
        dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeField<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        T::serialize_field(self, src, desc, dst, dst_desc)
    }

    #[inline]
    fn serialize_method<Dst, DstDesc>(
        &mut self,
        src: &str,
        desc: Option<&str>,
        dst: Dst,
        dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeMethod<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        T::serialize_method(self, src, desc, dst, dst_desc)
    }

    #[inline]
    fn serialize_method_arg<Dst>(
        &mut self,
        src: Option<&str>,
        dst: Option<Dst>,
        pos: Option<usize>,
        lv_index: Option<usize>,
    ) -> Result<Self::SerializeMethodArg<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        T::serialize_method_arg(self, src, dst, pos, lv_index)
    }

    #[inline]
    fn serialize_method_var<Dst>(
        &mut self,
        src: Option<&str>,
        dst: Option<Dst>,
        lv_index: Option<usize>,
        lvt_row_index: Option<usize>,
        op_idx: Option<(usize, Option<usize>)>,
    ) -> Result<Self::SerializeMethodVar<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        T::serialize_method_var(self, src, dst, lv_index, lvt_row_index, op_idx)
    }
}

/// Helper type for implementing a `Serializer` that does not support serializing one of the element types.
#[allow(missing_debug_implementations)]
pub struct Impossible<Err>(PhantomData<Err>);

impl<Err> Serializer for Impossible<Err>
where
    Err: Error,
{
    type Error = Err;

    const FLAT_CLASSES: bool = false;

    type SerializeClass<'a>
        = Self
    where
        Self: 'a;

    type SerializeField<'a>
        = Self
    where
        Self: 'a;

    type SerializeMethod<'a>
        = Self
    where
        Self: 'a;

    type SerializeMethodArg<'a>
        = Self
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = Self
    where
        Self: 'a;

    fn serialize_comment(&mut self, _value: &str) -> Result<(), Self::Error> {
        Err(Err::unsupported_type("comment"))
    }

    fn serialize_class<Dst>(
        &mut self,
        _src: &str,
        _dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        Err(Err::unsupported_type("class"))
    }

    fn serialize_field<Dst, DstDesc>(
        &mut self,
        _src: &str,
        _desc: Option<&str>,
        _dst: Dst,
        _dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeField<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        Err(Err::unsupported_type("field"))
    }

    fn serialize_method<Dst, DstDesc>(
        &mut self,
        _src: &str,
        _desc: Option<&str>,
        _dst: Dst,
        _dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeMethod<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        Err(Err::unsupported_type("method"))
    }

    fn serialize_method_arg<Dst>(
        &mut self,
        _src: Option<&str>,
        _dst: Option<Dst>,
        _pos: Option<usize>,
        _lv_index: Option<usize>,
    ) -> Result<Self::SerializeMethodArg<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        Err(Err::unsupported_type("method argument"))
    }

    fn serialize_method_var<Dst>(
        &mut self,
        _src: Option<&str>,
        _dst: Option<Dst>,
        _lv_index: Option<usize>,
        _lvt_row_index: Option<usize>,
        _op_idx: Option<(usize, Option<usize>)>,
    ) -> Result<Self::SerializeMethodVar<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        Err(Err::unsupported_type("method variable"))
    }
}

/// Helper type for implementing a `Serializer` that skips serializing one of the element types.
pub struct Skip<Err>(PhantomData<Err>);

impl<Err> Skip<Err> {
    /// Creates a new skip helper.
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<Err> Default for Skip<Err> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<Err> Debug for Skip<Err> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Skip").finish()
    }
}

impl<Err> Serializer for Skip<Err>
where
    Err: Error,
{
    type Error = Err;

    const FLAT_CLASSES: bool = false;

    type SerializeClass<'a>
        = Self
    where
        Self: 'a;

    type SerializeField<'a>
        = Self
    where
        Self: 'a;

    type SerializeMethod<'a>
        = Self
    where
        Self: 'a;

    type SerializeMethodArg<'a>
        = Self
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = Self
    where
        Self: 'a;

    #[inline]
    fn serialize_comment(&mut self, _value: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_class<Dst>(
        &mut self,
        _src: &str,
        _dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        Ok(Self::new())
    }

    #[inline]
    fn serialize_field<Dst, DstDesc>(
        &mut self,
        _src: &str,
        _desc: Option<&str>,
        _dst: Dst,
        _dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeField<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        Ok(Self::new())
    }

    #[inline]
    fn serialize_method<Dst, DstDesc>(
        &mut self,
        _src: &str,
        _desc: Option<&str>,
        _dst: Dst,
        _dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeMethod<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        Ok(Self::new())
    }

    #[inline]
    fn serialize_method_arg<Dst>(
        &mut self,
        _src: Option<&str>,
        _dst: Option<Dst>,
        _pos: Option<usize>,
        _lv_index: Option<usize>,
    ) -> Result<Self::SerializeMethodArg<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        Ok(Self::new())
    }

    #[inline]
    fn serialize_method_var<Dst>(
        &mut self,
        _src: Option<&str>,
        _dst: Option<Dst>,
        _lv_index: Option<usize>,
        _lvt_row_index: Option<usize>,
        _op_idx: Option<(usize, Option<usize>)>,
    ) -> Result<Self::SerializeMethodVar<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        Ok(Self::new())
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

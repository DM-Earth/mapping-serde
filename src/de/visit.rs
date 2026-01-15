//! Mapping visit traits.

use core::{fmt::Formatter, ops::Range};

use crate::de::Deserializer;

use super::Error;

/// Visitors for visiting a single item in a mapping file.
pub trait Visitor<'de>: Sized {
    /// The returned value type.
    type Value;

    /// Expecting item content, for error messages.
    fn expecting(&self, f: &mut Formatter<'_>) -> core::fmt::Result;

    /// Visits a possibly multi-line comment.
    fn visit_comment<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let _ = value;
        Err(E::invalid_type(
            "comment",
            core::fmt::from_fn(|f| self.expecting(f)),
        ))
    }

    /// Visits a possibly multi-line comment with borrowed lifetime.
    ///
    /// **Never** implement this method only but not `visit_comment`, unless the format deserializer guarantees so.
    /// By default this forwards to `visit_comment`.
    #[inline]
    fn visit_comment_borrowed<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_comment(value)
    }

    /// Visits a class.
    fn visit_class<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'b>,
    {
        drop(access);
        Err(A::Error::invalid_type(
            "class",
            core::fmt::from_fn(|f| self.expecting(f)),
        ))
    }

    /// Visits a class with borrowed lifetime.
    ///
    /// **Never** implement this method only but not `visit_class`, unless the format deserializer guarantees so.
    /// By default this forwards to `visit_class`.
    #[inline]
    fn visit_class_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de>,
    {
        self.visit_class(access)
    }

    /// Visits a field.
    fn visit_field<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: FieldAccess<'b>,
    {
        drop(access);
        Err(A::Error::invalid_type(
            "field",
            core::fmt::from_fn(|f| self.expecting(f)),
        ))
    }

    /// Visits a field with borrowed lifetime.
    ///
    /// **Never** implement this method only but not `visit_field`, unless the format deserializer guarantees so.
    /// By default this forwards to `visit_field`.
    #[inline]
    fn visit_field_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: FieldAccess<'de>,
    {
        self.visit_field(access)
    }

    /// Visits a method.
    fn visit_method<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodAccess<'b>,
    {
        drop(access);
        Err(A::Error::invalid_type(
            "method",
            core::fmt::from_fn(|f| self.expecting(f)),
        ))
    }

    /// Visits a method argument with borrowed lifetime.
    ///
    /// **Never** implement this method only but not `visit_method`, unless the format deserializer guarantees so.
    /// By default this forwards to `visit_method`.
    #[inline]
    fn visit_method_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodAccess<'de>,
    {
        self.visit_method(access)
    }

    /// Visits a method argument.
    fn visit_method_arg<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodArgAccess<'b>,
    {
        drop(access);
        Err(A::Error::invalid_type(
            "method argument",
            core::fmt::from_fn(|f| self.expecting(f)),
        ))
    }

    /// Visits a method argument with borrowed lifetime.
    ///
    /// **Never** implement this method only but not `visit_method_arg`, unless the format deserializer guarantees so.
    /// By default this forwards to `visit_method_arg`.
    #[inline]
    fn visit_method_arg_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodArgAccess<'de>,
    {
        self.visit_method_arg(access)
    }

    /// Visits a method variable.
    fn visit_method_var<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodVarAccess<'b>,
    {
        drop(access);
        Err(A::Error::invalid_type(
            "method variable",
            core::fmt::from_fn(|f| self.expecting(f)),
        ))
    }

    /// Visits a method variable with borrowed lifetime.
    ///
    /// **Never** implement this method only but not `visit_method_var`, unless the format deserializer guarantees so.
    /// By default this forwards to `visit_method_var`.
    #[inline]
    fn visit_method_var_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodVarAccess<'de>,
    {
        self.visit_method_var(access)
    }
}

/// Class name and content accessor.
pub trait ClassAccess<'de> {
    /// Error type.
    type Error: Error;

    /// Type of deserializer of the element's contents.
    type ContentDeserializer: Deserializer<'de, Error = Self::Error>;

    /// Source name in internal form of binary name.
    fn src(&self) -> &'de str;

    /// Destination names in internal form of binary name.
    fn dst(&self) -> impl Iterator<Item = &'de str>;

    /// Returns the content deserializer for further deserialization of this class.
    fn content(self) -> Self::ContentDeserializer;
}

/// Field name, desc and comment accessor.
pub trait FieldAccess<'de> {
    /// Error type.
    type Error: Error;

    /// Type of deserializer of the element's contents.
    type ContentDeserializer: Deserializer<'de, Error = Self::Error>;

    /// Source simple name.
    fn src(&self) -> &'de str;

    /// Destination simple names.
    fn dst(&self) -> impl Iterator<Item = &'de str>;

    /// Descriptor of this field as `FieldType` shown in JVMS.
    fn desc(&self) -> Option<&'de str>;

    /// Descriptor of this field's destinations as `FieldType` shown in JVMS.
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'de str>>;

    /// Returns the content deserializer for further deserialization of this field.
    ///
    /// It may only contain comments.
    fn content(self) -> Self::ContentDeserializer;
}

/// Method name, desc and content accessor.
pub trait MethodAccess<'de> {
    /// Error type.
    type Error: Error;

    /// Type of deserializer of the element's contents.
    type ContentDeserializer: Deserializer<'de, Error = Self::Error>;

    /// Source simple name.
    fn src(&self) -> &'de str;

    /// Destination simple names.
    fn dst(&self) -> impl Iterator<Item = &'de str>;

    /// Descriptor of this method.
    fn desc(&self) -> Option<&'de str>;

    /// Descriptor of this field's destinations.
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'de str>>;

    /// Returns the content deserializer for further deserialization of this method.
    fn content(self) -> Self::ContentDeserializer;
}

/// Method argument name, pos, slot and comment accessor.
pub trait MethodArgAccess<'de> {
    /// Error type.
    type Error: Error;

    /// Type of deserializer of the element's contents.
    type ContentDeserializer: Deserializer<'de, Error = Self::Error>;

    /// Source simple name.
    fn src(&self) -> Option<&'de str>;

    /// Destination simple names.
    fn dst(&self) -> Option<impl Iterator<Item = &'de str>>;

    /// The position of this argument starts from zero, and increase by one.
    fn pos(&self) -> Option<usize>;

    /// The local variable index of this parameter in the current method.
    ///
    /// Starts at zero for static methods and one otherwise, increase by 1, or by 2 if it's a double-wide primitive.
    ///
    /// Also known as `slot`.
    #[doc(alias = "slot")]
    fn lv_index(&self) -> Option<usize>;

    /// Returns the content deserializer for further deserialization of this method.
    ///
    /// It may only contain comments.
    fn content(self) -> Self::ContentDeserializer;
}

/// Method argument name, pos, slot and comment accessor.
pub trait MethodVarAccess<'de> {
    /// Error type.
    type Error: Error;

    /// Type of deserializer of the element's contents.
    type ContentDeserializer: Deserializer<'de, Error = Self::Error>;

    /// Source simple name.
    fn src(&self) -> Option<&'de str>;

    /// Destination simple names.
    fn dst(&self) -> Option<impl Iterator<Item = &'de str>>;

    /// The local variable index of this variable in the current method.
    ///
    /// Starts at the last parameter's slot plus wideness, and increase by 1, or by 2 if it's a double-wide primitive.
    ///
    /// Also known as `slot`.
    #[doc(alias = "slot")]
    fn lv_index(&self) -> Option<usize>;

    /// The index of variable in the method's local variable table.
    fn lvt_row_index(&self) -> Option<usize>;

    /// > Required for cases when the lvIndex alone doesn't uniquely identify a local variable.
    /// > This is the case when variables get re-defined later on, in which case most decompilers opt to
    /// > not re-define the existing var, but instead generate a new one (with both sharing the same `lv_index`).
    ///
    /// (from `mapping-io`)
    fn op_idx(&self) -> Option<Range<usize>>;

    /// Returns the content deserializer for further deserialization of this method.
    ///
    /// It may only contain comments.
    fn content(self) -> Self::ContentDeserializer;
}

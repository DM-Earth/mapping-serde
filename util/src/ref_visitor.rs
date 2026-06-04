use mapping_serde::de::Visitor;

/// A visitor wrapper which visit element by reference.
///
/// Check the presence of visitor with [`Self::visited`] before passing this to a deserializer,
/// or the implementation will return an error.
#[derive(Debug, Clone)]
pub struct RefVisitor<V> {
    inner: Option<V>,
}

impl<V> RefVisitor<V> {
    /// Creates a new reference-taking visitor from given visitor.
    #[inline]
    pub const fn new(visitor: V) -> Self {
        Self {
            inner: Some(visitor),
        }
    }

    /// Returns the underlying visitor, or `None` if already visited.
    #[inline]
    pub fn into_inner(self) -> Option<V> {
        self.inner
    }

    /// Whether an visit has already occurred.
    #[inline]
    pub fn visited(&self) -> bool {
        self.inner.is_some()
    }

    fn get_or_err<E>(&mut self) -> Result<V, E>
    where
        E: mapping_serde::de::Error,
    {
        self.inner
            .take()
            .ok_or_else(|| E::custom("already visited"))
    }
}

impl<'de, V> Visitor<'de> for &mut RefVisitor<V>
where
    V: Visitor<'de>,
{
    type Value = V::Value;

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.inner {
            Some(ref inner) => inner.expecting(f),
            None => write!(f, "(end visit)"),
        }
    }

    #[inline]
    fn visit_comment<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: mapping_serde::de::Error,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_comment(value))
    }

    #[inline]
    fn visit_comment_borrowed<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: mapping_serde::de::Error,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_comment_borrowed(value))
    }

    #[inline]
    fn visit_class<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::ClassAccess<'de, 'b>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_class(access))
    }

    #[inline]
    fn visit_class_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::ClassAccess<'de, 'de>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_class_borrowed(access))
    }

    #[inline]
    fn visit_field<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::FieldAccess<'de, 'b>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_field(access))
    }

    #[inline]
    fn visit_field_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::FieldAccess<'de, 'de>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_field_borrowed(access))
    }

    #[inline]
    fn visit_method<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodAccess<'de, 'b>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_method(access))
    }

    #[inline]
    fn visit_method_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodAccess<'de, 'de>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_method_borrowed(access))
    }

    #[inline]
    fn visit_method_arg<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodArgAccess<'de, 'b>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_method_arg(access))
    }

    #[inline]
    fn visit_method_arg_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodArgAccess<'de, 'de>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_method_arg_borrowed(access))
    }

    #[inline]
    fn visit_method_var<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodVarAccess<'de, 'b>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_method_var(access))
    }

    #[inline]
    fn visit_method_var_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodVarAccess<'de, 'de>,
    {
        self.get_or_err()
            .and_then(|inner| inner.visit_method_var_borrowed(access))
    }
}

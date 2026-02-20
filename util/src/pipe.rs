use mapping_serde::{Deserializer, Serializer};

#[inline]
fn se2de<DE, SE>(err: SE) -> DE
where
    DE: mapping_serde::de::Error,
    SE: mapping_serde::ser::Error,
{
    DE::custom(format_args!("serialization error: {err}"))
}

struct SerializeVisitor<S> {
    inner: S,
}

impl<'de, S> mapping_serde::de::Visitor<'de> for SerializeVisitor<S>
where
    S: Serializer,
{
    type Value = ();

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "an element supported by the serializer")
    }

    #[inline]
    fn visit_class<'b, A>(mut self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::ClassAccess<'de, 'b>,
    {
        self.inner
            .serialize_class(access.src(), access.dst())
            .and_then(|content_ser| pipe_into_serr(access.content(), content_ser))
            .map_err(se2de)
    }

    #[inline]
    fn visit_comment<E>(mut self, value: &str) -> Result<Self::Value, E>
    where
        E: mapping_serde::de::Error,
    {
        self.inner.serialize_comment(value).map_err(se2de)
    }

    #[inline]
    fn visit_field<'b, A>(mut self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::FieldAccess<'de, 'b>,
    {
        self.inner
            .serialize_field(access.src(), access.desc(), access.dst(), access.dst_desc())
            .and_then(|content_ser| pipe_into_serr(access.content(), content_ser))
            .map_err(se2de)
    }

    #[inline]
    fn visit_method<'b, A>(mut self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodAccess<'de, 'b>,
    {
        self.inner
            .serialize_method(access.src(), access.desc(), access.dst(), access.dst_desc())
            .and_then(|content_ser| pipe_into_serr(access.content(), content_ser))
            .map_err(se2de)
    }

    #[inline]
    fn visit_method_arg<'b, A>(mut self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodArgAccess<'de, 'b>,
    {
        self.inner
            .serialize_method_arg(access.src(), access.dst(), access.pos(), access.lv_index())
            .and_then(|content_ser| pipe_into_serr(access.content(), content_ser))
            .map_err(se2de)
    }

    #[inline]
    fn visit_method_var<'b, A>(mut self, access: A) -> Result<Self::Value, A::Error>
    where
        A: mapping_serde::de::MethodVarAccess<'de, 'b>,
    {
        self.inner
            .serialize_method_var(
                access.src(),
                access.dst(),
                access.lv_index(),
                access.lvt_row_index(),
                access.op_idx(),
            )
            .and_then(|content_ser| pipe_into_serr(access.content(), content_ser))
            .map_err(se2de)
    }
}

pub(crate) fn pipe_into<'de, D, S>(mut deserializer: D, mut serializer: S) -> Result<(), D::Error>
where
    S: Serializer,
    D: Deserializer<'de>,
{
    loop {
        let ret = deserializer.deserialize_any(SerializeVisitor {
            inner: &mut serializer,
        })?;
        if ret.is_none() {
            return Ok(());
        }
    }
}

#[inline]
fn pipe_into_serr<'de, D, S>(deserializer: D, serializer: S) -> Result<(), S::Error>
where
    S: Serializer,
    D: Deserializer<'de>,
{
    pipe_into(deserializer, serializer).map_err(<S::Error as mapping_serde::ser::Error>::custom)
}

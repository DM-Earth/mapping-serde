use std::{fmt::Display, io::Write, marker::PhantomData};

use mapping_serde::ser::{self, Error as _, Impossible};

use crate::{Error, INDENT, PROPERTY_ESCAPED_NAMES};

trait ContentSpec {
    const COMMENT: bool = false;
    const CLASS: bool = false;
    const FIELD: bool = false;
    const METHOD: bool = false;
    const METHOD_PARAM: bool = false;
    const METHOD_VAR: bool = false;
}

macro_rules! specs {
    ($($v:vis $s:ident: $($k:ident),*;)*) => {
        $(
        #[derive(Debug)]
        $v struct $s;
        impl ContentSpec for $s {
            $(const $k: bool = true;)*
        }
        )*
    };
}

struct Indent(usize);

impl Display for Indent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const HAYSTACK: &str = if let Ok(s) = str::from_utf8(&[INDENT; 8]) {
            s
        } else {
            unreachable!()
        };

        for _ in 0..(self.0 / HAYSTACK.len()) {
            f.write_str(HAYSTACK)?;
        }
        f.write_str(&HAYSTACK[..self.0 % HAYSTACK.len()])
    }
}

fn write_many<W, I>(mut writer: W, iter: I, escape_names: bool) -> std::io::Result<()>
where
    I: IntoIterator<Item: AsRef<str>>,
    W: Write,
{
    for v in iter.into_iter() {
        write!(writer, "\t{}", MaybeEscaped(v.as_ref(), escape_names))?;
    }
    Ok(())
}

// (_, escaped_names)
struct MaybeEscaped<'a>(&'a str, bool);

impl Display for MaybeEscaped<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.1 {
            write!(f, "{}", self.0.escape_default())
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/// Tiny2 mapping file serializer.
#[derive(Debug)]
pub struct Serializer<W> {
    escaped_names: bool,
    writer: W,
}

impl<W> Serializer<W>
where
    W: Write,
{
    /// Creates a new Tiny2 serializer from the given writer.
    #[allow(clippy::missing_errors_doc)] // io errors. omitted
    pub fn new<Dst, P, PKey, PValue>(
        mut writer: W,
        src: &str,
        dst: Dst,
        minor_version: u16,
        props: P,
    ) -> Result<Self, Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        P: IntoIterator<Item = (PKey, Option<PValue>)>,
        PKey: AsRef<str>,
        PValue: AsRef<str>,
    {
        write!(writer, "tiny\t2\t{}\t{}", minor_version, src)?;
        write_many(&mut writer, dst, false)?;
        writeln!(writer)?;

        let mut escaped_names = false;
        let props = props.into_iter().inspect(|(k, _)| {
            if k.as_ref() == PROPERTY_ESCAPED_NAMES {
                escaped_names = true
            }
        });

        for (key, val) in props {
            if let Some(val) = val {
                writeln!(
                    writer,
                    "\t{}\t{}",
                    key.as_ref(),
                    val.as_ref().escape_default(),
                )?;
            } else {
                writeln!(writer, "\t{}", key.as_ref())?;
            }
        }

        Ok(Self {
            escaped_names,
            writer,
        })
    }
}

impl<W> ser::Serializer for Serializer<W>
where
    W: Write,
{
    type Error = Error;

    type SerializeClass<'a>
        = ContentSerializer<'a, W, ClassSpec>
    where
        Self: 'a;

    type SerializeField<'a>
        = Impossible<Error>
    where
        Self: 'a;

    type SerializeMethod<'a>
        = Impossible<Error>
    where
        Self: 'a;

    type SerializeMethodArg<'a>
        = Impossible<Error>
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = Impossible<Error>
    where
        Self: 'a;

    fn serialize_comment(&mut self, _value: &str) -> Result<(), Self::Error> {
        Err(Error::unsupported_type("comment"))
    }

    fn serialize_class<Dst>(
        &mut self,
        src: &str,
        dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        serialize_class_impl(src, dst, 0, &mut self.writer, self.escaped_names)?;
        Ok(ContentSerializer {
            indent: 1,
            writer: &mut self.writer,
            escaped_names: self.escaped_names,
            spec: PhantomData::<ClassSpec>,
        })
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
        Err(Error::unsupported_type("field"))
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
        Err(Error::unsupported_type("method"))
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
        Err(Error::unsupported_type("method parameter"))
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
        Err(Error::unsupported_type("method variable"))
    }
}

mod _priv {
    use super::*;

    #[derive(Debug)]
    pub struct ContentSerializer<'a, W, Spec> {
        pub(super) indent: usize,
        pub(super) writer: &'a mut W,
        pub(super) escaped_names: bool,
        pub(super) spec: PhantomData<Spec>,
    }

    specs! {
        pub ClassSpec: COMMENT, METHOD, FIELD;
        pub FieldSpec: COMMENT;
        pub MethodSpec: COMMENT, METHOD_PARAM, METHOD_VAR;
        pub MethodParamSpec: COMMENT;
        pub MethodVarSpec: COMMENT;
    }
}

use _priv::*;

impl<W, Spec1> ContentSerializer<'_, W, Spec1> {
    fn fork<Spec2>(&mut self) -> ContentSerializer<'_, W, Spec2> {
        ContentSerializer {
            indent: self.indent + 1,
            writer: &mut *self.writer,
            escaped_names: self.escaped_names,
            spec: PhantomData,
        }
    }
}

fn serialize_class_impl<Dst, W>(
    src: &str,
    dst: Dst,
    indent: usize,
    mut writer: W,
    escaped_names: bool,
) -> Result<(), Error>
where
    Dst: IntoIterator<Item: AsRef<str>>,
    W: Write,
{
    write!(
        writer,
        "{}c\t{}",
        Indent(indent),
        MaybeEscaped(src, escaped_names)
    )?;
    write_many(&mut writer, dst, escaped_names)?;
    writeln!(writer)?;
    Ok(())
}

impl<W, S> ser::Serializer for ContentSerializer<'_, W, S>
where
    S: ContentSpec,
    W: Write,
{
    type Error = Error;

    type SerializeClass<'a>
        = ContentSerializer<'a, W, ClassSpec>
    where
        Self: 'a;

    type SerializeField<'a>
        = ContentSerializer<'a, W, FieldSpec>
    where
        Self: 'a;

    type SerializeMethod<'a>
        = ContentSerializer<'a, W, MethodSpec>
    where
        Self: 'a;

    type SerializeMethodArg<'a>
        = ContentSerializer<'a, W, MethodParamSpec>
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = ContentSerializer<'a, W, MethodVarSpec>
    where
        Self: 'a;

    fn serialize_comment(&mut self, value: &str) -> Result<(), Self::Error> {
        if S::COMMENT {
            writeln!(
                self.writer,
                "{}c\t{}",
                Indent(self.indent),
                value.escape_default()
            )
            .map_err(Into::into)
        } else {
            Err(Error::unsupported_type("comment"))
        }
    }

    fn serialize_class<Dst>(
        &mut self,
        src: &str,
        dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        if S::CLASS {
            serialize_class_impl(src, dst, self.indent, &mut *self.writer, self.escaped_names)?;
            Ok(self.fork())
        } else {
            Err(Error::unsupported_type("class"))
        }
    }

    fn serialize_field<Dst, DstDesc>(
        &mut self,
        src: &str,
        desc: Option<&str>,
        dst: Dst,
        _dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeField<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        if S::FIELD {
            let desc = desc.ok_or_else(|| Error::missing_field("field-desc-a"))?;
            write!(
                self.writer,
                "{}f\t{}\t{}",
                Indent(self.indent),
                desc,
                MaybeEscaped(src, self.escaped_names)
            )?;
            write_many(&mut *self.writer, dst, self.escaped_names)?;
            writeln!(self.writer)?;
            Ok(self.fork())
        } else {
            Err(Error::unsupported_type("field"))
        }
    }

    fn serialize_method<Dst, DstDesc>(
        &mut self,
        src: &str,
        desc: Option<&str>,
        dst: Dst,
        _dst_desc: Option<DstDesc>,
    ) -> Result<Self::SerializeMethod<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        DstDesc: IntoIterator<Item: AsRef<str>>,
    {
        if S::METHOD {
            let desc = desc.ok_or_else(|| Error::missing_field("method-desc-a"))?;
            write!(
                self.writer,
                "{}m\t{}\t{}",
                Indent(self.indent),
                desc,
                MaybeEscaped(src, self.escaped_names)
            )?;
            write_many(&mut *self.writer, dst, self.escaped_names)?;
            writeln!(self.writer)?;
            Ok(self.fork())
        } else {
            Err(Error::unsupported_type("field"))
        }
    }

    fn serialize_method_arg<Dst>(
        &mut self,
        src: Option<&str>,
        dst: Option<Dst>,
        _pos: Option<usize>,
        lv_index: Option<usize>,
    ) -> Result<Self::SerializeMethodArg<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        if S::METHOD_PARAM {
            let lv_index = lv_index.ok_or_else(|| Error::missing_field("lv-index"))?;
            write!(self.writer, "{}p\t{}", Indent(self.indent), lv_index)?;
            if let Some(src) = src {
                write!(self.writer, "\t{}", MaybeEscaped(src, self.escaped_names))?;
                if let Some(dst) = dst {
                    write_many(&mut *self.writer, dst, self.escaped_names)?;
                }
            }
            writeln!(self.writer)?;
            Ok(self.fork())
        } else {
            Err(Error::unsupported_type("method parameter"))
        }
    }

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
        if S::METHOD_VAR {
            let lv_index = lv_index.ok_or_else(|| Error::missing_field("lv-index"))?;
            let (lv_start_offset, _) =
                op_idx.ok_or_else(|| Error::missing_field("lv-start-offset"))?;
            write!(
                self.writer,
                "{}v\t{}\t{}\t",
                Indent(self.indent),
                lv_index,
                lv_start_offset,
            )?;
            if let Some(lvt) = lvt_row_index {
                write!(self.writer, "{lvt}")?;
            } else {
                write!(self.writer, "-1")?;
            }
            if let Some(src) = src {
                write!(self.writer, "\t{}", MaybeEscaped(src, self.escaped_names))?;
                if let Some(dst) = dst {
                    write_many(&mut *self.writer, dst, self.escaped_names)?;
                }
            }
            writeln!(self.writer)?;
            Ok(self.fork())
        } else {
            Err(Error::unsupported_type("method variable"))
        }
    }
}

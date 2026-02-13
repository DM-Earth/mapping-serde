use std::io::Write;

use mapping_serde::ser::{self, Error as _, Impossible, Skip};
use smol_str::SmolStr;

use crate::Error;

/// A Tiny1 mapping file serializer.
#[derive(Debug)]
pub struct Serializer<W> {
    writer: W,
}

impl<W> Serializer<W>
where
    W: Write,
{
    /// Creates a new Tiny1 serializer from the given writer.
    #[allow(clippy::missing_errors_doc)] // io errors. omitted
    pub fn new<Dst, P, PKey, PValue>(
        mut writer: W,
        src: &str,
        dst: Dst,
        props: P,
    ) -> Result<Self, Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
        P: IntoIterator<Item = (PKey, Option<PValue>)>,
        PKey: AsRef<str>,
        PValue: AsRef<str>,
    {
        write!(writer, "v1\t{}", src)?;
        for dst in dst {
            write!(writer, "\t{}", dst.as_ref())?;
        }
        writeln!(writer)?;

        for (key, val) in props {
            write!(writer, "# {}", key.as_ref())?;
            if let Some(val) = val {
                write!(writer, " {}", val.as_ref())?;
            }
            writeln!(writer)?;
        }

        Ok(Self { writer })
    }
}

impl<W> ser::Serializer for Serializer<W>
where
    W: Write,
{
    type Error = Error;

    type SerializeClass<'a>
        = ContentSerializer<'a, W>
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
        = Skip<Error>
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = Skip<Error>
    where
        Self: 'a;

    #[inline]
    fn serialize_comment(&mut self, _value: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    fn serialize_class<Dst>(
        &mut self,
        src: &str,
        dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        write!(self.writer, "CLASS\t{}", src)?;
        for ns in dst {
            write!(self.writer, "\t{}", ns.as_ref())?;
        }
        writeln!(self.writer)?;

        Ok(ContentSerializer {
            parent: src.into(),
            writer: &mut self.writer,
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
        Ok(Skip::new())
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
        Ok(Skip::new())
    }
}

#[derive(Debug)]
pub struct ContentSerializer<'a, W> {
    parent: SmolStr,
    writer: &'a mut W,
}

impl<W> ser::Serializer for ContentSerializer<'_, W>
where
    W: Write,
{
    type Error = Error;

    type SerializeClass<'a>
        = Impossible<Error>
    where
        Self: 'a;

    type SerializeField<'a>
        = Skip<Error>
    where
        Self: 'a;

    type SerializeMethod<'a>
        = Skip<Error>
    where
        Self: 'a;

    type SerializeMethodArg<'a>
        = Skip<Error>
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = Skip<Error>
    where
        Self: 'a;

    #[inline]
    fn serialize_comment(&mut self, _value: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    fn serialize_class<Dst>(
        &mut self,
        _src: &str,
        _dst: Dst,
    ) -> Result<Self::SerializeClass<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        Err(Error::unsupported_type("class"))
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
        write!(
            self.writer,
            "FIELD\t{}\t{}\t{}",
            self.parent,
            desc.ok_or_else(|| Error::missing_field("field-desc-a"))?,
            src
        )?;
        for ns in dst {
            write!(self.writer, "\t{}", ns.as_ref())?;
        }
        writeln!(self.writer)?;
        Ok(Skip::new())
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
        write!(
            self.writer,
            "METHOD\t{}\t{}\t{}",
            self.parent,
            desc.ok_or_else(|| Error::missing_field("method-desc-a"))?,
            src
        )?;
        for ns in dst {
            write!(self.writer, "\t{}", ns.as_ref())?;
        }
        writeln!(self.writer)?;
        Ok(Skip::new())
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
        Ok(Skip::new())
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
        Ok(Skip::new())
    }
}

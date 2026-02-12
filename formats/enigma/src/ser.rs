use std::{fmt::Display, io::Write};

use io_util::MaybeMut;
use mapping_serde::ser::{self, Error as _};

use crate::{Error, INDENT};

/// Enigma mapping file serializer.
#[derive(Debug)]
pub struct Serializer<'a, W> {
    indent: usize,
    writer: MaybeMut<'a, W>,
}

impl<W> Serializer<'_, W> {
    /// Creates a new Enigma serializer from the given writer.
    #[inline]
    pub const fn new(writer: W) -> Self {
        Self {
            indent: 0,
            writer: MaybeMut::Owned(writer),
        }
    }

    fn fork(&mut self) -> Serializer<'_, W> {
        Serializer {
            indent: self.indent + 1,
            writer: self.writer.reclaim(),
        }
    }
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

impl<W> ser::Serializer for Serializer<'_, W>
where
    W: Write,
{
    type Error = Error;

    type SerializeClass<'a>
        = Serializer<'a, W>
    where
        Self: 'a;

    type SerializeField<'a>
        = Serializer<'a, W>
    where
        Self: 'a;

    type SerializeMethod<'a>
        = Serializer<'a, W>
    where
        Self: 'a;

    type SerializeMethodArg<'a>
        = Serializer<'a, W>
    where
        Self: 'a;

    type SerializeMethodVar<'a>
        = Serializer<'a, W>
    where
        Self: 'a;

    fn serialize_comment(&mut self, value: &str) -> Result<(), Self::Error> {
        for line in value.lines() {
            self.writer
                .write_fmt(format_args!("{}COMMENT {line}\n", Indent(self.indent)))?
        }
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
        let dst = dst.into_iter().next();
        self.writer.write_fmt(format_args!(
            "{}CLASS {} {}\n",
            Indent(self.indent),
            src,
            dst.as_ref().map_or("-", AsRef::as_ref),
        ))?;
        Ok(self.fork())
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
        let dst = dst.into_iter().next();
        let desc = desc.ok_or_else(|| Error::missing_field("desc"))?;
        self.writer.write_fmt(format_args!(
            "{}FIELD {} {} {}\n",
            Indent(self.indent),
            src,
            dst.as_ref().map_or("-", AsRef::as_ref),
            desc
        ))?;
        Ok(self.fork())
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
        let dst = dst.into_iter().next();
        let desc = desc.ok_or_else(|| Error::missing_field("desc"))?;
        self.writer.write_fmt(format_args!(
            "{}METHOD {} {} {}\n",
            Indent(self.indent),
            src,
            dst.as_ref().map_or("-", AsRef::as_ref),
            desc
        ))?;
        Ok(self.fork())
    }

    fn serialize_method_arg<Dst>(
        &mut self,
        _src: Option<&str>,
        dst: Option<Dst>,
        _pos: Option<usize>,
        lv_index: Option<usize>,
    ) -> Result<Self::SerializeMethodArg<'_>, Self::Error>
    where
        Dst: IntoIterator<Item: AsRef<str>>,
    {
        let dst = dst.and_then(|it| it.into_iter().next());
        let lv_index = lv_index.map(|i| i as i32).unwrap_or(-1);
        self.writer.write_fmt(format_args!(
            "{}ARG {} {}\n",
            Indent(self.indent),
            lv_index,
            dst.as_ref().map_or("-", AsRef::as_ref),
        ))?;
        Ok(self.fork())
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
        Err(Error {
            kind: crate::ErrorKind::Unsupported("method variable"),
            line: 0,
            col: 0,
        })
    }
}

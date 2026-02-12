//! Enigma mapping file deserialization.

use io_util::{
    ColumnRead, ColumnReadAdapter, ColumnReader, IoReader, MaybeBorrowed, MaybeMut, SliceReader,
    SmolCowStr,
};
use mapping_serde::de::{self, Error as _};
use smol_str::ToSmolStr as _;

use crate::{Error, INDENT, SEPARATOR};

/// Enigma mapping file deserializer.
///
/// Access Modifiers are not supported.
#[derive(Debug)]
pub struct Deserializer<'a, R> {
    src: &'a str,
    dst: &'a str,
    indent: usize,
    aborted: bool,
    read: MaybeMut<'a, ColumnReader<R>>,
}

impl<'a, R> Deserializer<'a, R> {
    /// Creates a new Enigma deserializer.
    pub fn new(src: &'a str, dst: &'a str, read: R) -> Self {
        Self {
            src,
            dst,
            indent: 0,
            aborted: false,
            read: MaybeMut::Owned(ColumnReader::new(INDENT, SEPARATOR, read)),
        }
    }
}

impl<'a, 'slice> Deserializer<'a, ColumnReadAdapter<Box<SliceReader<'slice>>>> {
    /// Creates a new deserializer from the given slice.
    ///
    /// Note that this involves heap allocation. To avoid it, pin a reader in the stack and
    /// create a deserializer with [`Self::new`].
    pub fn from_slice(src: &'a str, dst: &'a str, slice: &'slice [u8]) -> Self {
        Self::new(
            src,
            dst,
            ColumnReadAdapter::new(Box::new(SliceReader::new(slice))),
        )
    }
}

impl<'a, R> Deserializer<'a, ColumnReadAdapter<Box<IoReader<R>>>>
where
    R: Unpin,
{
    /// Creates a new deserializer from the given I/O reader.
    /// The reader should implement [`std::io::BufRead`].
    ///
    /// Note that this involves heap allocation. To avoid it, pin a reader in the stack and
    /// create a deserializer with [`Self::new`].
    pub fn from_reader(src: &'a str, dst: &'a str, reader: R) -> Self {
        Self::new(
            src,
            dst,
            ColumnReadAdapter::new(Box::new(IoReader::new(reader))),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DescribedKind {
    Method,
    Field,
}

#[inline]
fn parse_bytes<'a, 'b>(
    b: Option<MaybeBorrowed<'a, 'b, [u8]>>,
    section: &str,
) -> Result<MaybeBorrowed<'a, 'b, str>, Error> {
    b.ok_or_else(|| Error::missing_field(section))
        .and_then(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
}

#[inline]
fn parse_bytes_optional<'a, 'b>(
    b: Option<MaybeBorrowed<'a, 'b, [u8]>>,
    section: &str,
) -> Result<Option<MaybeBorrowed<'a, 'b, str>>, Error> {
    b.ok_or_else(|| Error::missing_field(section))
        .map(|b| (&*b != b"-").then_some(b))
        .and_then(|b| {
            b.map(|b| b.try_map(str::from_utf8).map_err(Into::into))
                .transpose()
        })
}

impl<'de, R> Deserializer<'_, R>
where
    R: ColumnRead<'de>,
{
    fn deserialize_impl<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Error>
    where
        V: de::Visitor<'de>,
    {
        if self.aborted {
            return Ok(None);
        }
        if self.read.is_fresh_line() {
            if self.read.this_indent() != Some(self.indent) {
                self.aborted = true;
                return Ok(None);
            }
        } else {
            loop {
                let indent = self.read.next_line()?;
                if indent.is_none_or(|i| i < self.indent) {
                    self.aborted = true;
                    return Ok(None);
                } else if indent.is_some_and(|i| i > self.indent) {
                    continue;
                } else {
                    debug_assert_eq!(Some(self.indent), indent);
                    break;
                }
            }
        }

        let ty = self.read.read_col()?;
        let ty = ty.as_deref().unwrap_or_default();
        if ty.starts_with(b"#") {
            // it is literally a comment
            return self.deserialize_impl(visitor);
        }
        match ty {
            b"CLASS" => self.deserialize_class_impl(visitor),
            b"COMMENT" => self.deserialize_comment_impl(visitor),
            b"FIELD" => self.deserialize_described_impl(visitor, DescribedKind::Field),
            b"METHOD" => self.deserialize_described_impl(visitor, DescribedKind::Method),
            b"ARG" => self.deserialize_arg_impl(visitor),
            _ => Err(Error::invalid_type(
                String::from_utf8_lossy(ty),
                "CLASS, FIELD, METHOD, ARG, COMMENT",
            )),
        }
        .map(Some)
    }

    fn deserialize_arg_impl<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        let [lv_index, dst] = self.read.read_cols()?;
        let lv_index = parse_bytes(lv_index, "lv-index")?
            .parse::<isize>()
            .ok()
            .filter(|&i| i > -1)
            .ok_or_else(|| Error::custom(format_args!("invalid parameter lv-index")))?;
        let lv_index = (lv_index >= 0).then_some(lv_index as usize);
        let dst = parse_bytes_optional(dst, "name-b")?;

        struct Arg<'env, 'd, R> {
            dst: Option<&'env str>,
            lv_index: Option<usize>,
            deser: Deserializer<'d, R>,
        }

        impl<'de, 'env, 'd, R> de::MethodArgAccess<'de, 'env> for Arg<'env, 'd, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;
            type ContentDeserializer = Deserializer<'d, R>;

            #[inline]
            fn src(&self) -> Option<&'env str> {
                None
            }
            #[inline]
            fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
                self.dst.map(std::iter::once)
            }
            #[inline]
            fn pos(&self) -> Option<usize> {
                None
            }
            #[inline]
            fn lv_index(&self) -> Option<usize> {
                self.lv_index
            }
            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        match dst {
            Some(MaybeBorrowed::Short(dst)) => {
                let dst = dst.to_smolstr();
                visitor.visit_method_arg(Arg {
                    dst: Some(&dst),
                    lv_index,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        indent: self.indent + 1,
                        aborted: false,
                        read: self.read.reclaim(),
                    },
                })
            }
            dst @ (None | Some(MaybeBorrowed::Borrowed(_))) => {
                visitor.visit_method_arg_borrowed(Arg {
                    dst: dst.map(|b| b.as_borrowed().unwrap()),
                    lv_index,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        indent: self.indent + 1,
                        aborted: false,
                        read: self.read.reclaim(),
                    },
                })
            }
        }
    }

    fn deserialize_comment_impl<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        if let Some(comment) = self.read.this_line() {
            let comment = comment.try_map(str::from_utf8)?;
            match comment {
                MaybeBorrowed::Short(v) => visitor.visit_comment(v),
                MaybeBorrowed::Borrowed(v) => visitor.visit_comment_borrowed(v),
            }
        } else {
            visitor.visit_comment_borrowed("")
        }
    }

    fn deserialize_described_impl<V>(
        &mut self,
        visitor: V,
        kind: DescribedKind,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        let [src, dst, desc] = self.read.read_cols()?;
        let src = parse_bytes(src, "name-a")?;
        let dst = parse_bytes_optional(dst, "name-b")?;
        let desc = parse_bytes(desc, "desc-a")?;

        struct Described<'env, 'd, R> {
            src: &'env str,
            dst: Option<&'env str>,
            desc: &'env str,
            deser: Deserializer<'d, R>,
        }

        impl<'de, 'env, 'd, R> de::FieldAccess<'de, 'env> for Described<'env, 'd, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;
            type ContentDeserializer = Deserializer<'d, R>;

            #[inline]
            fn src(&self) -> &'env str {
                self.src
            }
            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'env str> {
                self.dst.into_iter()
            }
            #[inline]
            fn desc(&self) -> Option<&'env str> {
                Some(self.desc)
            }
            #[inline]
            fn dst_desc(&self) -> Option<impl Iterator<Item = &'env str>> {
                None::<std::iter::Empty<_>>
            }
            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        impl<'de, 'env, 'd, R> de::MethodAccess<'de, 'env> for Described<'env, 'd, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;
            type ContentDeserializer = Deserializer<'d, R>;

            #[inline]
            fn src(&self) -> &'env str {
                self.src
            }
            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'env str> {
                self.dst.into_iter()
            }
            #[inline]
            fn desc(&self) -> Option<&'env str> {
                Some(self.desc)
            }
            #[inline]
            fn dst_desc(&self) -> Option<impl Iterator<Item = &'env str>> {
                None::<std::iter::Empty<_>>
            }
            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        match (&src, dst, &desc) {
            (
                MaybeBorrowed::Borrowed(src),
                dst @ (Some(MaybeBorrowed::Borrowed(_)) | None),
                MaybeBorrowed::Borrowed(desc),
            ) => {
                let described = Described {
                    src,
                    dst: dst.map(|b| b.as_borrowed().unwrap()),
                    desc,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        indent: self.indent + 1,
                        aborted: false,
                        read: self.read.reclaim(),
                    },
                };
                match kind {
                    DescribedKind::Method => visitor.visit_method_borrowed(described),
                    DescribedKind::Field => visitor.visit_field_borrowed(described),
                }
            }
            _ => {
                let (src, dst, desc) = (
                    SmolCowStr::from(src),
                    dst.map(SmolCowStr::from),
                    SmolCowStr::from(desc),
                );
                let described = Described {
                    src: &src,
                    dst: dst.as_deref(),
                    desc: &desc,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        indent: self.indent + 1,
                        aborted: false,
                        read: self.read.reclaim(),
                    },
                };
                match kind {
                    DescribedKind::Method => visitor.visit_method(described),
                    DescribedKind::Field => visitor.visit_field(described),
                }
            }
        }
    }

    fn deserialize_class_impl<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        let [src, dst] = self.read.read_cols()?;
        let src = parse_bytes(src, "class-name-a")?;
        let dst = parse_bytes_optional(dst, "class-name-b")?;

        struct Class<'env, 'd, R> {
            src: &'env str,
            dst: Option<&'env str>,
            deser: Deserializer<'d, R>,
        }

        impl<'de, 'env, 'd, R> de::ClassAccess<'de, 'env> for Class<'env, 'd, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;
            type ContentDeserializer = Deserializer<'d, R>;

            #[inline]
            fn src(&self) -> &'env str {
                self.src
            }
            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'env str> {
                self.dst.into_iter()
            }
            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        match (src, dst) {
            (MaybeBorrowed::Borrowed(src), dst @ (Some(MaybeBorrowed::Borrowed(_)) | None)) => {
                visitor.visit_class_borrowed(Class {
                    src,
                    dst: dst.map(|b| b.as_borrowed().unwrap()),
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        indent: self.indent + 1,
                        aborted: false,
                        read: self.read.reclaim(),
                    },
                })
            }
            _ => {
                let (src, dst) = (SmolCowStr::from(src), dst.map(SmolCowStr::from));
                visitor.visit_class(Class {
                    src: &src,
                    dst: dst.as_deref(),
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        indent: self.indent + 1,
                        aborted: false,
                        read: self.read.reclaim(),
                    },
                })
            }
        }
    }
}

impl<'de, R> mapping_serde::Deserializer<'de> for Deserializer<'_, R>
where
    R: ColumnRead<'de>,
{
    type Error = Error;

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_impl(visitor)
            .map_err(|err| err.with_loc(self.read.line(), self.read.col()))
    }

    #[inline]
    fn src_namespace(&self) -> &str {
        self.src
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.dst)
    }
}

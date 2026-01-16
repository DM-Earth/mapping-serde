//! Enigma mapping file deserialization.

use mapping_serde::de::{self, Error as _};
use smol_str::ToSmolStr as _;

use crate::{
    Error,
    io::{ColumnRead, ColumnReader, MaybeBorrowed, MaybeMut, SmolCowStr},
};

/// Enigma mapping file deserializer.
#[derive(Debug)]
pub struct Deserializer<'a, R> {
    src: &'a str,
    dst: &'a str,
    ident: usize,
    last_abort_ident: MaybeMut<'a, Option<usize>>,
    read: ColumnReader<R>,
}

impl<'a, R> Deserializer<'a, R> {
    /// Creates a new Enigma deserializer.
    pub fn new(src: &'a str, dst: &'a str, read: R) -> Self {
        Self {
            src,
            dst,
            ident: 0,
            last_abort_ident: MaybeMut::Owned(None),
            read: ColumnReader::new(b'\t', b' ', read),
        }
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
    b.ok_or_else(|| Error::custom(format_args!("missing {section} section")))
        .and_then(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
}

impl<'de, R> Deserializer<'_, R>
where
    R: ColumnRead<'de>,
{
    fn deserialize_impl<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Error>
    where
        V: de::Visitor<'de>,
    {
        if !self.read.is_fresh_line() && self.last_abort_ident.is_none_or(|i| i != self.ident) {
            loop {
                let ident = self.read.next_line()?;
                if ident.is_none_or(|i| i < self.ident) {
                    *self.last_abort_ident = Some(self.ident);
                    return Ok(None);
                } else if ident.is_some_and(|i| i > self.ident) {
                    continue;
                } else {
                    *self.last_abort_ident = None;
                    break;
                }
            }
        }

        let ty = self.read.read_col()?;
        let ty = ty.as_deref().unwrap_or_default();
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
            .parse::<usize>()
            .map_err(|err| Error::custom(format_args!("invalid parameter lv-index: {err}")))?;
        let dst = parse_bytes(dst, "name-b")?;

        struct Arg<'env, 'd, R> {
            dst: &'env str,
            lv_index: usize,
            deser: Deserializer<'d, &'d mut R>,
        }

        impl<'de, 'env, 'd, R> de::MethodArgAccess<'de, 'env> for Arg<'env, 'd, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;
            type ContentDeserializer = Deserializer<'d, &'d mut R>;

            #[inline]
            fn src(&self) -> Option<&'env str> {
                None
            }
            #[inline]
            fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
                Some(std::iter::once(self.dst))
            }
            #[inline]
            fn pos(&self) -> Option<usize> {
                None
            }
            #[inline]
            fn lv_index(&self) -> Option<usize> {
                Some(self.lv_index)
            }
            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        match dst {
            MaybeBorrowed::Short(dst) => {
                let dst = dst.to_smolstr();
                visitor.visit_method_arg(Arg {
                    dst: &dst,
                    lv_index,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        ident: self.ident + 1,
                        last_abort_ident: self.last_abort_ident.reclaim(),
                        read: self.read.get_mut(),
                    },
                })
            }
            MaybeBorrowed::Borrowed(dst) => visitor.visit_method_arg_borrowed(Arg {
                dst,
                lv_index,
                deser: Deserializer {
                    src: self.src,
                    dst: self.dst,
                    ident: self.ident + 1,
                    last_abort_ident: self.last_abort_ident.reclaim(),
                    read: self.read.get_mut(),
                },
            }),
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
        let dst = parse_bytes(dst, "name-b")?;
        let desc = parse_bytes(desc, "desc-a")?;

        struct Described<'env, 'd, R> {
            src: &'env str,
            dst: &'env str,
            desc: &'env str,
            deser: Deserializer<'d, &'d mut R>,
        }

        impl<'de, 'env, 'd, R> de::FieldAccess<'de, 'env> for Described<'env, 'd, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;
            type ContentDeserializer = Deserializer<'d, &'d mut R>;

            #[inline]
            fn src(&self) -> &'env str {
                self.src
            }
            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'env str> {
                std::iter::once(self.dst)
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
            type ContentDeserializer = Deserializer<'d, &'d mut R>;

            #[inline]
            fn src(&self) -> &'env str {
                self.src
            }
            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'env str> {
                std::iter::once(self.dst)
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

        match (&src, &dst, &desc) {
            (
                MaybeBorrowed::Borrowed(src),
                MaybeBorrowed::Borrowed(dst),
                MaybeBorrowed::Borrowed(desc),
            ) => {
                let described = Described {
                    src,
                    dst,
                    desc,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        ident: self.ident + 1,
                        last_abort_ident: self.last_abort_ident.reclaim(),
                        read: self.read.get_mut(),
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
                    SmolCowStr::from(dst),
                    SmolCowStr::from(desc),
                );
                let described = Described {
                    src: &src,
                    dst: &dst,
                    desc: &desc,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        ident: self.ident + 1,
                        last_abort_ident: self.last_abort_ident.reclaim(),
                        read: self.read.get_mut(),
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
        let dst = parse_bytes(dst, "class-name-b")?;

        struct Class<'env, 'd, R> {
            src: &'env str,
            dst: &'env str,
            deser: Deserializer<'d, &'d mut R>,
        }

        impl<'de, 'env, 'd, R> de::ClassAccess<'de, 'env> for Class<'env, 'd, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;
            type ContentDeserializer = Deserializer<'d, &'d mut R>;

            #[inline]
            fn src(&self) -> &'env str {
                self.src
            }
            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'env str> {
                std::iter::once(self.dst)
            }
            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        match (&src, &dst) {
            (MaybeBorrowed::Borrowed(src), MaybeBorrowed::Borrowed(dst)) => visitor
                .visit_class_borrowed(Class {
                    src,
                    dst,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        ident: self.ident + 1,
                        last_abort_ident: self.last_abort_ident.reclaim(),
                        read: self.read.get_mut(),
                    },
                }),
            _ => {
                let (src, dst) = (SmolCowStr::from(src), SmolCowStr::from(dst));
                visitor.visit_class(Class {
                    src: &src,
                    dst: &dst,
                    deser: Deserializer {
                        src: self.src,
                        dst: self.dst,
                        ident: self.ident + 1,
                        last_abort_ident: self.last_abort_ident.reclaim(),
                        read: self.read.get_mut(),
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

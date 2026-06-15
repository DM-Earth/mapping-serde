use io_util::{ColumnRead, ColumnReader, SmolCowStr};
use mapping_serde::de::{self, ClassAccess, Error as _, FieldAccess, MethodAccess};
use smol_str::{SmolStr, StrExt as _, format_smolstr};

use crate::{Error, INDENT};

/// Deserializer of a ProGuard mapping file.
#[derive(Debug)]
pub struct Deserializer<'a, R> {
    src: &'a str,
    dst: &'a str,
    read: ColumnReader<R>,
}

/// Attempts to fetch next line and returns if the deserialization process should be continued.
fn fetch_line_impl<'de, R>(
    indent: usize,
    aborted: &mut bool,
    read: &mut ColumnReader<R>,
) -> Result<bool, Error>
where
    R: ColumnRead<'de>,
{
    if *aborted {
        return Ok(true);
    }
    if read.is_fresh_line() {
        if read.this_indent() != Some(indent)
            || read.this_line().is_some_and(|l| l.starts_with(b"#"))
        {
            *aborted = true;
            return Ok(false);
        }
    } else {
        loop {
            let Some(i) = read.next_line()? else {
                *aborted = true;
                return Ok(false);
            };
            if read.this_line().is_some_and(|l| l.starts_with(b"#")) {
                continue;
            }
            if i < indent {
                *aborted = true;
                return Ok(false);
            } else if i > indent {
                continue;
            } else {
                debug_assert_eq!(indent, i);
                break;
            }
        }
    }
    Ok(true)
}

impl<'a, 'de, R> Deserializer<'a, R>
where
    R: ColumnRead<'de>,
{
    /// Creates a new ProGuard deserializer.
    pub fn new(src: &'a str, dst: &'a str, read: R) -> Self {
        Self {
            src,
            dst,
            read: ColumnReader::new(INDENT, b'\0', read),
        }
    }
}

fn process_src(name: &str) -> SmolStr {
    name.replace_smolstr(".", "/")
}

// #[inline]
// fn process_desc(name: &str) -> SmolStr {
//     process_src(name)
// }

impl<'de, R> Deserializer<'_, R>
where
    R: ColumnRead<'de>,
{
    #[inline]
    fn deserialize_impl<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Error>
    where
        V: de::Visitor<'de>,
    {
        if !fetch_line_impl(0, &mut false, &mut self.read)? {
            return Ok(None);
        }
        self.read.mark_dirty();
        let Some(line) = self.read.this_line() else {
            return Ok(None);
        };
        // somehow untrimmed
        let (src, dst) = str::from_utf8(line.as_short())?
            .strip_suffix(':')
            .ok_or_else(|| Error::custom("missing trailing colon in classline"))?
            .split_once(" -> ")
            .ok_or_else(|| Error::custom("missing separator ' -> ' in classline"))?;

        let (src, dst) = (process_src(src), SmolStr::new(dst));

        struct Class<'a, R> {
            src: &'a str,
            dst: &'a str,
            deser: ContentDeserializer<'a, R>,
        }

        impl<'a, 'de, R> ClassAccess<'de, 'a> for Class<'a, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;

            type ContentDeserializer = ContentDeserializer<'a, R>;

            #[inline]
            fn src(&self) -> &'a str {
                self.src
            }

            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'a str> {
                std::iter::once(self.dst)
            }

            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        let access = Class {
            src: &src,
            dst: &dst,
            deser: ContentDeserializer {
                src: self.src,
                dst: self.dst,
                read: &mut self.read,
                aborted: false,
            },
        };
        visitor.visit_class(access).map(Some)
    }
}

impl<'de, R> de::Deserializer<'de> for Deserializer<'_, R>
where
    R: ColumnRead<'de>,
{
    type Error = Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.src
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.dst)
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_impl(visitor)
            .map_err(|err| err.with_loc(self.read.line(), self.read.col()))
    }
}

struct ContentDeserializer<'a, R> {
    src: &'a str,
    dst: &'a str,
    read: &'a mut ColumnReader<R>,
    aborted: bool,
}

const CONTENT_INDENT: usize = 4;

fn parse_type(ty: &str) -> SmolCowStr<'_> {
    if let Some(bare) = ty.strip_suffix("[]") {
        let c = parse_type(bare);
        SmolCowStr::Owned(format_smolstr!("[{c}"))
    } else {
        match ty {
            "byte" => SmolCowStr::Borrowed("B"),
            "char" => SmolCowStr::Borrowed("C"),
            "double" => SmolCowStr::Borrowed("D"),
            "float" => SmolCowStr::Borrowed("F"),
            "int" => SmolCowStr::Borrowed("I"),
            "long" => SmolCowStr::Borrowed("J"),
            "short" => SmolCowStr::Borrowed("S"),
            "boolean" => SmolCowStr::Borrowed("Z"),
            "void" => SmolCowStr::Borrowed("V"),
            class => SmolCowStr::Owned(format_smolstr!("L{class};")),
        }
    }
}

struct Described<'a> {
    src: &'a str,
    dst: &'a str,
    desc: &'a str,
    deser: EmptyDeserializer<'a>,
}

impl<'a> FieldAccess<'_, 'a> for Described<'a> {
    type Error = Error;

    type ContentDeserializer = EmptyDeserializer<'a>;

    #[inline]
    fn src(&self) -> &'a str {
        self.src
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'a str> {
        std::iter::once(self.dst)
    }

    #[inline]
    fn desc(&self) -> Option<&'a str> {
        Some(self.desc)
    }

    #[inline]
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'a str>> {
        None::<std::iter::Empty<_>>
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        self.deser
    }
}

impl<'a> MethodAccess<'_, 'a> for Described<'a> {
    type Error = Error;

    type ContentDeserializer = EmptyDeserializer<'a>;

    #[inline]
    fn src(&self) -> &'a str {
        self.src
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'a str> {
        std::iter::once(self.dst)
    }

    #[inline]
    fn desc(&self) -> Option<&'a str> {
        Some(self.desc)
    }

    #[inline]
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'a str>> {
        None::<std::iter::Empty<_>>
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        self.deser
    }
}

impl<'de, R> de::Deserializer<'de> for ContentDeserializer<'_, R>
where
    R: ColumnRead<'de>,
{
    type Error = Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.src
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.dst)
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.aborted || !fetch_line_impl(CONTENT_INDENT, &mut self.aborted, &mut *self.read)? {
            return Ok(None);
        }
        self.read.mark_dirty();
        let Some(line) = self.read.this_line().map(|m| m.as_short()) else {
            return Ok(None);
        };
        let (line, dst) = str::from_utf8(line)?
            .split_once(" -> ")
            .ok_or_else(|| Error::custom("missing separator ' -> ' in methodline or fieldline"))?;
        let mut elements = line.split_ascii_whitespace();

        let orig_ty = elements
            .next()
            .ok_or_else(|| Error::missing_field("originaltype"))?;
        let orig_ty = parse_type(orig_ty.rsplit_once(':').map_or(orig_ty, |(_, b)| b));

        let src_and_args = elements
            .next()
            .ok_or_else(|| Error::missing_field("originalname"))?;
        let src_and_args = orig_ty.split_once(':').map_or(src_and_args, |(a, _)| a);
        let (src, args) = src_and_args.split_once('(').unzip();
        let src = src.unwrap_or(src_and_args);
        let src = process_src(src.split_once('.').map_or(src, |(_, b)| b));
        let args = args
            .map(|a| {
                a.strip_suffix(')')
                    .ok_or_else(|| Error::custom("unclosed parentheses"))
            })
            .transpose()?;

        let deser = EmptyDeserializer {
            src: self.src,
            dst: self.dst,
        };

        match args {
            // method
            Some(args) => {
                let mut desc = String::new();
                desc.push('(');
                for arg in args.split(',') {
                    if !arg.is_empty() {
                        desc.push_str(&parse_type(arg));
                    }
                }
                desc.push(')');
                desc.push_str(&orig_ty);
                let access = Described {
                    src: &src,
                    dst,
                    desc: &desc,
                    deser,
                };
                visitor.visit_method(access)
            }
            // field
            None => {
                let access = Described {
                    src: &src,
                    dst,
                    desc: &orig_ty,
                    deser,
                };
                visitor.visit_field(access)
            }
        }
        .map(Some)
    }
}

struct EmptyDeserializer<'a> {
    src: &'a str,
    dst: &'a str,
}

impl<'de> de::Deserializer<'de> for EmptyDeserializer<'_> {
    type Error = Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.src
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.dst)
    }

    #[inline]
    fn deserialize_any<V>(&mut self, _visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

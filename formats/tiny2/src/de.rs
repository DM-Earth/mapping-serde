use std::{collections::HashMap, ops::Deref};

use io_util::{ColumnRead, ColumnReader, MaybeBorrowed, SmolCowStr};
use mapping_serde::de::{self, Error as _};
use smallvec::SmallVec;
use smol_str::{SmolStr, ToSmolStr as _};

use crate::{Error, INDENT, SEPARATOR};

fn parse_version<'de, R>(reader: &mut ColumnReader<R>) -> Result<(u16, u16), Error>
where
    R: ColumnRead<'de>,
{
    let _ = reader
        .read_col()?
        .ok_or_else(|| Error::missing_field("magic in tiny2 header"))?;
    let major = reader
        .read_col()?
        .ok_or_else(|| Error::missing_field("major-version"))?;
    let major = str::from_utf8(&major).map_err(Error::from).and_then(|s| {
        s.parse::<u16>()
            .map_err(|e| Error::invalid_type(e, "unsigned integer"))
    })?;
    let minor = reader
        .read_col()?
        .ok_or_else(|| Error::missing_field("minor-version"))?;
    let minor = str::from_utf8(&minor).map_err(Error::from).and_then(|s| {
        s.parse::<u16>()
            .map_err(|e| Error::invalid_type(e, "unsigned integer"))
    })?;
    Ok((major, minor))
}

/// Tiny V2 mapping file deserializer.
#[derive(Debug)]
pub struct Deserializer<R> {
    src: SmolStr,
    dst: SmallVec<[SmolStr; 2]>,
    aborted: bool,
    read: ColumnReader<R>,

    props: HashMap<SmolStr, Option<SmolStr>>,
    escaped_names: bool,
    missing_lvt_indices: bool,
}

#[inline]
fn parse_bytes<'a, 'b>(
    b: Option<MaybeBorrowed<'a, 'b, [u8]>>,
    section: &str,
) -> Result<MaybeBorrowed<'a, 'b, str>, Error> {
    b.ok_or_else(|| Error::missing_field(section))
        .and_then(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
}

fn parse_names<'de, R>(
    read: &mut ColumnReader<R>,
    src_field: &str,
) -> Result<(SmolCowStr<'de>, SmallVec<[SmolCowStr<'de>; 2]>), Error>
where
    R: ColumnRead<'de>,
{
    let src = SmolCowStr::from(parse_bytes(read.read_col()?, src_field)?);
    let mut dst: SmallVec<[_; 2]> = SmallVec::new();
    while let Some(b) = read.read_col()? {
        dst.push(SmolCowStr::from(b.try_map(str::from_utf8)?));
    }
    Ok((src, dst))
}

impl<'de, R> Deserializer<R>
where
    R: ColumnRead<'de>,
{
    /// Creates a new tiny2 mapping deserializer.
    ///
    /// # Errors
    ///
    /// - Fails if the file version is not 2.x, or doesn't exist.
    /// - Fails if missing source or destination namespace.
    pub fn new(read: R) -> Result<Self, Error> {
        let mut reader = ColumnReader::new(INDENT, SEPARATOR, read);
        let (major, minor) = parse_version(&mut reader)?;
        if major != 2 {
            return Err(Error::custom(format_args!(
                "unexpected tiny v2 version: {major}.{minor}, expected 2.x"
            )));
        }

        let (src, dst) = parse_names(&mut reader, "namespace-a")?;
        if dst.is_empty() {
            return Err(Error::missing_field("namespace-b"));
        }

        let mut props = HashMap::new();
        while let Some(1) = reader.next_line()? {
            let key = reader
                .read_col()
                .map_err(Error::from)
                .and_then(|mb| {
                    mb.map(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
                        .transpose()
                })?
                .map(SmolStr::new);
            let val = reader.read_col().map_err(Error::from).and_then(|mb| {
                mb.map(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
                    .transpose()
            })?;
            if let Some(key) = key {
                props.insert(key, val.map(SmolStr::new));
            }
        }

        Ok(Self {
            src: src.to_smolstr(),
            dst: dst.into_iter().map(SmolStr::new).collect(),
            aborted: false,
            read: reader,

            escaped_names: props.contains_key("escaped-names"),
            missing_lvt_indices: props.contains_key("missing-lvt-indices"),

            props,
        })
    }
}

impl<R> Deserializer<R> {
    /// Returns an iterator over properties of the given file parsed by this deserializer.
    pub fn properties(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.props.iter().map(|(a, b)| (&**a, b.as_deref()))
    }

    fn local_cx(&mut self) -> LocalCx<'_, R> {
        LocalCx {
            src: &self.src,
            dst: &self.dst,
            escaped_names: self.escaped_names,
            missing_lvt_indices: self.missing_lvt_indices,
            read: &mut self.read,
        }
    }
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
        if read.this_indent() != Some(indent) {
            *aborted = true;
            return Ok(false);
        }
    } else {
        loop {
            let i = read.next_line()?;
            if i.is_none_or(|i| i < indent) {
                *aborted = true;
                return Ok(false);
            } else if i.is_some_and(|i| i > indent) {
                continue;
            } else {
                debug_assert_eq!(Some(indent), i);
                break;
            }
        }
    }
    Ok(true)
}

fn try_borrow_dst<'de>(dst: &SmallVec<[SmolCowStr<'de>; 2]>) -> Option<SmallVec<[&'de str; 2]>> {
    dst.iter().map(SmolCowStr::as_borrowed).try_fold(
        SmallVec::with_capacity(dst.capacity()),
        |mut sv, s| {
            sv.push(s?);
            Some(sv)
        },
    )
}

struct LocalCx<'a, R> {
    src: &'a str,
    dst: &'a SmallVec<[SmolStr; 2]>,
    escaped_names: bool,
    missing_lvt_indices: bool,
    read: &'a mut ColumnReader<R>,
}

impl<R> LocalCx<'_, R> {
    #[inline]
    fn reclaim(&mut self) -> LocalCx<'_, R> {
        // literally copies the struct
        LocalCx {
            src: self.src,
            dst: self.dst,
            escaped_names: self.escaped_names,
            missing_lvt_indices: self.missing_lvt_indices,
            read: self.read,
        }
    }
}

fn deserialize_class_impl<'de, R, V>(
    visitor: V,
    indent: usize,
    cx: LocalCx<'_, R>,
) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
    R: ColumnRead<'de>,
{
    // class-name-b is optional
    let (src, dst) = parse_names(cx.read, "class-name-a")?;

    struct ContentDeserializer<'a, R> {
        indent: usize,
        aborted: bool,
        cx: LocalCx<'a, R>,
    }

    impl<'de, R> ContentDeserializer<'_, R>
    where
        R: ColumnRead<'de>,
    {
        fn deserialize_impl<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Error>
        where
            V: de::Visitor<'de>,
        {
            if !fetch_line_impl(self.indent, &mut self.aborted, self.cx.read)? {
                return Ok(None);
            }
            let ty = self.cx.read.read_col()?;
            let ty = ty.as_deref().unwrap_or_default();

            match ty {
                b"c" => deserialize_comment_impl(visitor, self.cx.reclaim()),
                // TODO: field, method
                _ => Err(Error::invalid_type(
                    String::from_utf8_lossy(ty),
                    "c(comment), f, m",
                )),
            }
            .map(Some)
        }
    }

    impl<'de, R> de::Deserializer<'de> for ContentDeserializer<'_, R>
    where
        R: ColumnRead<'de>,
    {
        type Error = Error;

        #[inline]
        fn src_namespace(&self) -> &str {
            self.cx.src
        }
        #[inline]
        fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
            self.cx.dst.iter().map(Deref::deref)
        }
        #[inline]
        fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            self.deserialize_impl(visitor)
                .map_err(|err| err.with_loc(self.cx.read.line(), self.cx.read.col()))
        }
    }

    struct Class<'env, 'd, R> {
        src: &'env str,
        dst: SmallVec<[&'env str; 2]>,
        deser: ContentDeserializer<'d, R>,
    }

    impl<'de, 'env, 'd, R> de::ClassAccess<'de, 'env> for Class<'env, 'd, R>
    where
        R: ColumnRead<'de>,
    {
        type Error = Error;
        type ContentDeserializer = ContentDeserializer<'d, R>;

        #[inline]
        fn src(&self) -> &'env str {
            self.src
        }
        #[inline]
        fn dst(&self) -> impl Iterator<Item = &'env str> {
            self.dst.iter().copied()
        }
        #[inline]
        fn content(self) -> Self::ContentDeserializer {
            self.deser
        }
    }

    if let (SmolCowStr::Borrowed(src), Some(dst)) = (&src, try_borrow_dst(&dst)) {
        visitor.visit_class_borrowed(Class {
            src,
            dst,
            deser: ContentDeserializer {
                indent: indent + 1,
                aborted: false,
                cx,
            },
        })
    } else {
        visitor.visit_class(Class {
            src: &src,
            dst: dst.iter().map(Deref::deref).collect(),
            deser: ContentDeserializer {
                indent: indent + 1,
                aborted: false,
                cx,
            },
        })
    }
}

fn deserialize_comment_impl<'de, R, V>(visitor: V, cx: LocalCx<'_, R>) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
    R: ColumnRead<'de>,
{
    let comment = cx
        .read
        .read_col()?
        .unwrap_or(MaybeBorrowed::Borrowed(b""))
        .try_map(str::from_utf8)?;
    match comment {
        MaybeBorrowed::Short(c) => visitor.visit_comment(c),
        MaybeBorrowed::Borrowed(c) => visitor.visit_comment_borrowed(c),
    }
}

impl<'de, R> Deserializer<R>
where
    R: ColumnRead<'de>,
{
    fn deserialize_impl<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Error>
    where
        V: de::Visitor<'de>,
    {
        if !fetch_line_impl(0, &mut self.aborted, &mut self.read)? {
            return Ok(None);
        }
        let ty = self.read.read_col()?;
        let ty = ty.as_deref().unwrap_or_default();

        match ty {
            b"c" => deserialize_class_impl(visitor, 0, self.local_cx()),
            _ => Err(Error::invalid_type(String::from_utf8_lossy(ty), "c(class)")),
        }
        .map(Some)
    }
}

impl<'de, R> de::Deserializer<'de> for Deserializer<R>
where
    R: ColumnRead<'de>,
{
    type Error = Error;

    #[inline]
    fn src_namespace(&self) -> &str {
        &self.src
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.dst.iter().map(Deref::deref)
    }

    #[inline]
    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_impl(visitor)
            .map_err(|err| err.with_loc(self.read.line(), self.read.col()))
    }
}

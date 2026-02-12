use std::{borrow::Cow, collections::BTreeMap, io::BufRead, ops::Deref};

use fast_unescape::try_unescape;
use io_util::{
    ColumnRead, ColumnReadAdapter, ColumnReader, IoReader, MaybeBorrowed, SliceReader, SmolCowStr,
};
use mapping_serde::de::{
    self, Error as _, FieldAccess, MethodAccess, MethodArgAccess, MethodVarAccess,
};
use smallvec::SmallVec;
use smol_str::{SmolStr, ToSmolStr as _};

use crate::{Error, INDENT, PROPERTY_ESCAPED_NAMES, SEPARATOR};

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

    props: BTreeMap<SmolStr, Option<SmolStr>>,
    escaped_names: bool,
    // missing_lvt_indices: bool,
}

#[inline]
fn parse_bytes<'a, 'b>(
    b: Option<MaybeBorrowed<'a, 'b, [u8]>>,
    section: &str,
) -> Result<MaybeBorrowed<'a, 'b, str>, Error> {
    b.ok_or_else(|| Error::missing_field(section))
        .and_then(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
}

fn parse_dst<'de, R>(
    read: &mut ColumnReader<R>,
    escaped_names: bool,
) -> Result<SmallVec<[SmolCowStr<'de>; 2]>, Error>
where
    R: ColumnRead<'de>,
{
    let mut dst: SmallVec<[_; 2]> = SmallVec::new();
    while let Some(b) = read.read_col()? {
        dst.push(
            b.try_map(str::from_utf8)
                .map_err(Into::into)
                .and_then(|b| make_smol_cow_str(b, escaped_names))?,
        );
    }
    Ok(dst)
}

fn parse_names<'de, R>(
    read: &mut ColumnReader<R>,
    src_field: &str,
    escaped_names: bool, // defaults to false
) -> Result<(SmolCowStr<'de>, SmallVec<[SmolCowStr<'de>; 2]>), Error>
where
    R: ColumnRead<'de>,
{
    let src = parse_bytes(read.read_col()?, src_field)
        .and_then(|b| make_smol_cow_str(b, escaped_names))?;
    let dst = parse_dst(read, escaped_names)?;
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
        reader.next_line()?;
        let (major, minor) = parse_version(&mut reader)?;
        if major != 2 {
            return Err(Error::custom(format_args!(
                "unexpected tiny v2 version: {major}.{minor}, expected 2.x"
            )));
        }

        let (src, dst) = parse_names(&mut reader, "namespace-a", false)?;
        if dst.is_empty() {
            return Err(Error::missing_field("namespace-b"));
        }

        let mut props = BTreeMap::new();
        while let Some(1) = reader.next_line()? {
            let key = reader
                .read_col()
                .map_err(Error::from)
                .and_then(|mb| {
                    mb.map(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
                        .transpose()
                })?
                .map(SmolStr::new);
            let val = reader
                .read_col()
                .map_err(Error::from)
                .and_then(|mb| {
                    mb.map(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
                        .transpose()
                })?
                .map(|s| try_unescape(&s).map(SmolStr::new))
                .transpose()?;
            if let Some(key) = key {
                props.insert(key, val);
            }
        }

        Ok(Self {
            src: src.to_smolstr(),
            dst: dst.into_iter().map(SmolStr::new).collect(),
            aborted: false,
            read: reader,

            escaped_names: props.contains_key(PROPERTY_ESCAPED_NAMES),
            // missing_lvt_indices: props.contains_key("missing-lvt-indices"),
            props,
        })
    }
}

impl<'slice> Deserializer<ColumnReadAdapter<Box<SliceReader<'slice>>>> {
    /// Creates a new deserializer from the given slice.
    ///
    /// Note that this involves heap allocation. To avoid it, pin a reader in the stack and
    /// create a deserializer with [`Self::new`].
    ///
    /// # Errors
    ///
    /// See [`Self::new`].
    pub fn from_slice(slice: &'slice [u8]) -> Result<Self, Error> {
        Self::new(ColumnReadAdapter::new(Box::new(SliceReader::new(slice))))
    }
}

impl<R> Deserializer<ColumnReadAdapter<Box<IoReader<R>>>>
where
    R: Unpin + BufRead,
{
    /// Creates a new deserializer from the given I/O reader.
    /// The reader should implement [`std::io::BufRead`].
    ///
    /// Note that this involves heap allocation. To avoid it, pin a reader in the stack and
    /// create a deserializer with [`Self::new`].
    ///
    /// # Errors
    ///
    /// See [`Self::new`].
    pub fn from_reader(reader: R) -> Result<Self, Error> {
        Self::new(ColumnReadAdapter::new(Box::new(IoReader::new(reader))))
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
            // missing_lvt_indices: self.missing_lvt_indices,
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

fn try_borrow_many<'de>(many: &[SmolCowStr<'de>]) -> Option<SmallVec<[&'de str; 2]>> {
    many.iter().map(SmolCowStr::as_borrowed).try_fold(
        SmallVec::with_capacity(many.len()),
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
    // missing_lvt_indices: bool,
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
            // missing_lvt_indices: self.missing_lvt_indices,
            read: self.read,
        }
    }
}

trait ContentSpec<'de, R> {
    type Context;

    #[allow(clippy::type_complexity)]
    fn process(&mut self, ty: &[u8]) -> Result<Self::Context, Error>;

    fn visit<V>(
        &mut self,
        local: Self::Context,
        visitor: V,
        cx: LocalCx<'_, R>,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>;
}

struct CommentOnlySpec;

impl<'de, R> ContentSpec<'de, R> for CommentOnlySpec
where
    R: ColumnRead<'de>,
{
    type Context = ();

    fn process(&mut self, ty: &[u8]) -> Result<Self::Context, Error> {
        if ty == b"c" {
            Ok(())
        } else {
            Err(Error::invalid_type(
                String::from_utf8_lossy(ty),
                "c(comment)",
            ))
        }
    }

    fn visit<V>(
        &mut self,
        _local: Self::Context,
        visitor: V,
        cx: LocalCx<'_, R>,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        deserialize_comment_impl(visitor, cx)
    }
}

struct ContentDeserializer<'a, R, S> {
    indent: usize,
    aborted: bool,
    cx: LocalCx<'a, R>,
    spec: S,
}

impl<'de, R, S> ContentDeserializer<'_, R, S>
where
    R: ColumnRead<'de>,
    S: ContentSpec<'de, R>,
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

        self.spec
            .process(ty)
            .and_then(|l| self.spec.visit(l, visitor, self.cx.reclaim()))
            .map(Some)
    }
}

impl<'de, R, S> de::Deserializer<'de> for ContentDeserializer<'_, R, S>
where
    R: ColumnRead<'de>,
    S: ContentSpec<'de, R>,
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

fn deserialize_class_impl<'de, R, V>(
    visitor: V,
    indent: usize,
    cx: LocalCx<'_, R>,
) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
    R: ColumnRead<'de>,
{
    let (src, dst) = parse_names(cx.read, "class-name-a", cx.escaped_names)?;

    struct ClassSpec {
        indent: usize,
    }

    enum SibKind {
        Comment,
        Field,
        Method,
    }

    impl<'de, R> ContentSpec<'de, R> for ClassSpec
    where
        R: ColumnRead<'de>,
    {
        type Context = SibKind;

        fn process(&mut self, ty: &[u8]) -> Result<Self::Context, Error> {
            match ty {
                b"c" => Ok(SibKind::Comment),
                b"f" => Ok(SibKind::Field),
                b"m" => Ok(SibKind::Method),
                _ => Err(Error::invalid_type(
                    String::from_utf8_lossy(ty),
                    "c(comment), f, m",
                )),
            }
        }

        fn visit<V>(
            &mut self,
            local: Self::Context,
            visitor: V,
            cx: LocalCx<'_, R>,
        ) -> Result<V::Value, Error>
        where
            V: de::Visitor<'de>,
        {
            match local {
                SibKind::Comment => deserialize_comment_impl(visitor, cx),
                SibKind::Field => deserialize_field_impl(visitor, self.indent, cx),
                SibKind::Method => deserialize_method_impl(visitor, self.indent, cx),
            }
        }
    }

    struct Class<'env, 'd, R> {
        src: &'env str,
        dst: SmallVec<[&'env str; 2]>,
        deser: ContentDeserializer<'d, R, ClassSpec>,
    }

    impl<'de, 'env, 'd, R> de::ClassAccess<'de, 'env> for Class<'env, 'd, R>
    where
        R: ColumnRead<'de>,
    {
        type Error = Error;
        type ContentDeserializer = ContentDeserializer<'d, R, ClassSpec>;

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

    let deser = ContentDeserializer {
        indent: indent + 1,
        aborted: false,
        cx,
        spec: ClassSpec { indent: indent + 1 },
    };

    if let SmolCowStr::Borrowed(src) = &src
        && let Some(dst) = try_borrow_many(&dst)
    {
        visitor.visit_class_borrowed(Class { src, dst, deser })
    } else {
        visitor.visit_class(Class {
            src: &src,
            dst: dst.iter().map(Deref::deref).collect(),
            deser,
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
        .this_line()
        .unwrap_or(MaybeBorrowed::Borrowed(b""))
        .try_map(str::from_utf8)?;

    match comment {
        MaybeBorrowed::Short(c) => visitor.visit_comment(&try_unescape(c)?),
        MaybeBorrowed::Borrowed(c) => match try_unescape(c)? {
            Cow::Borrowed(c) => visitor.visit_comment_borrowed(c),
            Cow::Owned(c) => visitor.visit_comment(&c),
        },
    }
}

fn deserialize_field_impl<'de, R, V>(
    visitor: V,
    indent: usize,
    cx: LocalCx<'_, R>,
) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
    R: ColumnRead<'de>,
{
    let desc = parse_bytes(cx.read.read_col()?, "field-desc-a")
        .and_then(|b| make_smol_cow_str(b, cx.escaped_names))?;
    let (src, dst) = parse_names(&mut *cx.read, "field-name-a", cx.escaped_names)?;

    type FieldSpec = CommentOnlySpec;

    struct Field<'env, 'd, R> {
        desc: &'env str,
        src: &'env str,
        dst: SmallVec<[&'env str; 2]>,
        deser: ContentDeserializer<'d, R, FieldSpec>,
    }

    impl<'de, 'env, 'd, R> FieldAccess<'de, 'env> for Field<'env, 'd, R>
    where
        R: ColumnRead<'de>,
    {
        type Error = Error;
        type ContentDeserializer = ContentDeserializer<'d, R, FieldSpec>;

        #[inline]
        fn src(&self) -> &'env str {
            self.src
        }
        #[inline]
        fn dst(&self) -> impl Iterator<Item = &'env str> {
            self.dst.iter().copied()
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

    let deser = ContentDeserializer {
        indent: indent + 1,
        aborted: false,
        cx,
        spec: CommentOnlySpec,
    };

    if let (SmolCowStr::Borrowed(desc), SmolCowStr::Borrowed(src)) = (&desc, &src)
        && let Some(dst) = try_borrow_many(&dst)
    {
        visitor.visit_field_borrowed(Field {
            desc,
            src,
            dst,
            deser,
        })
    } else {
        visitor.visit_field(Field {
            desc: &desc,
            src: &src,
            dst: dst.iter().map(Deref::deref).collect(),
            deser,
        })
    }
}

fn deserialize_method_impl<'de, R, V>(
    visitor: V,
    indent: usize,
    cx: LocalCx<'_, R>,
) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
    R: ColumnRead<'de>,
{
    let desc = parse_bytes(cx.read.read_col()?, "field-desc-a")
        .and_then(|b| make_smol_cow_str(b, cx.escaped_names))?;
    let (src, dst) = parse_names(&mut *cx.read, "field-name-a", cx.escaped_names)?;

    enum SibKind {
        Comment,
        Param,
        Var,
    }

    struct MethodSpec {
        indent: usize,
    }

    impl<'de, R> ContentSpec<'de, R> for MethodSpec
    where
        R: ColumnRead<'de>,
    {
        type Context = SibKind;

        fn process(&mut self, ty: &[u8]) -> Result<Self::Context, Error> {
            match ty {
                b"c" => Ok(SibKind::Comment),
                b"p" => Ok(SibKind::Param),
                b"v" => Ok(SibKind::Var),
                _ => Err(Error::invalid_type(
                    String::from_utf8_lossy(ty),
                    "c(comment), p, v",
                )),
            }
        }

        fn visit<V>(
            &mut self,
            local: Self::Context,
            visitor: V,
            cx: LocalCx<'_, R>,
        ) -> Result<V::Value, Error>
        where
            V: de::Visitor<'de>,
        {
            match local {
                SibKind::Comment => deserialize_comment_impl(visitor, cx),
                SibKind::Param => deserialize_method_param_impl(visitor, self.indent, cx),
                SibKind::Var => deserialize_method_var_impl(visitor, self.indent, cx),
            }
        }
    }

    struct Method<'env, 'd, R> {
        desc: &'env str,
        src: &'env str,
        dst: SmallVec<[&'env str; 2]>,
        deser: ContentDeserializer<'d, R, MethodSpec>,
    }

    impl<'de, 'env, 'd, R> MethodAccess<'de, 'env> for Method<'env, 'd, R>
    where
        R: ColumnRead<'de>,
    {
        type Error = Error;
        type ContentDeserializer = ContentDeserializer<'d, R, MethodSpec>;

        #[inline]
        fn src(&self) -> &'env str {
            self.src
        }
        #[inline]
        fn dst(&self) -> impl Iterator<Item = &'env str> {
            self.dst.iter().copied()
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

    let deser = ContentDeserializer {
        indent: indent + 1,
        aborted: false,
        cx,
        spec: MethodSpec { indent: indent + 1 },
    };

    if let (SmolCowStr::Borrowed(desc), SmolCowStr::Borrowed(src)) = (&desc, &src)
        && let Some(dst) = try_borrow_many(&dst)
    {
        visitor.visit_method_borrowed(Method {
            desc,
            src,
            dst,
            deser,
        })
    } else {
        visitor.visit_method(Method {
            desc: &desc,
            src: &src,
            dst: dst.iter().map(Deref::deref).collect(),
            deser,
        })
    }
}

fn deserialize_method_param_impl<'de, R, V>(
    visitor: V,
    indent: usize,
    cx: LocalCx<'_, R>,
) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
    R: ColumnRead<'de>,
{
    let lv_index = parse_bytes(cx.read.read_col()?, "lv-index").and_then(|s| {
        s.parse::<usize>()
            .map_err(|err| Error::invalid_type(format_args!("error: {err}"), "unsigned integer"))
    })?;

    // below are all optional
    let src = cx
        .read
        .read_col()?
        .map(|b| {
            b.try_map(str::from_utf8)
                .map_err(Into::into)
                .and_then(|b| make_smol_cow_str(b, cx.escaped_names))
        })
        .transpose()?;
    let dst: Option<SmallVec<[_; 2]>> = if src.is_some() {
        Some(parse_dst(&mut *cx.read, cx.escaped_names)?)
    } else {
        None
    };

    type MethodParamSpec = CommentOnlySpec;

    struct MethodParam<'env, 'd, R> {
        lv_index: usize,
        src: Option<&'env str>,
        dst: Option<SmallVec<[&'env str; 2]>>,
        deser: ContentDeserializer<'d, R, MethodParamSpec>,
    }

    impl<'de, 'env, 'd, R> MethodArgAccess<'de, 'env> for MethodParam<'env, 'd, R>
    where
        R: ColumnRead<'de>,
    {
        type Error = Error;
        type ContentDeserializer = ContentDeserializer<'d, R, MethodParamSpec>;

        #[inline]
        fn src(&self) -> Option<&'env str> {
            self.src
        }
        #[inline]
        fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
            self.dst.as_deref().map(|d| d.iter().copied())
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

    let deser = ContentDeserializer {
        indent: indent + 1,
        aborted: false,
        cx,
        spec: CommentOnlySpec,
    };

    if let (Some(SmolCowStr::Borrowed(src)), Some(dst)) = (&src, &dst)
        && let dst @ Some(_) = try_borrow_many(dst)
    {
        visitor.visit_method_arg_borrowed(MethodParam {
            lv_index,
            src: Some(src),
            dst,
            deser,
        })
    } else {
        visitor.visit_method_arg(MethodParam {
            lv_index,
            src: src.as_deref(),
            dst: dst
                .as_ref()
                .map(|dst| dst.iter().map(Deref::deref).collect()),
            deser,
        })
    }
}

fn deserialize_method_var_impl<'de, R, V>(
    visitor: V,
    indent: usize,
    cx: LocalCx<'_, R>,
) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
    R: ColumnRead<'de>,
{
    let lv_index = parse_bytes(cx.read.read_col()?, "lv-index").and_then(|s| {
        s.parse::<usize>()
            .map_err(|err| Error::invalid_type(format_args!("error: {err}"), "unsigned integer"))
    })?;
    let lv_start_offset = parse_bytes(cx.read.read_col()?, "lv-start-offset").and_then(|s| {
        s.parse::<usize>()
            .map_err(|err| Error::invalid_type(format_args!("error: {err}"), "unsigned integer"))
    })?;
    let lvt_index = {
        let b = parse_bytes(cx.read.read_col()?, "lv-start-offset")?;
        if &b == "-1" {
            None
        } else {
            Some(b.parse::<usize>().map_err(|err| {
                Error::invalid_type(format_args!("error: {err}"), "unsigned integer")
            })?)
        }
    };

    let src = cx
        .read
        .read_col()?
        .map(|b| {
            b.try_map(str::from_utf8)
                .map_err(Into::into)
                .and_then(|b| make_smol_cow_str(b, cx.escaped_names))
        })
        .transpose()?;
    let dst: Option<SmallVec<[_; 2]>> = if src.is_some() {
        Some(parse_dst(&mut *cx.read, cx.escaped_names)?)
    } else {
        None
    };

    type MethodVarSpec = CommentOnlySpec;

    struct MethodVar<'env, 'd, R> {
        lv_index: usize,
        lv_start_offset: usize,
        lvt_index: Option<usize>,
        src: Option<&'env str>,
        dst: Option<SmallVec<[&'env str; 2]>>,
        deser: ContentDeserializer<'d, R, MethodVarSpec>,
    }

    impl<'de, 'env, 'd, R> MethodVarAccess<'de, 'env> for MethodVar<'env, 'd, R>
    where
        R: ColumnRead<'de>,
    {
        type Error = Error;
        type ContentDeserializer = ContentDeserializer<'d, R, MethodVarSpec>;

        #[inline]
        fn src(&self) -> Option<&'env str> {
            self.src
        }
        #[inline]
        fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
            self.dst.as_deref().map(|dst| dst.iter().copied())
        }
        #[inline]
        fn lv_index(&self) -> Option<usize> {
            Some(self.lv_index)
        }
        #[inline]
        fn lvt_row_index(&self) -> Option<usize> {
            self.lvt_index
        }
        #[inline]
        fn op_idx(&self) -> Option<(usize, Option<usize>)> {
            Some((self.lv_start_offset, None))
        }
        #[inline]
        fn content(self) -> Self::ContentDeserializer {
            self.deser
        }
    }

    let deser = ContentDeserializer {
        indent: indent + 1,
        aborted: false,
        cx,
        spec: CommentOnlySpec,
    };

    if let (Some(SmolCowStr::Borrowed(src)), Some(dst)) = (&src, &dst)
        && let dst @ Some(_) = try_borrow_many(dst)
    {
        visitor.visit_method_var_borrowed(MethodVar {
            lv_index,
            lv_start_offset,
            lvt_index,
            src: Some(src),
            dst,
            deser,
        })
    } else {
        visitor.visit_method_var(MethodVar {
            lv_index,
            lv_start_offset,
            lvt_index,
            src: src.as_deref(),
            dst: dst
                .as_ref()
                .map(|dst| dst.iter().map(Deref::deref).collect()),
            deser,
        })
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

// utilities

fn make_smol_cow_str_unescape<'de>(
    src: MaybeBorrowed<'_, 'de, str>,
) -> Result<SmolCowStr<'de>, Error> {
    match src {
        MaybeBorrowed::Short(src) => Ok(SmolCowStr::Owned(try_unescape(src)?.into())),
        MaybeBorrowed::Borrowed(src) => match try_unescape(src)? {
            Cow::Borrowed(b) => Ok(SmolCowStr::Borrowed(b)),
            Cow::Owned(o) => Ok(SmolCowStr::Owned(o.into())),
        },
    }
}

#[inline]
fn make_smol_cow_str<'de>(
    src: MaybeBorrowed<'_, 'de, str>,
    escaped_names: bool,
) -> Result<SmolCowStr<'de>, Error> {
    if escaped_names {
        make_smol_cow_str_unescape(src)
    } else {
        Ok(SmolCowStr::from(src))
    }
}

//! Stream tiny1 file content.

use std::{collections::BTreeMap, ops::Deref};

use io_util::{ColumnRead, ColumnReader, MaybeBorrowed};
use mapping_serde::de::Error as _;
use smol_str::SmolStr;

use crate::{Error, SEPARATOR};

/// A low-level content visitor.
pub trait Visitor {
    /// Output type of this visitor.
    type Value;

    /// Visits a class entry.
    fn visit_class_entry<'a, I>(
        self,
        name_a: &'a str,
        name_b: Option<&'a str>,
        extra_ns_names: I,
    ) -> Self::Value
    where
        I: IntoIterator<Item = &'a str>;

    /// Visits a field entry.
    fn visit_field_entry<'a, I>(
        self,
        parent_class_name_a: &'a str,
        desc_a: &'a str,
        name_a: &'a str,
        name_b: Option<&'a str>,
        extra_ns_names: I,
    ) -> Self::Value
    where
        I: IntoIterator<Item = &'a str>;

    /// Visits a method entry.
    fn visit_method_entry<'a, I>(
        self,
        parent_class_name_a: &'a str,
        desc_a: &'a str,
        name_a: &'a str,
        name_b: Option<&'a str>,
        extra_ns_names: I,
    ) -> Self::Value
    where
        I: IntoIterator<Item = &'a str>;
}

/// Deserailizer of flattened entries from a Tiny1 mapping file.
#[derive(Debug)]
pub struct StreamDeserializer<R> {
    namespace_a: SmolStr,
    namespace_b: SmolStr,
    extra_namespaces: Box<[SmolStr]>,
    props: BTreeMap<SmolStr, Option<SmolStr>>,
    read: ColumnReader<R>,
}

impl<R> StreamDeserializer<R> {
    /// Returns the source namespace of this mapping.
    #[inline]
    pub fn src(&self) -> &str {
        &self.namespace_a
    }

    /// Returns the destination namespace of this mapping.
    pub fn dst(&self) -> impl Iterator<Item = &str> {
        std::iter::once(&*self.namespace_b).chain(self.extra_namespaces.iter().map(Deref::deref))
    }

    /// Returns an iterator over properties of this mapping.
    pub fn properties(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.props.iter().map(|(a, b)| (&**a, b.as_deref()))
    }
}

#[inline]
fn parse_bytes<'a, 'b>(
    b: Option<MaybeBorrowed<'a, 'b, [u8]>>,
    section: &str,
) -> Result<MaybeBorrowed<'a, 'b, str>, Error> {
    b.ok_or_else(|| Error::missing_field(section))
        .and_then(|mb| mb.try_map(str::from_utf8).map_err(Into::into))
}

impl<'de, R> StreamDeserializer<R>
where
    R: ColumnRead<'de>,
{
    /// Creates a new stream deserailizer.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not valid utf8-encoded, or missing following fields:
    ///
    /// - v1
    /// - `namespace-a`
    /// - `namespace-b`
    pub fn new(read: R) -> Result<Self, Error> {
        let mut read = ColumnReader::new(b'\0', SEPARATOR, read);
        read.next_line()?;

        if read.read_col()?.as_deref() != Some(b"v1") {
            return Err(Error::custom("wrong tiny version, expected v1"));
        }

        let namespace_a = parse_bytes(read.read_col()?, "namespace-a")?.into();
        let namespace_b = parse_bytes(read.read_col()?, "namespace-b")?.into();
        let mut extra_namespaces = Vec::new();
        while let Some(b) = read.read_col()? {
            extra_namespaces.push(b.try_map(str::from_utf8)?.into());
        }

        let mut props = BTreeMap::new();
        while read.next_line()?.is_some()
            && let Some(line) = read
                .this_line()
                .and_then(|line| line.as_short().strip_prefix(b"# "))
        {
            let line = str::from_utf8(line)?.trim_ascii_end();
            if let Some((key, val)) = line.split_once(' ') {
                props.insert(key.into(), Some(val.into()));
            } else {
                props.insert(line.into(), None);
            }
        }

        Ok(Self {
            namespace_a,
            namespace_b,
            extra_namespaces: extra_namespaces.into_boxed_slice(),
            props,
            read,
        })
    }

    fn deserialize_class<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor,
    {
        let mut iter = self.read.iter_cols();
        let name_a = parse_bytes(iter.next().transpose()?, "class-name-a")?;
        let mut dst_iter =
            iter.filter_map(|r| r.ok().and_then(|b| str::from_utf8(b.as_short()).ok()));
        Ok(visitor.visit_class_entry(&name_a, dst_iter.next(), dst_iter))
    }

    fn deserialize_field<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor,
    {
        let mut iter = self.read.iter_cols();
        let parent = parse_bytes(iter.next().transpose()?, "parent-class-name-a")?;
        let desc_a = parse_bytes(iter.next().transpose()?, "field-desc-a")?;
        let name_a = parse_bytes(iter.next().transpose()?, "field-name-a")?;
        let mut dst_iter =
            iter.filter_map(|r| r.ok().and_then(|b| str::from_utf8(b.as_short()).ok()));
        Ok(visitor.visit_field_entry(&parent, &desc_a, &name_a, dst_iter.next(), dst_iter))
    }

    fn deserialize_method<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor,
    {
        let mut iter = self.read.iter_cols();
        let parent = parse_bytes(iter.next().transpose()?, "parent-class-name-a")?;
        let desc_a = parse_bytes(iter.next().transpose()?, "method-desc-a")?;
        let name_a = parse_bytes(iter.next().transpose()?, "method-name-a")?;
        let mut dst_iter =
            iter.filter_map(|r| r.ok().and_then(|b| str::from_utf8(b.as_short()).ok()));
        Ok(visitor.visit_method_entry(&parent, &desc_a, &name_a, dst_iter.next(), dst_iter))
    }

    #[inline]
    fn deserialize_impl<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Error>
    where
        V: Visitor,
    {
        if !self.read.is_fresh_line() {
            self.read.next_line()?;
        }
        let ty = self
            .read
            .read_col()?
            .ok_or_else(|| Error::missing_field("entry-type"))?;

        match &*ty {
            b"CLASS" => self.deserialize_class(visitor),
            b"FIELD" => self.deserialize_field(visitor),
            b"METHOD" => self.deserialize_method(visitor),
            other => Err(Error::custom(format_args!(
                "invalid entry type: {}",
                String::from_utf8_lossy(other)
            ))),
        }
        .map(Some)
    }

    /// Deserializes next entry of this mapping file with given visitor.
    #[allow(clippy::missing_errors_doc)] // too many
    pub fn deserialize_next<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Error>
    where
        V: Visitor,
    {
        self.deserialize_impl(visitor).map_err(|err| Error {
            kind: err.kind,
            line: self.read.line(),
            col: self.read.col(),
        })
    }
}

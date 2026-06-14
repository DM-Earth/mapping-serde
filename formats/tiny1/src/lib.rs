//! mapping-serde support for Tiny v1 mapping format.
//!
//! > Tiny v1 consists of a list of flat (non-hierarchical) mapping entries.
//! > Every line in the content section corresponds to a new entry.
//! > Supported elements are classes, fields and methods.
//!
//! - from Fabric wiki [Tiny v1](https://wiki.fabricmc.net/documentation:tiny).
//!
//! # Deserialization
//!
//! Due to the flatten nature of Tiny1 format it's more difficult to deserialize it in a tree fashion, which is
//! required by `mapping-serde`, and therefore there are three ways to deserialize it in this crate:
//!
//! * Index the whole file with [`Index::from_stream`] then create a deserializer with [`Index::as_deserializer`].
//!   This is the most conservative way to achieve it and is the slowest. The classes are still flattened.
//! * Treat the file as tree-style, like Tiny2 but not indented through [`PseudoTreeDeserializer`].
//!   This is useful for generated mappings like `intermediary`.
//! * Visits each entry directly with [`StreamDeserializer`], only if you need low-level access to the file.

use std::{
    fmt::Display,
    io::{BufRead, Write},
};

mod de;
mod ser;

pub use de::*;
use io_util::{ColumnReadAdapter, IoReader, SliceReader};
use mapping_serde::{Deserialize, Serialize};
pub use ser::Serializer;

const DST_INLINE: usize = 2;

const SEPARATOR: u8 = b'\t';

/// An error occurred in serialization or deserialization.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    line: usize,
    col: usize,
}

impl Error {
    #[inline]
    fn with_loc(self, line: usize, col: usize) -> Self {
        Self { line, col, ..self }
    }
}

#[derive(Debug)]
enum ErrorKind {
    Msg(Box<str>),
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Msg(msg) => write!(f, "{msg}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Utf8(utf8_error) => write!(f, "utf8 conversion error: {utf8_error}"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "an error occured at line {}, logical column {}: {}",
            self.line, self.col, self.kind
        )
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Io(error) => Some(error),
            ErrorKind::Utf8(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Io(value),
            line: 0,
            col: 0,
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Self {
            kind: ErrorKind::Utf8(value),
            line: 0,
            col: 0,
        }
    }
}

impl mapping_serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self {
            kind: ErrorKind::Msg(msg.to_string().into_boxed_str()),
            line: 0,
            col: 0,
        }
    }
}

impl mapping_serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self {
            kind: ErrorKind::Msg(msg.to_string().into_boxed_str()),
            line: 0,
            col: 0,
        }
    }
}

/// Deserializes a value from given reader using [`IndexDeserializer`] if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_reader_indexed<R, T>(reader: R) -> Result<Option<T>, Error>
where
    R: BufRead,
    T: for<'de> Deserialize<'de>,
{
    let mut reader = IoReader::new(Box::new(reader));
    let mut stream = StreamDeserializer::new(ColumnReadAdapter::new(&mut reader))?;
    let index = Index::from_stream(&mut stream)?;
    T::deserialize(index.as_deserializer())
}

/// Deserializes a value from given byte slice using [`IndexDeserializer`] if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_slice_indexed<T>(slice: &[u8]) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let mut reader = SliceReader::new(slice);
    let mut stream = StreamDeserializer::new(ColumnReadAdapter::new(&mut reader))?;
    let index = Index::from_stream(&mut stream)?;
    T::deserialize(index.as_deserializer())
}

/// Deserializes a value from given string slice using [`IndexDeserializer`] if present.
#[inline]
#[allow(clippy::missing_errors_doc)]
pub fn from_str_indexed<T>(slice: &str) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    from_slice_indexed(slice.as_bytes())
}

/// Deserializes a value from given reader using [`PseudoTreeDeserializer`] if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_reader_fast<R, T>(reader: R) -> Result<Option<T>, Error>
where
    R: BufRead,
    T: for<'de> Deserialize<'de>,
{
    let mut reader = IoReader::new(Box::new(reader));
    let mut stream = StreamDeserializer::new(ColumnReadAdapter::new(&mut reader))?;
    T::deserialize(PseudoTreeDeserializer::new(&mut stream))
}

/// Deserializes a value from given byte slice using [`PseudoTreeDeserializer`] if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_slice_fast<T>(slice: &[u8]) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let mut reader = SliceReader::new(slice);
    let mut stream = StreamDeserializer::new(ColumnReadAdapter::new(&mut reader))?;
    T::deserialize(PseudoTreeDeserializer::new(&mut stream))
}

/// Deserializes a value from given string slice using [`PseudoTreeDeserializer`] if present.
#[inline]
#[allow(clippy::missing_errors_doc)]
pub fn from_str_fast<T>(slice: &str) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    from_slice_fast(slice.as_bytes())
}

/// Serializes a value into the given writer.
#[allow(clippy::missing_errors_doc)]
pub fn to_writer<T, W>(
    writer: W,
    value: T,
    src: &str,
    dst: &[&str],
    props: &[(&str, Option<&str>)],
) -> Result<(), Error>
where
    T: Serialize,
    W: Write,
{
    let serializer = Serializer::new(writer, src, dst, props.iter().map(|(a, b)| (*a, *b)))?;
    value.serialize(serializer)
}

/// Serializes a value into vector.
#[allow(clippy::missing_errors_doc)]
pub fn to_vec<T>(
    value: T,
    src: &str,
    dst: &[&str],
    props: &[(&str, Option<&str>)],
) -> Result<Vec<u8>, Error>
where
    T: Serialize,
{
    let mut vec = Vec::new();
    to_writer(&mut vec, value, src, dst, props)?;
    Ok(vec)
}

/// Serializes a value into string buffer.
#[allow(clippy::missing_errors_doc)]
pub fn to_string<T>(
    value: T,
    src: &str,
    dst: &[&str],
    props: &[(&str, Option<&str>)],
) -> Result<String, Error>
where
    T: Serialize,
{
    to_vec(value, src, dst, props)
        .and_then(|vec| String::from_utf8(vec).map_err(mapping_serde::ser::Error::custom))
}

#[cfg(test)]
mod tests;

//! mapping-serde support for Tiny V2 mapping format.

use std::{
    fmt::Display,
    io::{BufRead, Write},
};

mod de;
mod ser;

pub use de::Deserializer;
use io_util::{ColumnReadAdapter, IoReader, SliceReader};
use mapping_serde::{Deserialize, Serialize};
pub use ser::Serializer;

const INDENT: u8 = b'\t';
const SEPARATOR: u8 = b'\t';

const DST_INLINE: usize = 2;

/// The property key for marking item names as 'escaped'.
pub const PROPERTY_ESCAPED_NAMES: &str = "escaped-names";

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
    Unescape(fast_unescape::Error),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Msg(msg) => write!(f, "{msg}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Utf8(utf8_error) => write!(f, "utf8 conversion error: {utf8_error}"),
            Self::Unescape(error) => write!(f, "string unescape error: {error}"),
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

impl From<fast_unescape::Error> for Error {
    fn from(value: fast_unescape::Error) -> Self {
        Self {
            kind: ErrorKind::Unescape(value),
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

/// Deserializes a value from given reader if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_reader<R, T>(reader: R) -> Result<Option<T>, Error>
where
    R: BufRead,
    T: for<'de> Deserialize<'de>,
{
    let mut reader = IoReader::new(Box::new(reader));
    T::deserialize(Deserializer::new(ColumnReadAdapter::new(&mut reader))?)
}

/// Deserializes a value from given byte slice if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_slice<T>(slice: &[u8]) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let mut reader = SliceReader::new(slice);
    T::deserialize(Deserializer::new(ColumnReadAdapter::new(&mut reader))?)
}

/// Deserializes a value from given string slice if present.
#[inline]
#[allow(clippy::missing_errors_doc)]
pub fn from_str<T>(slice: &str) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    from_slice(slice.as_bytes())
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
    const MINOR_VERSION: u16 = 0;
    let serializer = Serializer::new(
        writer,
        src,
        dst,
        MINOR_VERSION,
        props.iter().map(|(a, b)| (*a, *b)),
    )?;
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
mod tests {
    mod serde;
    const TEST_MAPPING: &[u8] = include_bytes!("../testset/tinyV2.tiny");
}

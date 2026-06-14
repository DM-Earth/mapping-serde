//! mapping-serde support for Enigma mapping format.

use std::{
    fmt::Display,
    io::{BufRead, Write},
    path::Path,
};

mod de;
mod ser;
mod walk;

use io_util::{ColumnReadAdapter, IoReader, SliceReader};
use mapping_serde::{Deserialize, Serialize};

pub use de::Deserializer;
pub use ser::Serializer;
pub use walk::DirDeserializer;

const INDENT: u8 = b'\t';
const SEPARATOR: u8 = b' ';

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

/// Deserializes a value from given reader if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_reader<R, T>(reader: R, src: &str, dst: &str) -> Result<Option<T>, Error>
where
    R: BufRead,
    T: for<'de> Deserialize<'de>,
{
    let mut reader = IoReader::new(Box::new(reader));
    T::deserialize(Deserializer::new(
        src,
        dst,
        ColumnReadAdapter::new(&mut reader),
    ))
}

/// Deserializes a value from given byte slice if present.
#[allow(clippy::missing_errors_doc)]
pub fn from_slice<T>(slice: &[u8], src: &str, dst: &str) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let mut reader = SliceReader::new(slice);
    T::deserialize(Deserializer::new(
        src,
        dst,
        ColumnReadAdapter::new(&mut reader),
    ))
}

/// Deserializes a value from given string slice if present.
#[inline]
#[allow(clippy::missing_errors_doc)]
pub fn from_str<T>(slice: &str, src: &str, dst: &str) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
{
    from_slice(slice.as_bytes(), src, dst)
}

/// Deserializes a value from given directory.
#[allow(clippy::missing_errors_doc)]
pub fn from_directory<T, P>(root: P, src: &str, dst: &str) -> Result<Option<T>, Error>
where
    T: for<'de> Deserialize<'de>,
    P: AsRef<Path>,
{
    T::deserialize(DirDeserializer::new(root, src, dst)?)
}

/// Serializes a value into the given writer.
#[allow(clippy::missing_errors_doc)]
pub fn to_writer<T, W>(writer: W, value: T) -> Result<(), Error>
where
    T: Serialize,
    W: Write,
{
    value.serialize(Serializer::new(writer))
}

/// Serializes a value into vector.
#[allow(clippy::missing_errors_doc)]
pub fn to_vec<T>(value: T) -> Result<Vec<u8>, Error>
where
    T: Serialize,
{
    let mut vec = Vec::new();
    to_writer(&mut vec, value)?;
    Ok(vec)
}

/// Serializes a value into string buffer.
#[allow(clippy::missing_errors_doc)]
pub fn to_string<T>(value: T) -> Result<String, Error>
where
    T: Serialize,
{
    to_vec(value).and_then(|vec| String::from_utf8(vec).map_err(mapping_serde::ser::Error::custom))
}

#[cfg(test)]
mod tests {
    mod de;
    mod ser;

    const TEST_MAPPING: &[u8] = include_bytes!("../testset/valid.mappings");
}

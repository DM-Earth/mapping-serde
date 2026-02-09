//! mapping-serde support for Tiny V2 mapping format.

use std::fmt::Display;

mod de;

pub use de::Deserializer;

const INDENT: u8 = b'\t';
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
    Unsupported(&'static str),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Msg(msg) => write!(f, "{msg}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Utf8(utf8_error) => write!(f, "utf8 conversion error: {utf8_error}"),
            Self::Unsupported(msg) => write!(f, "unsupported: {msg}"),
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

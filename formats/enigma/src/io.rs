use std::ops::Deref;

use smol_str::{SmolStr, ToSmolStr as _};

pub(crate) use io_util::*;

#[derive(Debug, Clone)]
pub enum SmolCowStr<'a> {
    Owned(SmolStr),
    Borrowed(&'a str),
}

impl Deref for SmolCowStr<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            SmolCowStr::Owned(smol_str) => smol_str,
            SmolCowStr::Borrowed(s) => s,
        }
    }
}

impl AsRef<str> for SmolCowStr<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl<'de> From<MaybeBorrowed<'_, 'de, str>> for SmolCowStr<'de> {
    #[inline]
    fn from(value: MaybeBorrowed<'_, 'de, str>) -> Self {
        match value {
            MaybeBorrowed::Short(s) => SmolCowStr::Owned(s.to_smolstr()),
            MaybeBorrowed::Borrowed(b) => SmolCowStr::Borrowed(b),
        }
    }
}

use std::{borrow::Cow, hash::Hash, ops::Deref};

use smol_str::{SmolStr, ToSmolStr as _};

use crate::MaybeBorrowed;

/// `Cow<'_, str>` but use [`SmolStr`] as owned variant.
#[derive(Debug, Clone)]
pub enum SmolCowStr<'a> {
    /// The owned variant.
    Owned(SmolStr),
    /// The borrowed variant.
    Borrowed(&'a str),
}

impl<'a> SmolCowStr<'a> {
    /// Returns the borrowed string if it is one.
    #[inline]
    pub fn as_borrowed(&self) -> Option<&'a str> {
        match self {
            SmolCowStr::Owned(_) => None,
            SmolCowStr::Borrowed(a) => Some(*a),
        }
    }
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

impl<'de> From<Cow<'de, str>> for SmolCowStr<'de> {
    #[inline]
    fn from(value: Cow<'de, str>) -> Self {
        match value {
            Cow::Borrowed(b) => SmolCowStr::Borrowed(b),
            Cow::Owned(o) => SmolCowStr::Owned(o.into()),
        }
    }
}

impl PartialEq for SmolCowStr<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Eq for SmolCowStr<'_> {}

impl Hash for SmolCowStr<'_> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

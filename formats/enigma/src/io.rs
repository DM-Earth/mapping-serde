use std::{
    borrow::Cow,
    io::BufRead,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
};

use memchr::memchr;
use smol_str::{SmolStr, ToSmolStr as _};

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

#[derive(Debug, Clone, Copy)]
pub enum MaybeBorrowed<'s, 'de: 's, T: ?Sized> {
    Short(&'s T),
    Borrowed(&'de T),
}

impl<'s, 'de: 's, T: ?Sized> MaybeBorrowed<'s, 'de, T> {
    #[inline]
    pub fn map<F, U: ?Sized>(self, f: F) -> MaybeBorrowed<'s, 'de, U>
    where
        F: for<'a> FnOnce(&'a T) -> &'a U,
    {
        match self {
            MaybeBorrowed::Short(v) => MaybeBorrowed::Short(f(v)),
            MaybeBorrowed::Borrowed(v) => MaybeBorrowed::Borrowed(f(v)),
        }
    }

    #[inline]
    pub fn try_map<F, U: ?Sized, Err>(self, f: F) -> Result<MaybeBorrowed<'s, 'de, U>, Err>
    where
        F: for<'a> FnOnce(&'a T) -> Result<&'a U, Err>,
    {
        match self {
            MaybeBorrowed::Short(v) => f(v).map(MaybeBorrowed::Short),
            MaybeBorrowed::Borrowed(v) => f(v).map(MaybeBorrowed::Borrowed),
        }
    }

    #[inline]
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Self::Borrowed(_))
    }

    #[inline]
    pub const unsafe fn from_raw_parts(borrowed: bool, ptr: NonNull<T>) -> Self {
        if borrowed {
            Self::Borrowed(unsafe { ptr.as_ref() })
        } else {
            Self::Short(unsafe { ptr.as_ref() })
        }
    }
}

impl<'de, T: ToOwned + ?Sized> From<MaybeBorrowed<'_, 'de, T>> for Cow<'de, T> {
    #[inline]
    fn from(value: MaybeBorrowed<'_, 'de, T>) -> Self {
        match value {
            MaybeBorrowed::Short(s) => Cow::Owned(s.to_owned()),
            MaybeBorrowed::Borrowed(b) => Cow::Borrowed(b),
        }
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

impl<T: ?Sized> Deref for MaybeBorrowed<'_, '_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            MaybeBorrowed::Short(v) => v,
            MaybeBorrowed::Borrowed(v) => v,
        }
    }
}

impl<T: ?Sized> AsRef<T> for MaybeBorrowed<'_, '_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized + PartialEq> PartialEq<T> for MaybeBorrowed<'_, '_, T> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        (**self) == *other
    }
}

#[derive(Debug)]
pub enum MaybeMut<'a, T> {
    Owned(T),
    Mut(&'a mut T),
}

impl<T> MaybeMut<'_, T> {
    #[inline]
    pub fn reclaim(&mut self) -> MaybeMut<'_, T> {
        MaybeMut::Mut(&mut *self)
    }
}

impl<T> DerefMut for MaybeMut<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MaybeMut::Owned(v) => v,
            MaybeMut::Mut(v) => v,
        }
    }
}

impl<T> Deref for MaybeMut<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            MaybeMut::Owned(v) => v,
            MaybeMut::Mut(v) => v,
        }
    }
}

/// Borrowed read adaptor.
///
/// # Safety
///
/// `last_read` method should guarantees that its returned slice won't expire until next read,
/// as well as the `read_until` method.
pub unsafe trait Read<'de> {
    fn read_until(&mut self, separator: u8) -> std::io::Result<MaybeBorrowed<'_, 'de, [u8]>>;

    fn last_read(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>>;
}

pub trait ColumnRead<'de> {
    /// Parses next column.
    fn read_col(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<Option<MaybeBorrowed<'_, 'de, [u8]>>>;

    /// Parses next column.
    fn read_cols<const N: usize>(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<[Option<MaybeBorrowed<'_, 'de, [u8]>>; N]>;

    /// Jumps to next line and returns the ident depth.
    fn next_line(&mut self, ident: u8) -> std::io::Result<Option<usize>>;

    fn last_col(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>>;
    fn this_line(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>>;
}

unsafe impl<'de, T> Read<'de> for &mut T
where
    T: Read<'de>,
{
    #[inline]
    fn read_until(&mut self, separator: u8) -> std::io::Result<MaybeBorrowed<'_, 'de, [u8]>> {
        T::read_until(self, separator)
    }

    #[inline]
    fn last_read(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        T::last_read(self)
    }
}

impl<'de, T> ColumnRead<'de> for &mut T
where
    T: ColumnRead<'de>,
{
    #[inline]
    fn read_col(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<Option<MaybeBorrowed<'_, 'de, [u8]>>> {
        T::read_col(self, col_separator)
    }

    #[inline]
    fn next_line(&mut self, ident: u8) -> std::io::Result<Option<usize>> {
        T::next_line(self, ident)
    }

    #[inline]
    fn last_col(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        T::last_col(self)
    }

    #[inline]
    fn this_line(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        T::this_line(self)
    }

    #[inline]
    fn read_cols<const N: usize>(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<[Option<MaybeBorrowed<'_, 'de, [u8]>>; N]> {
        T::read_cols(self, col_separator)
    }
}

#[derive(Debug, Clone)]
pub struct SliceReader<'a> {
    remaining: &'a [u8],
    last: Option<&'a [u8]>,
}

impl<'a> SliceReader<'a> {
    pub const fn new(slice: &'a [u8]) -> Self {
        Self {
            remaining: slice,
            last: None,
        }
    }
}

unsafe impl<'de> Read<'de> for SliceReader<'de> {
    fn read_until(&mut self, separator: u8) -> std::io::Result<MaybeBorrowed<'_, 'de, [u8]>> {
        if let Some(loc) = memchr(separator, self.remaining) {
            let (sliced, rest) = unsafe { self.remaining.split_at_unchecked(loc) };
            self.remaining = rest;
            self.last = Some(sliced);
            Ok(MaybeBorrowed::Borrowed(sliced))
        } else {
            let slice = self.remaining;
            self.last = Some(slice);
            self.remaining = &[];
            Ok(MaybeBorrowed::Borrowed(slice))
        }
    }

    #[inline]
    fn last_read(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        self.last.map(MaybeBorrowed::Borrowed)
    }
}

#[derive(Debug, Clone)]
pub struct IoReader<R> {
    buf: Vec<u8>,
    inner: R,
}

impl<R> IoReader<R> {
    pub const fn new(inner: R) -> Self {
        Self {
            buf: Vec::new(),
            inner,
        }
    }
}

unsafe impl<'de, R> Read<'de> for IoReader<R>
where
    R: BufRead,
{
    fn read_until(&mut self, separator: u8) -> std::io::Result<MaybeBorrowed<'_, 'de, [u8]>> {
        self.buf.clear();
        self.inner.read_until(separator, &mut self.buf)?;
        Ok(MaybeBorrowed::Short(self.buf.as_slice()))
    }

    #[inline]
    fn last_read(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        Some(MaybeBorrowed::Short(self.buf.as_slice()))
    }
}

#[derive(Debug, Clone)]
pub struct ColumnReadAdapter<R> {
    line: Option<NonNull<[u8]>>,
    last_col: Option<NonNull<[u8]>>,
    borrowed: bool,
    inner: Pin<R>,
}

impl<R> ColumnReadAdapter<R>
where
    R: Deref<Target: Unpin>,
{
    pub const fn new(inner: R) -> Self {
        Self {
            line: None,
            last_col: None,
            borrowed: false,
            inner: Pin::new(inner),
        }
    }
}

impl<'de, R> ColumnRead<'de> for ColumnReadAdapter<R>
where
    R: Deref<Target: Read<'de> + Unpin> + DerefMut,
{
    fn read_col(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<Option<MaybeBorrowed<'_, 'de, [u8]>>> {
        let Some(line) = self.this_line() else {
            return Ok(None);
        };
        if line.is_empty() {
            return Ok(None);
        }
        let line = &*line;
        let sptr = if let Some(pos) = memchr(col_separator, line) {
            let (col, mut rest) = unsafe { line.split_at_unchecked(pos) };
            rest.split_off_first();
            let ret = NonNull::from_ref(col);
            self.line = Some(NonNull::from_ref(rest));
            ret
        } else {
            let ret = NonNull::from_ref(line);
            self.line = None;
            ret
        };
        self.last_col = Some(sptr);
        Ok((*self).last_col())
    }

    fn read_cols<const N: usize>(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<[Option<MaybeBorrowed<'_, 'de, [u8]>>; N]> {
        let mut arr = [const { None }; N];
        let mut ptr = NonNull::from_mut(self);
        for e in &mut arr {
            // SAFETY: multiple columns coexist inside one line thus its safe
            *e = unsafe { ptr.as_mut() }.read_col(col_separator)?;
        }
        Ok(arr)
    }

    fn next_line(&mut self, ident: u8) -> std::io::Result<Option<usize>> {
        let mut slice = self.inner.as_mut().get_mut().read_until(b'\n')?;
        if slice.is_empty() {
            return Ok(None);
        }
        let mut count = 0;
        slice = slice.map(|mut s| {
            while let [first, rest @ ..] = s {
                if *first == ident {
                    s = rest;
                    count += 1;
                } else {
                    break;
                }
            }
            s.trim_ascii()
        });
        self.borrowed = slice.is_borrowed();
        self.line = Some(NonNull::from_ref(&slice));
        Ok(Some(count))
    }

    #[inline]
    fn last_col(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        self.last_col
            .map(|ptr| unsafe { MaybeBorrowed::from_raw_parts(self.borrowed, ptr) })
    }

    #[inline]
    fn this_line(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        self.line
            .map(|ptr| unsafe { MaybeBorrowed::from_raw_parts(self.borrowed, ptr) })
    }
}

#[derive(Debug, Clone)]
pub struct ColumnReader<R> {
    ident: u8,
    col_separator: u8,
    line: usize,
    col: usize,
    fresh_line: bool,

    inner: R,
}

impl<R> ColumnReader<R> {
    pub const fn new(ident: u8, col_separator: u8, inner: R) -> Self {
        Self {
            ident,
            col_separator,
            line: 0,
            col: 0,
            fresh_line: false,
            inner,
        }
    }

    #[inline]
    pub fn col(&self) -> usize {
        self.col
    }

    #[inline]
    pub fn line(&self) -> usize {
        self.line
    }

    #[inline]
    pub fn is_fresh_line(&self) -> bool {
        self.fresh_line
    }
}

impl<'de, R> ColumnReader<R>
where
    R: ColumnRead<'de>,
{
    #[inline]
    pub fn last_col(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        self.inner.last_col()
    }

    pub fn read_col(&mut self) -> std::io::Result<Option<MaybeBorrowed<'_, 'de, [u8]>>> {
        let col = self.inner.read_col(self.col_separator)?;
        self.fresh_line = false;
        if col.is_some() {
            self.col += 1;
        }
        Ok(col)
    }

    pub fn read_cols<const N: usize>(
        &mut self,
    ) -> std::io::Result<[Option<MaybeBorrowed<'_, 'de, [u8]>>; N]> {
        let cols = self.inner.read_cols(self.col_separator)?;
        self.fresh_line = false;
        self.col += cols.iter().filter(|o| o.is_some()).count();
        Ok(cols)
    }

    pub fn next_line(&mut self) -> std::io::Result<Option<usize>> {
        while let Some(ident) = self.inner.next_line(self.ident)? {
            self.line += 1;
            self.fresh_line = true;
            if self.inner.this_line().is_some_and(|l| !l.is_empty()) {
                return Ok(Some(ident));
            }
        }
        Ok(None)
    }

    #[inline]
    pub fn this_line(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        self.inner.this_line()
    }
}

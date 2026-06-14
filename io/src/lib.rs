//! I/O utilities for Java deobfuscation mappings' deserialization.

#![allow(clippy::missing_errors_doc, clippy::exhaustive_enums)]

use std::{
    borrow::Cow,
    io::BufRead,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
};

use memchr::memchr;

#[cfg(feature = "smol-str")]
mod smol_str;
#[cfg(feature = "smol-str")]
pub use smol_str::SmolCowStr;

/// Reference that could have two variants of lifetime.
#[derive(Debug)]
pub enum MaybeBorrowed<'s, 'de: 's, T: ?Sized> {
    /// Semantically temporary reference.
    Short(&'s T),
    /// Semantically borrowed reference.
    Borrowed(&'de T),
}

impl<'s, 'de: 's, T: ?Sized> MaybeBorrowed<'s, 'de, T> {
    /// Maps the inner reference into another type of reference.
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

    /// Maps the inner reference into another type of reference, or fails if the given function failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the given function do so.
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

    /// Whether this is a semantically borrowed-lifetime reference.
    #[inline]
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Self::Borrowed(_))
    }

    /// Returns the borrowed variant if valid.
    #[inline]
    pub const fn as_borrowed(&self) -> Option<&'de T> {
        match self {
            MaybeBorrowed::Short(_) => None,
            MaybeBorrowed::Borrowed(b) => Some(b),
        }
    }

    /// Returns the short variant if valid.
    #[inline]
    pub const fn as_short(&self) -> &'s T {
        match self {
            MaybeBorrowed::Short(s) => s,
            MaybeBorrowed::Borrowed(b) => b,
        }
    }

    /// Assembles a reference from raw parts - whether it is borrowed and the raw pointer.
    ///
    /// # Safety
    ///
    /// It dereferences the pointer without soundness guaranteed and without lifetime information.
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

impl From<MaybeBorrowed<'_, '_, str>> for String {
    #[inline]
    fn from(value: MaybeBorrowed<'_, '_, str>) -> Self {
        (*value).into()
    }
}

impl From<MaybeBorrowed<'_, '_, str>> for Box<str> {
    #[inline]
    fn from(value: MaybeBorrowed<'_, '_, str>) -> Self {
        (*value).into()
    }
}

#[cfg(feature = "smol-str")]
impl From<MaybeBorrowed<'_, '_, str>> for ::smol_str::SmolStr {
    #[inline]
    fn from(value: MaybeBorrowed<'_, '_, str>) -> Self {
        (*value).into()
    }
}

impl<T: ?Sized> Deref for MaybeBorrowed<'_, '_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_short()
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

impl<T: ?Sized> Clone for MaybeBorrowed<'_, '_, T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for MaybeBorrowed<'_, '_, T> {}

impl Default for MaybeBorrowed<'_, '_, str> {
    #[inline]
    fn default() -> Self {
        Self::Borrowed("")
    }
}

/// A mutable value that could be owned or be a mutable reference, possibly inherited from a parent node.
#[derive(Debug)]
pub enum MaybeMut<'a, T> {
    /// An owned value.
    Owned(T),
    /// A mutable reference.
    Mut(&'a mut T),
}

impl<T> MaybeMut<'_, T> {
    /// Reborrows the underlying value into a mutable reference.
    ///
    /// This is same as `MaybeMut::Mut(self)`.
    #[inline]
    #[doc(alias = "reborrow")]
    pub fn reclaim(&mut self) -> MaybeMut<'_, T> {
        MaybeMut::Mut(self)
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

/// Borrowed readers.
///
/// # Safety
///
/// `last_read` method should guarantees that its returned slice won't expire until next read,
/// as well as the `read_until` method.
pub unsafe trait Read<'de> {
    /// Reads the remaining bytes until occurs the given `separator`, or EOF, and returns the read bytes in slice,
    /// which means the slice should be buffered in an adapter implementing this trait.
    ///
    /// The previous-read bytes could then be safely discarded.
    fn read_until(&mut self, separator: u8) -> std::io::Result<MaybeBorrowed<'_, 'de, [u8]>>;

    /// Returns the latest bytes returned from `read_until`.
    fn last_read(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>>;
}

/// Borrowed column-based readers.
pub trait ColumnRead<'de> {
    /// Parses next column.
    fn read_col(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<Option<MaybeBorrowed<'_, 'de, [u8]>>>;

    /// Parses next columns.
    fn read_cols<const N: usize>(
        &mut self,
        col_separator: u8,
    ) -> std::io::Result<[Option<MaybeBorrowed<'_, 'de, [u8]>>; N]> {
        let mut arr = [const { None }; N];
        for (a, b) in arr.iter_mut().zip(self.iter_cols(col_separator)) {
            *a = b.ok();
        }
        Ok(arr)
    }

    /// Returns an iterator over columns in the current line.
    fn iter_cols<'s>(
        &'s mut self,
        col_separator: u8,
    ) -> impl Iterator<Item = std::io::Result<MaybeBorrowed<'s, 'de, [u8]>>>
    where
        'de: 's;

    /// Jumps to next line and returns the indent depth.
    fn next_line(&mut self, indent: u8) -> std::io::Result<Option<usize>>;

    /// The latest column bytes.
    fn last_col(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>>;

    /// The latest line bytes, would be truncated if some columns have already been read.
    fn this_line(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>>;

    /// The latest line indentation count.
    fn this_indent(&self) -> Option<usize>;
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

unsafe impl<'de, T> Read<'de> for Box<T>
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
    fn next_line(&mut self, indent: u8) -> std::io::Result<Option<usize>> {
        T::next_line(self, indent)
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

    #[inline]
    fn iter_cols<'s>(
        &'s mut self,
        col_separator: u8,
    ) -> impl Iterator<Item = std::io::Result<MaybeBorrowed<'s, 'de, [u8]>>>
    where
        'de: 's,
    {
        T::iter_cols(self, col_separator)
    }

    #[inline]
    fn this_indent(&self) -> Option<usize> {
        T::this_indent(self)
    }
}

impl<'de, T> ColumnRead<'de> for Box<T>
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
    fn next_line(&mut self, indent: u8) -> std::io::Result<Option<usize>> {
        T::next_line(self, indent)
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

    #[inline]
    fn iter_cols<'s>(
        &'s mut self,
        col_separator: u8,
    ) -> impl Iterator<Item = std::io::Result<MaybeBorrowed<'s, 'de, [u8]>>>
    where
        'de: 's,
    {
        T::iter_cols(self, col_separator)
    }

    #[inline]
    fn this_indent(&self) -> Option<usize> {
        T::this_indent(self)
    }
}

/// [`Read`] adapter for borrowed slices.
#[derive(Debug, Clone)]
pub struct SliceReader<'a> {
    remaining: &'a [u8],
    last: Option<&'a [u8]>,
}

impl<'a> SliceReader<'a> {
    /// Creates a new reader from given slice.
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
            let (sliced, mut rest) = unsafe { self.remaining.split_at_unchecked(loc) };
            rest.split_off_first();
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

/// [`Read`] adapter for I/O readers.
#[derive(Debug, Clone)]
pub struct IoReader<R> {
    buf: Vec<u8>,
    inner: R,
}

impl<R> IoReader<R> {
    /// Creates a new reader from given I/O reader.
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

/// A simple [`ColumnRead`] implementation around [`Read`] trait.
#[derive(Debug, Clone)]
pub struct ColumnReadAdapter<R> {
    line: Option<NonNull<[u8]>>,
    indent: Option<usize>,
    last_col: Option<NonNull<[u8]>>,
    borrowed: bool,
    inner: Pin<R>,
}

impl<R> ColumnReadAdapter<R>
where
    R: Deref<Target: Unpin>,
{
    /// Creates a new column read adapter from given reader.
    ///
    /// The reader should be properly pinned somewhere as the current implementation
    /// relies on self-reference.
    pub const fn new(inner: R) -> Self {
        Self {
            line: None,
            indent: None,
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

    fn iter_cols<'s>(
        &'s mut self,
        col_separator: u8,
    ) -> impl Iterator<Item = std::io::Result<MaybeBorrowed<'s, 'de, [u8]>>>
    where
        'de: 's,
    {
        let mut ptr = NonNull::from_mut(self);
        // SAFETY: multiple columns coexist inside one line thus its safe
        std::iter::from_fn(move || unsafe { ptr.as_mut() }.read_col(col_separator).transpose())
    }

    fn next_line(&mut self, indent: u8) -> std::io::Result<Option<usize>> {
        let mut slice = self.inner.as_mut().get_mut().read_until(b'\n')?;
        if slice.is_empty() {
            return Ok(None);
        }
        let mut count = 0;
        slice = slice.map(|mut s| {
            while let [first, rest @ ..] = s {
                if *first == indent {
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
        self.indent = Some(count);
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

    #[inline]
    fn this_indent(&self) -> Option<usize> {
        self.indent
    }
}

/// High-level column-based reader for human.
#[derive(Debug, Clone)]
pub struct ColumnReader<R> {
    indent: u8,
    col_separator: u8,
    line: usize,
    col: usize,
    fresh_line: bool,

    inner: R,
}

impl<R> ColumnReader<R> {
    /// Creates a new column reader from given properties and the underlying read adapter.
    pub const fn new(indent: u8, col_separator: u8, inner: R) -> Self {
        Self {
            indent,
            col_separator,
            line: 0,
            col: 0,
            fresh_line: false,
            inner,
        }
    }

    /// Returns the current logical column number.
    #[inline]
    pub fn col(&self) -> usize {
        self.col
    }

    /// Returns the current line number.
    #[inline]
    pub fn line(&self) -> usize {
        self.line
    }

    /// Whether the current line is freshly-read (no columns been accessed yet).
    #[inline]
    pub fn is_fresh_line(&self) -> bool {
        self.fresh_line
    }

    /// Marks the current line dirty.
    ///
    /// See [`Self::is_fresh_line`] for details.
    #[inline]
    pub fn mark_dirty(&mut self) {
        self.fresh_line = false;
    }
}

impl<'de, R> ColumnReader<R>
where
    R: ColumnRead<'de>,
{
    /// The latest column bytes.
    #[inline]
    pub fn last_col(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        self.inner.last_col()
    }

    /// Parses next column.
    pub fn read_col(&mut self) -> std::io::Result<Option<MaybeBorrowed<'_, 'de, [u8]>>> {
        let col = self.inner.read_col(self.col_separator)?;
        self.fresh_line = false;
        if col.is_some() {
            self.col += 1;
        }
        Ok(col)
    }

    /// Parses next columns.
    pub fn read_cols<const N: usize>(
        &mut self,
    ) -> std::io::Result<[Option<MaybeBorrowed<'_, 'de, [u8]>>; N]> {
        let cols = self.inner.read_cols(self.col_separator)?;
        self.fresh_line = false;
        self.col += cols.iter().filter(|o| o.is_some()).count();
        Ok(cols)
    }

    /// Returns an iterator over columns in the current line.
    pub fn iter_cols<'s>(
        &'s mut self,
    ) -> impl Iterator<Item = std::io::Result<MaybeBorrowed<'s, 'de, [u8]>>>
    where
        'de: 's,
    {
        self.inner.iter_cols(self.col_separator).inspect(|b| {
            if b.is_ok() {
                self.col += 1
            }
        })
    }

    /// Jumps to next line and returns the indent depth.
    pub fn next_line(&mut self) -> std::io::Result<Option<usize>> {
        while let Some(indent) = self.inner.next_line(self.indent)? {
            self.line += 1;
            self.col = 0;
            self.fresh_line = true;
            if self.inner.this_line().is_some_and(|l| !l.is_empty()) {
                return Ok(Some(indent));
            }
        }
        Ok(None)
    }

    /// The latest line bytes, would be truncated if some columns have already been read.
    #[inline]
    pub fn this_line(&self) -> Option<MaybeBorrowed<'_, 'de, [u8]>> {
        self.inner.this_line()
    }

    /// The latest line indentation count.
    #[inline]
    pub fn this_indent(&self) -> Option<usize> {
        self.inner.this_indent()
    }
}

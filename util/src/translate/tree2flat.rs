use std::{boxed::Box, string::String, vec::Vec};

use io_util::SmolCowStr;
use mapping_serde::{
    Deserializer,
    de::{ClassAccess, FieldAccess, MethodAccess, MethodArgAccess, MethodVarAccess, Visitor},
};
use smallvec::SmallVec;

use crate::{
    RefVisitor,
    translate::{Class, Content, DST_INLINE, Described, MethodArg, MethodVar},
};

struct FlattenedVisitor<'env, 'de, V> {
    inner: V,
    flat: &'env mut Vec<Class<'de>>,
    // class name stack, containing both source and destinations
    stack: &'env mut Vec<SmallVec<[SmolCowStr<'de>; DST_INLINE]>>,
}

enum ControlFlow<T, V> {
    Return(T),
    Continue(V),
}

const CLASS_SPLIT: &str = "$";
const CLASS_SPLIT_BYTE: u8 = CLASS_SPLIT.as_bytes()[0];

impl<'de, V> Visitor<'de> for FlattenedVisitor<'_, 'de, V>
where
    V: Visitor<'de>,
{
    type Value = ControlFlow<V::Value, V>;

    #[inline]
    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.inner.expecting(f)
    }

    fn visit_class<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'b>,
    {
        self.stack.push(
            std::iter::once(access.src())
                .chain(access.dst())
                .map(Into::into)
                .map(SmolCowStr::Owned)
                .collect(),
        );
        let mut class = Class::from_access(access, |content, de| {
            fill_contents(content, de, self.flat, self.stack)
        })?;
        let stacked = (0..class.dst.len() + 1).map(|i| {
            let mut comb = self
                .stack
                .iter()
                .map(move |s| s.get(i).unwrap_or(&s[0]))
                .flat_map(|s| [&**s, CLASS_SPLIT])
                .collect::<String>();
            if comb
                .as_bytes()
                .get(comb.len() - 1)
                .is_some_and(|&b| b == CLASS_SPLIT_BYTE)
            {
                comb.pop();
            }
            comb
        });
        std::iter::once(&mut class.src)
            .chain(class.dst.as_mut_slice())
            .zip(stacked)
            .for_each(|(dst, src)| *dst = SmolCowStr::Owned(src.into()));
        self.flat.push(class);
        self.stack.pop();
        Ok(ControlFlow::Continue(self.inner))
    }

    fn visit_class_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'de>,
    {
        self.stack.push(
            std::iter::once(access.src())
                .chain(access.dst())
                .map(SmolCowStr::Borrowed)
                .collect(),
        );
        let mut class = Class::from_access_borrowed(access, |content, de| {
            fill_contents(content, de, self.flat, self.stack)
        })?;
        let stacked = (0..class.dst.len() + 1).map(|i| {
            let mut comb = self
                .stack
                .iter()
                .map(move |s| &s[i])
                .flat_map(|s| [&**s, CLASS_SPLIT])
                .collect::<String>();
            if comb
                .as_bytes()
                .get(comb.len() - 1)
                .is_some_and(|&b| b == CLASS_SPLIT_BYTE)
            {
                comb.pop();
            }
            comb
        });
        std::iter::once(&mut class.src)
            .chain(class.dst.as_mut_slice())
            .zip(stacked)
            .for_each(|(dst, src)| *dst = SmolCowStr::Owned(src.into()));
        self.flat.push(class);
        self.stack.pop();
        Ok(ControlFlow::Continue(self.inner))
    }

    #[inline]
    fn visit_comment<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: mapping_serde::de::Error,
    {
        self.inner.visit_comment(value).map(ControlFlow::Return)
    }

    #[inline]
    fn visit_comment_borrowed<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: mapping_serde::de::Error,
    {
        self.inner
            .visit_comment_borrowed(value)
            .map(ControlFlow::Return)
    }

    #[inline]
    fn visit_field<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: FieldAccess<'de, 'b>,
    {
        self.inner.visit_field(access).map(ControlFlow::Return)
    }

    #[inline]
    fn visit_field_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: FieldAccess<'de, 'de>,
    {
        self.inner
            .visit_field_borrowed(access)
            .map(ControlFlow::Return)
    }

    #[inline]
    fn visit_method<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodAccess<'de, 'b>,
    {
        self.inner.visit_method(access).map(ControlFlow::Return)
    }

    #[inline]
    fn visit_method_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodAccess<'de, 'de>,
    {
        self.inner
            .visit_method_borrowed(access)
            .map(ControlFlow::Return)
    }

    #[inline]
    fn visit_method_arg<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodArgAccess<'de, 'b>,
    {
        self.inner.visit_method_arg(access).map(ControlFlow::Return)
    }

    #[inline]
    fn visit_method_arg_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodArgAccess<'de, 'de>,
    {
        self.inner
            .visit_method_arg_borrowed(access)
            .map(ControlFlow::Return)
    }

    #[inline]
    fn visit_method_var<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodVarAccess<'de, 'b>,
    {
        self.inner.visit_method_var(access).map(ControlFlow::Return)
    }

    #[inline]
    fn visit_method_var_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: MethodVarAccess<'de, 'de>,
    {
        self.inner
            .visit_method_var_borrowed(access)
            .map(ControlFlow::Return)
    }
}

struct PlainVisitor<'env, 'de> {
    contents: &'env mut Vec<Content<'de>>,
}

impl<'de> Visitor<'de> for PlainVisitor<'_, 'de> {
    type Value = ();

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "an element that is not class")
    }

    fn visit_class<'b, A>(self, _access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'b>,
    {
        unreachable!("classes should be handled in flattened visitor")
    }

    fn visit_class_borrowed<A>(self, _access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'de>,
    {
        unreachable!("classes should be handled in flattened visitor")
    }

    // we won't receive any classes here therefore they won't contain subclasses
    push_contents!(contents, fill_contents_plain);
}

fn fill_contents_plain<'de, D>(
    contents: &mut Vec<Content<'de>>,
    mut deser: D,
) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    loop {
        match deser.deserialize_any(PlainVisitor { contents }) {
            Ok(Some(_)) => continue,
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        }
    }
}

fn fill_contents<'de, D>(
    contents: &mut Vec<Content<'de>>,
    mut deser: D,
    flat: &mut Vec<Class<'de>>,
    stack: &mut Vec<SmallVec<[SmolCowStr<'de>; DST_INLINE]>>,
) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    loop {
        match deser.deserialize_any(FlattenedVisitor {
            inner: PlainVisitor { contents },
            flat,
            stack,
        }) {
            Ok(None) => return Ok(()),
            Ok(Some(ControlFlow::Return(_) | ControlFlow::Continue(_))) => continue,
            Err(err) => return Err(err),
        }
    }
}

struct ClassWrap<'env, 'de, D> {
    inner: &'env Class<'de>,
    content: Box<[Content<'de>]>,
    d: &'env D,
}

impl<'env, 'de, D> ClassAccess<'de, 'env> for ClassWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer = PlainContentDeserializer<'env, 'de, D>;

    fn src(&self) -> &'env str {
        &self.inner.src
    }

    fn dst(&self) -> impl Iterator<Item = &'env str> {
        self.inner.dst.iter().map(|s| &**s)
    }

    fn content(self) -> Self::ContentDeserializer {
        PlainContentDeserializer {
            contents: self.content.into_iter(),
            d: self.d,
        }
    }
}

struct DescribedWrap<'env, 'de, D> {
    inner: &'env Described<'de>,
    content: Box<[Content<'de>]>,
    d: &'env D,
}

impl<'env, 'de, D> MethodAccess<'de, 'env> for DescribedWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer = PlainContentDeserializer<'env, 'de, D>;

    fn src(&self) -> &'env str {
        &self.inner.src
    }

    fn dst(&self) -> impl Iterator<Item = &'env str> {
        self.inner.dst.iter().map(|s| &**s)
    }

    fn desc(&self) -> Option<&'env str> {
        self.inner.desc.as_deref()
    }

    fn dst_desc(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.inner
            .dst_desc
            .as_deref()
            .map(|v| v.iter().map(|s| &**s))
    }

    fn content(self) -> Self::ContentDeserializer {
        PlainContentDeserializer {
            contents: self.content.into_iter(),
            d: self.d,
        }
    }
}

impl<'env, 'de, D> FieldAccess<'de, 'env> for DescribedWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer = PlainContentDeserializer<'env, 'de, D>;

    fn src(&self) -> &'env str {
        &self.inner.src
    }

    fn dst(&self) -> impl Iterator<Item = &'env str> {
        self.inner.dst.iter().map(|s| &**s)
    }

    fn desc(&self) -> Option<&'env str> {
        self.inner.desc.as_deref()
    }

    fn dst_desc(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.inner
            .dst_desc
            .as_deref()
            .map(|v| v.iter().map(|s| &**s))
    }

    fn content(self) -> Self::ContentDeserializer {
        PlainContentDeserializer {
            contents: self.content.into_iter(),
            d: self.d,
        }
    }
}

struct MethodArgWrap<'env, 'de, D> {
    inner: &'env MethodArg<'de>,
    content: Box<[Content<'de>]>,
    d: &'env D,
}

impl<'env, 'de, D> MethodArgAccess<'de, 'env> for MethodArgWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer = PlainContentDeserializer<'env, 'de, D>;

    fn src(&self) -> Option<&'env str> {
        self.inner.src.as_deref()
    }

    fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.inner.dst.as_deref().map(|v| v.iter().map(|s| &**s))
    }

    fn pos(&self) -> Option<usize> {
        self.inner.pos
    }

    fn lv_index(&self) -> Option<usize> {
        self.inner.lv_index
    }

    fn content(self) -> Self::ContentDeserializer {
        PlainContentDeserializer {
            contents: self.content.into_iter(),
            d: self.d,
        }
    }
}

struct MethodVarWrap<'env, 'de, D> {
    inner: &'env MethodVar<'de>,
    content: Box<[Content<'de>]>,
    d: &'env D,
}

impl<'env, 'de, D> MethodVarAccess<'de, 'env> for MethodVarWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer = PlainContentDeserializer<'env, 'de, D>;

    fn src(&self) -> Option<&'env str> {
        self.inner.src.as_deref()
    }

    fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.inner.dst.as_deref().map(|v| v.iter().map(|s| &**s))
    }

    fn lv_index(&self) -> Option<usize> {
        self.inner.lv_index
    }

    fn lvt_row_index(&self) -> Option<usize> {
        self.inner.lvt_row_index
    }

    fn op_idx(&self) -> Option<(usize, Option<usize>)> {
        self.inner.op_idx
    }

    fn content(self) -> Self::ContentDeserializer {
        PlainContentDeserializer {
            contents: self.content.into_iter(),
            d: self.d,
        }
    }
}

struct PlainContentDeserializer<'env, 'de, D> {
    contents: std::vec::IntoIter<Content<'de>>,
    d: &'env D,
}

impl<'de, D> Deserializer<'de> for PlainContentDeserializer<'_, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.d.src_namespace()
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.d.dst_namespaces()
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.contents.next() {
            Some(Content::Class(_)) => unreachable!("unexpected subclass in flattened structure"),
            Some(Content::Comment(SmolCowStr::Owned(comment))) => {
                visitor.visit_comment(&comment).map(Some)
            }
            Some(Content::Comment(SmolCowStr::Borrowed(comment))) => {
                visitor.visit_comment_borrowed(comment).map(Some)
            }
            Some(Content::Described(mut described)) => {
                let content = std::mem::take(&mut described.content);
                let access = DescribedWrap {
                    inner: &described,
                    content,
                    d: self.d,
                };
                match described.kind {
                    super::DescribedKind::Method => visitor.visit_method(access),
                    super::DescribedKind::Field => visitor.visit_field(access),
                }
                .map(Some)
            }
            Some(Content::MethodArg(mut arg)) => {
                let content = std::mem::take(&mut arg.content);
                let access = MethodArgWrap {
                    inner: &arg,
                    content,
                    d: self.d,
                };
                visitor.visit_method_arg(access).map(Some)
            }
            Some(Content::MethodVar(mut var)) => {
                let content = std::mem::take(&mut var.content);
                let access = MethodVarWrap {
                    inner: &var,
                    content,
                    d: self.d,
                };
                visitor.visit_method_var(access).map(Some)
            }
            None => Ok(None),
        }
    }
}

struct FlattenedDeserializer<'de, D> {
    inner: D,
    flat: Vec<Class<'de>>,
    stack: Vec<SmallVec<[SmolCowStr<'de>; DST_INLINE]>>,
}

impl<'de, D> FlattenedDeserializer<'de, D>
where
    D: Deserializer<'de>,
{
    fn deserialize_impl<V>(
        &mut self,
        visitor: V,
    ) -> ControlFlow<Result<Option<V::Value>, D::Error>, V>
    where
        V: Visitor<'de>,
    {
        if let Some(mut class) = self.flat.pop() {
            let content = std::mem::take(&mut class.content);
            let access = ClassWrap {
                inner: &class,
                content,
                d: &self.inner,
            };
            ControlFlow::Return(visitor.visit_class(access).map(Some))
        } else {
            let mut visitor = RefVisitor::new(visitor);
            match self.inner.deserialize_any(FlattenedVisitor {
                inner: &mut visitor,
                flat: &mut self.flat,
                stack: &mut self.stack,
            }) {
                Ok(Some(ControlFlow::Continue(_))) => {
                    ControlFlow::Continue(visitor.into_inner().unwrap())
                }
                Ok(Some(ControlFlow::Return(val))) => ControlFlow::Return(Ok(Some(val))),
                Ok(None) => {
                    if let Some(visitor) = visitor.into_inner()
                        && !self.flat.is_empty()
                    {
                        ControlFlow::Continue(visitor)
                    } else {
                        ControlFlow::Return(Ok(None))
                    }
                }
                Err(err) => ControlFlow::Return(Err(err)),
            }
        }
    }
}

impl<'de, D> Deserializer<'de> for FlattenedDeserializer<'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.inner.src_namespace()
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.inner.dst_namespaces()
    }

    fn deserialize_any<V>(&mut self, mut visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        loop {
            match self.deserialize_impl(visitor) {
                ControlFlow::Return(result) => return result,
                ControlFlow::Continue(v) => visitor = v,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a, b) = self.inner.size_hint();
        let s = self.flat.len();
        (a + s, b.map(|b| b + s))
    }
}

/// Flattened deserializer that convert tree mapping into flat-style ones.
pub struct Flatten<'de, D> {
    inner: FlattenedDeserializer<'de, D>,
}

impl<D> Flatten<'_, D> {
    pub(crate) fn new(deserializer: D) -> Self {
        Self {
            inner: FlattenedDeserializer {
                inner: deserializer,
                flat: Vec::new(),
                stack: Vec::new(),
            },
        }
    }
}

impl<'de, D> Deserializer<'de> for Flatten<'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.inner.src_namespace()
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.inner.dst_namespaces()
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        if D::FLAT_CLASSES {
            self.inner.inner.deserialize_any(visitor)
        } else {
            self.inner.deserialize_any(visitor)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if D::FLAT_CLASSES {
            self.inner.inner.size_hint()
        } else {
            self.inner.size_hint()
        }
    }
}

impl<D> std::fmt::Debug for Flatten<'_, D>
where
    D: std::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Flatten")
            .field("inner", &self.inner.inner)
            .finish()
    }
}

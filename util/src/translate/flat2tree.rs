use core::{fmt::Debug, iter};
use std::{
    collections::HashMap,
    vec::{self, Vec},
};

use io_util::SmolCowStr;
use mapping_serde::{
    Deserializer,
    de::{ClassAccess, FieldAccess, MethodAccess, MethodArgAccess, MethodVarAccess, Visitor},
};

use crate::translate::{Class, Content, Described, MethodArg, MethodVar};

struct Index<'de> {
    classes: Vec<ClassDesc<'de>>,
    top: Vec<usize>,
    top_leftover: Vec<Content<'de>>,
}

#[derive(Default)]
struct ClassDesc<'de> {
    class: Option<Class<'de>>,
    sibs: Vec<usize>,
}

macro_rules! push_contents {
    ($v:ident) => {
        fn visit_comment<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: mapping_serde::de::Error,
        {
            self.$v
                .push(Content::Comment(SmolCowStr::Owned(value.into())));
            Ok(())
        }

        fn visit_comment_borrowed<E>(self, value: &'de str) -> Result<Self::Value, E>
        where
            E: mapping_serde::de::Error,
        {
            self.$v.push(Content::Comment(SmolCowStr::Borrowed(value)));
            Ok(())
        }

        fn visit_field<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::FieldAccess<'de, 'b>,
        {
            self.$v
                .push(Content::Described(Described::from_field_access(
                    access,
                    fill_contents_plain,
                )?));
            Ok(())
        }

        fn visit_field_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::FieldAccess<'de, 'de>,
        {
            self.$v
                .push(Content::Described(Described::from_field_access_borrowed(
                    access,
                    fill_contents_plain,
                )?));
            Ok(())
        }

        fn visit_method<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodAccess<'de, 'b>,
        {
            self.$v
                .push(Content::Described(Described::from_method_access(
                    access,
                    fill_contents_plain,
                )?));
            Ok(())
        }

        fn visit_method_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodAccess<'de, 'de>,
        {
            self.$v
                .push(Content::Described(Described::from_method_access_borrowed(
                    access,
                    fill_contents_plain,
                )?));
            Ok(())
        }

        fn visit_method_arg<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodArgAccess<'de, 'b>,
        {
            self.$v.push(Content::MethodArg(MethodArg::from_access(
                access,
                fill_contents_plain,
            )?));
            Ok(())
        }

        fn visit_method_arg_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodArgAccess<'de, 'de>,
        {
            self.$v
                .push(Content::MethodArg(MethodArg::from_access_borrowed(
                    access,
                    fill_contents_plain,
                )?));
            Ok(())
        }

        fn visit_method_var<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodVarAccess<'de, 'b>,
        {
            self.$v.push(Content::MethodVar(MethodVar::from_access(
                access,
                fill_contents_plain,
            )?));
            Ok(())
        }

        fn visit_method_var_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodVarAccess<'de, 'de>,
        {
            self.$v
                .push(Content::MethodVar(MethodVar::from_access_borrowed(
                    access,
                    fill_contents_plain,
                )?));
            Ok(())
        }
    };
}

struct PlainVisitor<'env, 'de> {
    contents: &'env mut Vec<Content<'de>>,
}

impl<'de> Visitor<'de> for PlainVisitor<'_, 'de> {
    type Value = ();

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "an element")
    }

    fn visit_class<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'b>,
    {
        self.contents.push(Content::Class(Class::from_access(
            access,
            fill_contents_plain,
        )?));
        Ok(())
    }

    fn visit_class_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'de>,
    {
        self.contents
            .push(Content::Class(Class::from_access_borrowed(
                access,
                fill_contents_plain,
            )?));
        Ok(())
    }

    push_contents!(contents);
}

fn fill_contents_plain<'de, D>(
    contents: &mut Vec<Content<'de>>,
    mut deser: D,
) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    while deser.deserialize_any(PlainVisitor { contents })?.is_some() {}
    Ok(())
}

const CLASS_SPLIT: char = '$';

#[inline]
fn strip_parent(flat: &str) -> &str {
    flat.rsplit_once(CLASS_SPLIT)
        .map_or(flat, |(_, child)| child)
}

struct ChildClass<'a, A> {
    child_src: Option<&'a str>,
    access: A,
}

impl<'de, 'a, A> ClassAccess<'de, 'a> for ChildClass<'a, A>
where
    A: ClassAccess<'de, 'a>,
{
    type Error = A::Error;

    type ContentDeserializer = A::ContentDeserializer;

    #[inline]
    fn src(&self) -> &'a str {
        self.child_src
            .unwrap_or_else(|| strip_parent(self.access.src()))
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'a str> {
        self.access.dst().map(strip_parent)
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        self.access.content()
    }
}

struct BuildVisitor<'env, 'de> {
    classes: &'env mut Vec<ClassDesc<'de>>,
    index: &'env mut HashMap<SmolCowStr<'de>, usize>,
    top: &'env mut Vec<usize>,
    top_leftover: &'env mut Vec<Content<'de>>,
}

impl<'de> Visitor<'de> for BuildVisitor<'_, 'de> {
    type Value = ();

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "an element")
    }

    fn visit_class<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'b>,
    {
        let src = access.src().into();
        if let Some((parent, child)) = access.src().rsplit_once(CLASS_SPLIT) {
            let class = Class::from_access(
                ChildClass {
                    child_src: Some(child),
                    access,
                },
                fill_contents_plain,
            )?;
            let parent_index = *self
                .index
                .entry(SmolCowStr::Owned(parent.into()))
                .or_insert_with(|| {
                    self.classes.push(Default::default());
                    self.classes.len() - 1
                });
            let index = self.classes.len();
            self.classes.push(ClassDesc {
                class: Some(class),
                sibs: Vec::new(),
            });
            self.classes[parent_index].sibs.push(index);
            self.index.insert(SmolCowStr::Owned(src), index);
        } else {
            let class = Class::from_access(access, fill_contents_plain)?;
            let index = *self.index.entry(SmolCowStr::Owned(src)).or_insert_with(|| {
                self.classes.push(Default::default());
                self.classes.len() - 1
            });
            self.classes[index].class = Some(class);
            self.top.push(index);
        }
        Ok(())
    }

    fn visit_class_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: ClassAccess<'de, 'de>,
    {
        let src = access.src();
        if let Some((parent, child)) = access.src().rsplit_once(CLASS_SPLIT) {
            let class = Class::from_access(
                ChildClass {
                    child_src: Some(child),
                    access,
                },
                fill_contents_plain,
            )?;
            let parent_index = *self
                .index
                .entry(SmolCowStr::Borrowed(parent))
                .or_insert_with(|| {
                    self.classes.push(Default::default());
                    self.classes.len() - 1
                });
            let index = self.classes.len();
            self.classes.push(ClassDesc {
                class: Some(class),
                sibs: Vec::new(),
            });
            self.classes[parent_index].sibs.push(index);
            self.index.insert(SmolCowStr::Borrowed(src), index);
        } else {
            let class = Class::from_access(access, fill_contents_plain)?;
            let index = *self
                .index
                .entry(SmolCowStr::Borrowed(src))
                .or_insert_with(|| {
                    self.classes.push(Default::default());
                    self.classes.len() - 1
                });
            self.classes[index].class = Some(class);
            self.top.push(index);
        }
        Ok(())
    }

    push_contents!(top_leftover);
}

impl<'de> Index<'de> {
    fn deserialize_from<D>(mut deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut classes = Vec::new();
        let mut index = HashMap::new();
        let mut top = Vec::new();
        let mut top_leftover = Vec::new();

        while deserializer
            .deserialize_any(BuildVisitor {
                classes: &mut classes,
                index: &mut index,
                top: &mut top,
                top_leftover: &mut top_leftover,
            })?
            .is_some()
        {}

        Ok(Self {
            classes,
            top,
            top_leftover,
        })
    }

    #[inline]
    fn into_deserializer<D>(self, old: D) -> IndexedDeserializer<'de, D>
    where
        D: Deserializer<'de>,
    {
        IndexedDeserializer {
            classes: self.classes,
            top: self.top.into_iter(),
            top_leftover: self.top_leftover.into_iter(),
            old,
        }
    }
}

struct IndexedDeserializer<'de, D> {
    classes: Vec<ClassDesc<'de>>,
    top: vec::IntoIter<usize>,
    top_leftover: vec::IntoIter<Content<'de>>,

    old: D,
}

enum ControlFlow<T, V> {
    Yield(T),
    Break,
    Continue(V),
}

struct ClassWrap<'env, 'de, D> {
    class: &'env Class<'de>,
    sibs: Vec<usize>,
    content: Vec<Content<'de>>,

    classes: &'env mut [ClassDesc<'de>],
    old: &'env D,
}

impl<'env, 'de, D> ClassAccess<'de, 'env> for ClassWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer =
        IndexedDeserializerRef<'env, 'de, D, vec::IntoIter<usize>, vec::IntoIter<Content<'de>>>;

    #[inline]
    fn src(&self) -> &'env str {
        &self.class.src
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'env str> {
        self.class.dst.iter().map(|s| &**s)
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        IndexedDeserializerRef {
            classes: self.classes,
            top: self.sibs.into_iter(),
            content: self.content.into_iter(),
            old: self.old,
        }
    }
}

struct DescribedWrap<'env, 'de, D> {
    described: &'env Described<'de>,
    content: Vec<Content<'de>>,

    classes: &'env mut [ClassDesc<'de>],
    old: &'env D,
}

impl<'env, 'de, D> MethodAccess<'de, 'env> for DescribedWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer =
        IndexedDeserializerRef<'env, 'de, D, iter::Empty<usize>, vec::IntoIter<Content<'de>>>;

    #[inline]
    fn src(&self) -> &'env str {
        &self.described.src
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'env str> {
        self.described.dst.iter().map(|s| &**s)
    }

    #[inline]
    fn desc(&self) -> Option<&'env str> {
        self.described.desc.as_deref()
    }

    #[inline]
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.described
            .dst_desc
            .as_deref()
            .map(|s| s.iter().map(|s| &**s))
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        IndexedDeserializerRef {
            classes: self.classes,
            top: iter::empty(),
            content: self.content.into_iter(),
            old: self.old,
        }
    }
}

impl<'env, 'de, D> FieldAccess<'de, 'env> for DescribedWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer =
        IndexedDeserializerRef<'env, 'de, D, iter::Empty<usize>, vec::IntoIter<Content<'de>>>;

    #[inline]
    fn src(&self) -> &'env str {
        &self.described.src
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'env str> {
        self.described.dst.iter().map(|s| &**s)
    }

    #[inline]
    fn desc(&self) -> Option<&'env str> {
        self.described.desc.as_deref()
    }

    #[inline]
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.described
            .dst_desc
            .as_deref()
            .map(|s| s.iter().map(|s| &**s))
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        IndexedDeserializerRef {
            classes: self.classes,
            top: iter::empty(),
            content: self.content.into_iter(),
            old: self.old,
        }
    }
}

struct MethodArgWrap<'env, 'de, D> {
    method_arg: &'env MethodArg<'de>,
    content: Vec<Content<'de>>,

    classes: &'env mut [ClassDesc<'de>],
    old: &'env D,
}

impl<'env, 'de, D> MethodArgAccess<'de, 'env> for MethodArgWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer =
        IndexedDeserializerRef<'env, 'de, D, iter::Empty<usize>, vec::IntoIter<Content<'de>>>;

    #[inline]
    fn src(&self) -> Option<&'env str> {
        self.method_arg.src.as_deref()
    }

    #[inline]
    fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.method_arg
            .dst
            .as_deref()
            .map(|s| s.iter().map(|s| &**s))
    }

    #[inline]
    fn pos(&self) -> Option<usize> {
        self.method_arg.pos
    }

    #[inline]
    fn lv_index(&self) -> Option<usize> {
        self.method_arg.lv_index
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        IndexedDeserializerRef {
            classes: self.classes,
            top: iter::empty(),
            content: self.content.into_iter(),
            old: self.old,
        }
    }
}

struct MethodVarWrap<'env, 'de, D> {
    method_var: &'env MethodVar<'de>,
    content: Vec<Content<'de>>,

    classes: &'env mut [ClassDesc<'de>],
    old: &'env D,
}

impl<'env, 'de, D> MethodVarAccess<'de, 'env> for MethodVarWrap<'env, 'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    type ContentDeserializer =
        IndexedDeserializerRef<'env, 'de, D, iter::Empty<usize>, vec::IntoIter<Content<'de>>>;

    #[inline]
    fn src(&self) -> Option<&'env str> {
        self.method_var.src.as_deref()
    }

    #[inline]
    fn dst(&self) -> Option<impl Iterator<Item = &'env str>> {
        self.method_var
            .dst
            .as_deref()
            .map(|s| s.iter().map(|s| &**s))
    }

    #[inline]
    fn lv_index(&self) -> Option<usize> {
        self.method_var.lv_index
    }

    #[inline]
    fn lvt_row_index(&self) -> Option<usize> {
        self.method_var.lvt_row_index
    }

    #[inline]
    fn op_idx(&self) -> Option<(usize, Option<usize>)> {
        self.method_var.op_idx
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        IndexedDeserializerRef {
            classes: self.classes,
            top: iter::empty(),
            content: self.content.into_iter(),
            old: self.old,
        }
    }
}

impl<'de, D> IndexedDeserializer<'de, D> {
    #[inline]
    fn as_ref_deserializer(
        &mut self,
    ) -> IndexedDeserializerRef<
        '_,
        'de,
        D,
        &mut vec::IntoIter<usize>,
        &mut vec::IntoIter<Content<'de>>,
    > {
        IndexedDeserializerRef {
            classes: &mut self.classes,
            top: &mut self.top,
            content: &mut self.top_leftover,
            old: &self.old,
        }
    }
}

impl<'de, D> Deserializer<'de> for IndexedDeserializer<'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    const FLAT_CLASSES: bool = false;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.old.src_namespace()
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.old.dst_namespaces()
    }

    #[inline]
    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.as_ref_deserializer().deserialize_any(visitor)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (ll, lr) = self.top.size_hint();
        let (rl, rr) = self.top_leftover.size_hint();
        (ll + rl, lr.zip(rr).map(|(a, b)| a + b))
    }
}

struct IndexedDeserializerRef<'env, 'de, D, IterTop, IterContent> {
    classes: &'env mut [ClassDesc<'de>],
    top: IterTop,
    content: IterContent,

    old: &'env D,
}

impl<'de, D, IT, IC> IndexedDeserializerRef<'_, 'de, D, IT, IC>
where
    D: Deserializer<'de>,
    IT: Iterator<Item = usize>,
    IC: Iterator<Item = Content<'de>>,
{
    #[inline]
    fn deserialize_impl<V>(&mut self, visitor: V) -> Result<ControlFlow<V::Value, V>, D::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(i) = self.top.next() {
            let ClassDesc { class, sibs } = std::mem::take(&mut self.classes[i]);
            let Some(mut class) = class else {
                return Ok(ControlFlow::Continue(visitor));
            };
            let content = std::mem::take(&mut class.content);
            visitor
                .visit_class(ClassWrap {
                    class: &class,
                    sibs,
                    content: content.into_vec(),
                    classes: self.classes,
                    old: self.old,
                })
                .map(ControlFlow::Yield)
        } else if let Some(content) = self.content.next() {
            match content {
                Content::Comment(SmolCowStr::Owned(comment)) => visitor.visit_comment(&comment),
                Content::Comment(SmolCowStr::Borrowed(comment)) => {
                    visitor.visit_comment_borrowed(comment)
                }
                Content::Class(mut class) => {
                    let content = std::mem::take(&mut class.content);
                    visitor.visit_class(ClassWrap {
                        class: &class,
                        sibs: Vec::new(),
                        content: content.into_vec(),
                        classes: self.classes,
                        old: self.old,
                    })
                }
                Content::Described(mut described) => {
                    let content = std::mem::take(&mut described.content);
                    let access = DescribedWrap {
                        described: &described,
                        content: content.into_vec(),
                        classes: self.classes,
                        old: self.old,
                    };
                    match described.kind {
                        crate::translate::DescribedKind::Method => visitor.visit_method(access),
                        crate::translate::DescribedKind::Field => visitor.visit_field(access),
                    }
                }
                Content::MethodArg(mut method_arg) => {
                    let content = std::mem::take(&mut method_arg.content);
                    visitor.visit_method_arg(MethodArgWrap {
                        method_arg: &method_arg,
                        content: content.into_vec(),
                        classes: self.classes,
                        old: self.old,
                    })
                }
                Content::MethodVar(mut method_var) => {
                    let content = std::mem::take(&mut method_var.content);
                    visitor.visit_method_var(MethodVarWrap {
                        method_var: &method_var,
                        content: content.into_vec(),
                        classes: self.classes,
                        old: self.old,
                    })
                }
            }
            .map(ControlFlow::Yield)
        } else {
            Ok(ControlFlow::Break)
        }
    }
}

impl<'de, D, IT, IC> Deserializer<'de> for IndexedDeserializerRef<'_, 'de, D, IT, IC>
where
    D: Deserializer<'de>,
    IT: Iterator<Item = usize>,
    IC: Iterator<Item = Content<'de>>,
{
    type Error = D::Error;

    const FLAT_CLASSES: bool = false;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.old.src_namespace()
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.old.dst_namespaces()
    }

    fn deserialize_any<V>(&mut self, mut visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        loop {
            match self.deserialize_impl(visitor)? {
                ControlFlow::Yield(value) => return Ok(Some(value)),
                ControlFlow::Break => return Ok(None),
                ControlFlow::Continue(v) => visitor = v,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (ll, lr) = self.top.size_hint();
        let (rl, rr) = self.content.size_hint();
        (ll + rl, lr.zip(rr).map(|(a, b)| a + b))
    }
}

/// Nested deserializer that convert flattened mapping into tree-style ones.
#[derive(Debug)]
pub struct Nest<'de, D> {
    inner: NestedInner<'de, D>,
}

impl<D> Nest<'_, D> {
    pub(crate) fn new(inner: D) -> Self {
        Self {
            inner: NestedInner::Flat(inner),
        }
    }
}

enum NestedInner<'de, D> {
    Flat(D),
    Indexed(IndexedDeserializer<'de, D>),
    Vacant,
}

impl<'de, D> Deserializer<'de> for Nest<'de, D>
where
    D: Deserializer<'de>,
{
    type Error = D::Error;

    const FLAT_CLASSES: bool = false;

    #[inline]
    fn src_namespace(&self) -> &str {
        match &self.inner {
            NestedInner::Flat(d) => d.src_namespace(),
            NestedInner::Indexed(d) => d.src_namespace(),
            NestedInner::Vacant => unreachable!(),
        }
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        match &self.inner {
            NestedInner::Flat(d) => d.dst_namespaces(),
            NestedInner::Indexed(d) => d.old.dst_namespaces(),
            NestedInner::Vacant => unreachable!(),
        }
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &mut self.inner {
            NestedInner::Flat(d) => {
                if D::FLAT_CLASSES {
                    let index = Index::deserialize_from(d)?;
                    let NestedInner::Flat(d) =
                        std::mem::replace(&mut self.inner, NestedInner::Vacant)
                    else {
                        unreachable!()
                    };
                    self.inner = NestedInner::Indexed(index.into_deserializer(d));
                    let NestedInner::Indexed(indexed) = &mut self.inner else {
                        unreachable!();
                    };
                    indexed.deserialize_any(visitor)
                } else {
                    d.deserialize_any(visitor)
                }
            }
            NestedInner::Indexed(d) => d.deserialize_any(visitor),
            NestedInner::Vacant => unreachable!(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            NestedInner::Flat(_) => (0, None),
            NestedInner::Indexed(d) => d.size_hint(),
            NestedInner::Vacant => unreachable!(),
        }
    }
}

impl<D> Debug for NestedInner<'_, D>
where
    D: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Flat(arg0) => f.debug_tuple("Flat").field(arg0).finish(),
            Self::Indexed(_) => f.debug_tuple("Indexed").finish_non_exhaustive(),
            Self::Vacant => write!(f, "Vacant"),
        }
    }
}

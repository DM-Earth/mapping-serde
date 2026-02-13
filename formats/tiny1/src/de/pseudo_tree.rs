use std::marker::PhantomData;

use io_util::ColumnRead;
use mapping_serde::{
    Deserializer,
    de::{ClassAccess, FieldAccess, MethodAccess},
};
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::{DST_INLINE, Error, StreamDeserializer, StreamVisitor};

const EXTRA_INLINE: usize = DST_INLINE - 1;

/// A Tiny1 deserializer which sees Tiny1 file structure as 'pseudo trees', like Tiny2 without indents.
///
/// This can avoid indexing if you know the file satisfies following requirements before deserialization,
/// or it could miss up entries, which is an unexpected behavior:
///
/// - Fields and methods affiliated by one class must stay below the parent class.
/// - There shouldn't be any other classes inside section of those fields and methods.
#[derive(Debug)]
pub struct PseudoTreeDeserializer<'a, R> {
    stream: &'a mut StreamDeserializer<R>,
    class_buf: Option<ClassBuf>,

    ns_src: SmolStr,
    ns_dst: SmallVec<[SmolStr; DST_INLINE]>,

    aborted: bool,
}

impl<'a, R> PseudoTreeDeserializer<'a, R> {
    /// Creates a new deserializer from given stream.
    pub fn new(stream: &'a mut StreamDeserializer<R>) -> Self {
        Self {
            class_buf: None,
            ns_src: stream.src().into(),
            ns_dst: stream.dst().map(Into::into).collect(),
            aborted: false,
            stream,
        }
    }
}

#[derive(Debug)]
struct ClassBuf {
    fresh: bool,
    src: SmolStr,
    dst: Option<SmolStr>,
    dst_extra: SmallVec<[SmolStr; EXTRA_INLINE]>,
}

enum ControlFlow<T, V> {
    Yield(T),
    Continue(V),
    Break,
}

impl<'de, R> Deserializer<'de> for PseudoTreeDeserializer<'_, R>
where
    // this should actually be a unconstrained lifetime. but rust doesn't allow us to do this.
    R: ColumnRead<'de>,
{
    type Error = Error;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.stream.src()
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.stream.dst()
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: mapping_serde::de::Visitor<'de>,
    {
        struct ObtainClassBuf;

        impl StreamVisitor for ObtainClassBuf {
            type Value = ControlFlow<ClassBuf, ()>;

            fn visit_class_entry<'a, I>(
                self,
                name_a: &'a str,
                name_b: Option<&'a str>,
                extra_ns_names: I,
            ) -> Self::Value
            where
                I: IntoIterator<Item = &'a str>,
            {
                ControlFlow::Yield(ClassBuf {
                    src: name_a.into(),
                    dst: name_b.map(Into::into),
                    dst_extra: extra_ns_names.into_iter().map(Into::into).collect(),
                    fresh: false,
                })
            }

            #[inline]
            fn visit_field_entry<'a, I>(
                self,
                _parent_class_name_a: &'a str,
                _desc_a: &'a str,
                _name_a: &'a str,
                _name_b: Option<&'a str>,
                _extra_ns_names: I,
            ) -> Self::Value
            where
                I: IntoIterator<Item = &'a str>,
            {
                ControlFlow::Continue(())
            }

            #[inline]
            fn visit_method_entry<'a, I>(
                self,
                _parent_class_name_a: &'a str,
                _desc_a: &'a str,
                _name_a: &'a str,
                _name_b: Option<&'a str>,
                _extra_ns_names: I,
            ) -> Self::Value
            where
                I: IntoIterator<Item = &'a str>,
            {
                ControlFlow::Continue(())
            }
        }

        struct Class<'a, 'stream, R> {
            src: &'a str,
            dst: Option<&'a str>,
            dst_extra: &'a [SmolStr],
            deser: ContentDeserializer<'a, 'stream, R>,
        }

        impl<'de, 'a, 'stream, R> ClassAccess<'de, 'a> for Class<'a, 'stream, R>
        where
            R: ColumnRead<'de>,
        {
            type Error = Error;

            type ContentDeserializer = ContentDeserializer<'a, 'stream, R>;

            #[inline]
            fn src(&self) -> &'a str {
                self.src
            }

            fn dst(&self) -> impl Iterator<Item = &'a str> {
                self.dst
                    .into_iter()
                    .chain(self.dst_extra.iter().map(|s| &**s))
            }

            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                self.deser
            }
        }

        if self.aborted {
            return Ok(None);
        }

        if let Some(mut class) = self.class_buf.take()
            && class.fresh
        {
            class.fresh = false;
            let access = Class {
                src: &class.src,
                dst: class.dst.as_deref(),
                dst_extra: &class.dst_extra,
                deser: ContentDeserializer {
                    d: self,
                    parent: &class.src,
                    aborted: false,
                },
            };
            let ret = visitor.visit_class(access).map(Some);
            if self.class_buf.is_none() {
                self.class_buf = Some(class);
            }
            ret
        } else {
            loop {
                let ctl = self.stream.deserialize_next(ObtainClassBuf)?;
                match ctl {
                    Some(ControlFlow::Yield(class)) => {
                        let access = Class {
                            src: &class.src,
                            dst: class.dst.as_deref(),
                            dst_extra: &class.dst_extra,
                            deser: ContentDeserializer {
                                d: self,
                                parent: &class.src,
                                aborted: false,
                            },
                        };
                        let ret = visitor.visit_class(access).map(Some);
                        if self.class_buf.is_none() {
                            self.class_buf = Some(class);
                        }
                        return ret;
                    }
                    Some(ControlFlow::Continue(_)) => continue,
                    Some(ControlFlow::Break) | None => {
                        self.aborted = true;
                        return Ok(None);
                    }
                }
            }
        }
    }
}

struct EmptyDeserializer<'a> {
    ns_src: &'a str,
    ns_dst: &'a [SmolStr],
}

impl<'de> Deserializer<'de> for EmptyDeserializer<'_> {
    type Error = Error;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.ns_src
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.ns_dst.iter().map(|s| &**s)
    }

    #[inline]
    fn deserialize_any<V>(&mut self, _visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: mapping_serde::de::Visitor<'de>,
    {
        Ok(None)
    }
}

struct Described<'a, 's> {
    desc_a: &'a str,
    name_a: &'a str,
    name_b: Option<&'a str>,
    extra_dst_names: SmallVec<[&'a str; EXTRA_INLINE]>,

    ns_src: &'s str,
    ns_dst: &'s [SmolStr],
}

impl<'a, 's> FieldAccess<'_, 'a> for Described<'a, 's> {
    type Error = Error;

    type ContentDeserializer = EmptyDeserializer<'s>;

    #[inline]
    fn src(&self) -> &'a str {
        self.name_a
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'a str> {
        self.name_b
            .into_iter()
            .chain(self.extra_dst_names.iter().copied())
    }

    #[inline]
    fn desc(&self) -> Option<&'a str> {
        Some(self.desc_a)
    }

    #[inline]
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'a str>> {
        None::<std::iter::Empty<_>>
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        EmptyDeserializer {
            ns_src: self.ns_src,
            ns_dst: self.ns_dst,
        }
    }
}

impl<'a, 's> MethodAccess<'_, 'a> for Described<'a, 's> {
    type Error = Error;

    type ContentDeserializer = EmptyDeserializer<'s>;

    #[inline]
    fn src(&self) -> &'a str {
        self.name_a
    }

    #[inline]
    fn dst(&self) -> impl Iterator<Item = &'a str> {
        self.name_b
            .into_iter()
            .chain(self.extra_dst_names.iter().copied())
    }

    #[inline]
    fn desc(&self) -> Option<&'a str> {
        Some(self.desc_a)
    }

    #[inline]
    fn dst_desc(&self) -> Option<impl Iterator<Item = &'a str>> {
        None::<std::iter::Empty<_>>
    }

    #[inline]
    fn content(self) -> Self::ContentDeserializer {
        EmptyDeserializer {
            ns_src: self.ns_src,
            ns_dst: self.ns_dst,
        }
    }
}

struct ContentDeserializer<'a, 'stream, R> {
    d: &'a mut PseudoTreeDeserializer<'stream, R>,
    parent: &'a str,
    aborted: bool,
}

impl<'de, R> Deserializer<'de> for ContentDeserializer<'_, '_, R>
where
    R: ColumnRead<'de>,
{
    type Error = Error;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.d.stream.src()
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.d.stream.dst()
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: mapping_serde::de::Visitor<'de>,
    {
        struct Wrapper<'a, 'de, V> {
            class_buf: &'a mut Option<ClassBuf>,
            parent: &'a str,
            ns_src: &'a str,
            ns_dst: &'a [SmolStr],
            visitor: V,
            _ghost: PhantomData<&'de ()>,
        }

        impl<'de, V> StreamVisitor for Wrapper<'_, 'de, V>
        where
            V: mapping_serde::de::Visitor<'de>,
        {
            type Value = ControlFlow<Result<V::Value, Error>, V>;

            fn visit_class_entry<'a, I>(
                self,
                name_a: &'a str,
                name_b: Option<&'a str>,
                extra_ns_names: I,
            ) -> Self::Value
            where
                I: IntoIterator<Item = &'a str>,
            {
                *self.class_buf = Some(ClassBuf {
                    src: name_a.into(),
                    dst: name_b.map(Into::into),
                    dst_extra: extra_ns_names.into_iter().map(Into::into).collect(),
                    fresh: true,
                });
                ControlFlow::Break
            }

            fn visit_field_entry<'a, I>(
                self,
                parent_class_name_a: &'a str,
                desc_a: &'a str,
                name_a: &'a str,
                name_b: Option<&'a str>,
                extra_ns_names: I,
            ) -> Self::Value
            where
                I: IntoIterator<Item = &'a str>,
            {
                if parent_class_name_a == self.parent {
                    let access = Described {
                        ns_src: self.ns_src,
                        ns_dst: self.ns_dst,
                        desc_a,
                        name_a,
                        name_b,
                        extra_dst_names: extra_ns_names.into_iter().collect(),
                    };
                    ControlFlow::Yield(self.visitor.visit_field(access))
                } else {
                    ControlFlow::Continue(self.visitor)
                }
            }

            fn visit_method_entry<'a, I>(
                self,
                parent_class_name_a: &'a str,
                desc_a: &'a str,
                name_a: &'a str,
                name_b: Option<&'a str>,
                extra_ns_names: I,
            ) -> Self::Value
            where
                I: IntoIterator<Item = &'a str>,
            {
                if parent_class_name_a == self.parent {
                    let access = Described {
                        ns_src: self.ns_src,
                        ns_dst: self.ns_dst,
                        desc_a,
                        name_a,
                        name_b,
                        extra_dst_names: extra_ns_names.into_iter().collect(),
                    };
                    ControlFlow::Yield(self.visitor.visit_method(access))
                } else {
                    ControlFlow::Continue(self.visitor)
                }
            }
        }

        if self.aborted {
            return Ok(None);
        }

        let mut visitor = Some(visitor);
        loop {
            let wrapper = Wrapper {
                class_buf: &mut self.d.class_buf,
                parent: self.parent,
                ns_src: &self.d.ns_src,
                ns_dst: &self.d.ns_dst,
                visitor: visitor.take().unwrap(),
                _ghost: PhantomData,
            };
            let ctl = self.d.stream.deserialize_next(wrapper)?;
            match ctl {
                Some(ControlFlow::Yield(val)) => return val.map(Some),
                Some(ControlFlow::Continue(v)) => visitor = Some(v),
                Some(ControlFlow::Break) | None => {
                    self.aborted = true;
                    return Ok(None);
                }
            }
        }
    }
}

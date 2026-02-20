use std::{
    collections::{BTreeMap, btree_map},
    slice,
};

use io_util::ColumnRead;
use mapping_serde::{
    Deserializer, Serialize, Serializer,
    de::{ClassAccess, FieldAccess, MethodAccess},
};
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::{
    DST_INLINE, Error,
    de::{StreamDeserializer, stream},
};

/// Index of classes and contents of a Tiny1 mapping file.
///
/// This is especially helpful for deserializing Tiny1 in a tree fashion as it is a flattened format.
#[derive(Debug, Clone)]
pub struct Index {
    classes: BTreeMap<SmolStr, Class>,

    namespace_a: SmolStr,
    dst_namespaces: Box<[SmolStr]>,
}

#[derive(Debug, Clone)]
struct Class {
    parts: Option<ClassParts>,
    contents: Vec<Described>,
}

#[derive(Debug, Clone)]
struct ClassParts {
    // key as source name (name-a)
    // dst = b + extra. same for described
    dst_names: SmallVec<[SmolStr; DST_INLINE]>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum DescribedKind {
    Field,
    Method,
}

#[derive(Debug, Clone)]
struct Described {
    kind: DescribedKind,
    // parent: unneeded here
    desc_a: SmolStr,
    name_a: SmolStr,
    dst_names: SmallVec<[SmolStr; DST_INLINE]>,
}

struct BuildVisitor<'a> {
    index: &'a mut BTreeMap<SmolStr, Class>,
    cursor: &'a mut Option<(SmolStr, Class)>,
}

impl BuildVisitor<'_> {
    fn visit_described_entry<'a, I>(
        self,
        parent: &'a str,
        desc_a: &'a str,
        name_a: &'a str,
        name_b: Option<&'a str>,
        extra_ns_names: I,
        kind: DescribedKind,
    ) where
        I: IntoIterator<Item = &'a str>,
    {
        let described = Described {
            kind,
            desc_a: desc_a.into(),
            name_a: name_a.into(),
            dst_names: match name_b {
                Some(name) => std::iter::once(name)
                    .chain(extra_ns_names)
                    .map(Into::into)
                    .collect(),
                None => SmallVec::new(),
            },
        };

        if let Some((class_src, class)) = self.cursor
            && class_src == parent
        {
            class.contents.push(described);
        } else {
            self.index
                .entry(parent.into())
                .or_insert_with(|| Class {
                    parts: None,
                    contents: Vec::new(),
                })
                .contents
                .push(described);
        }
    }
}

impl stream::Visitor for BuildVisitor<'_> {
    type Value = ();

    fn visit_class_entry<'a, I>(
        self,
        name_a: &'a str,
        name_b: Option<&'a str>,
        extra_ns_names: I,
    ) -> Self::Value
    where
        I: IntoIterator<Item = &'a str>,
    {
        let parts = ClassParts {
            dst_names: match name_b {
                Some(name) => std::iter::once(name)
                    .chain(extra_ns_names)
                    .map(Into::into)
                    .collect(),
                None => SmallVec::new(),
            },
        };

        if let Some(class) = self.index.get_mut(name_a) {
            class.parts = Some(parts);
        } else {
            let class = Class {
                parts: Some(parts),
                contents: Vec::new(),
            };
            if let Some((old_name, old)) = self.cursor.replace((name_a.into(), class)) {
                self.index.insert(old_name, old);
            }
        }
    }

    #[inline]
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
        self.visit_described_entry(
            parent_class_name_a,
            desc_a,
            name_a,
            name_b,
            extra_ns_names,
            DescribedKind::Field,
        )
    }

    #[inline]
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
        self.visit_described_entry(
            parent_class_name_a,
            desc_a,
            name_a,
            name_b,
            extra_ns_names,
            DescribedKind::Method,
        )
    }
}

impl Index {
    /// Constructs index of a Tiny1 mapping file from its stream deserializer.
    #[allow(clippy::missing_errors_doc)]
    pub fn from_stream<'de, R>(stream: &mut StreamDeserializer<R>) -> Result<Self, Error>
    where
        R: ColumnRead<'de>,
    {
        let mut classes = BTreeMap::new();
        let mut cursor = None;

        loop {
            if stream
                .deserialize_next(BuildVisitor {
                    index: &mut classes,
                    cursor: &mut cursor,
                })?
                .is_none()
            {
                break;
            }
        }

        if let Some((k, class)) = cursor {
            classes.insert(k, class);
        }

        Ok(Self {
            classes,
            namespace_a: stream.src().into(),
            dst_namespaces: stream.dst().map(SmolStr::new).collect(),
        })
    }

    /// Returns a deserializer over the indexed entries.
    pub fn as_deserializer(&self) -> IndexDeserializer<'_> {
        IndexDeserializer {
            iter_classes: self.classes.iter(),
            namespace_a: &self.namespace_a,
            dst_namespaces: &self.dst_namespaces,
        }
    }
}

/// `mapping-serde` deserializer for `Index`.
///
/// Returned by [`Index::as_deserializer`].
#[derive(Debug)]
pub struct IndexDeserializer<'a> {
    iter_classes: btree_map::Iter<'a, SmolStr, Class>,
    namespace_a: &'a str,
    dst_namespaces: &'a [SmolStr],
}

impl<'a> Deserializer<'a> for IndexDeserializer<'a> {
    type Error = Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.namespace_a
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.dst_namespaces.iter().map(|s| &**s)
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: mapping_serde::de::Visitor<'a>,
    {
        let Some((src, class)) = self.iter_classes.find(|(_, c)| c.parts.is_some()) else {
            return Ok(None);
        };

        struct Wrapper<'a> {
            src: &'a str,
            item: &'a Class,
            namespace_a: &'a str,
            dst_namespaces: &'a [SmolStr],
        }

        impl<'a> ClassAccess<'a, 'a> for Wrapper<'a> {
            type Error = Error;

            type ContentDeserializer = ContentDeserializer<'a>;

            #[inline]
            fn src(&self) -> &'a str {
                self.src
            }

            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'a str> {
                self.item
                    .parts
                    .as_ref()
                    .unwrap()
                    .dst_names
                    .iter()
                    .map(|s| &**s)
            }

            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                ContentDeserializer {
                    iter_contents: self.item.contents.iter(),
                    namespace_a: self.namespace_a,
                    dst_namespaces: self.dst_namespaces,
                }
            }
        }

        visitor
            .visit_class_borrowed(Wrapper {
                src,
                item: class,
                namespace_a: self.namespace_a,
                dst_namespaces: self.dst_namespaces,
            })
            .map(Some)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter_classes.size_hint()
    }
}

struct ContentDeserializer<'a> {
    iter_contents: slice::Iter<'a, Described>,
    namespace_a: &'a str,
    dst_namespaces: &'a [SmolStr],
}

impl<'a> Deserializer<'a> for ContentDeserializer<'a> {
    type Error = Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.namespace_a
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.dst_namespaces.iter().map(|s| &**s)
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: mapping_serde::de::Visitor<'a>,
    {
        let Some(item) = self.iter_contents.next() else {
            return Ok(None);
        };

        struct Wrapper<'a> {
            item: &'a Described,
            namespace_a: &'a str,
            dst_namespaces: &'a [SmolStr],
        }

        impl<'a> FieldAccess<'a, 'a> for Wrapper<'a> {
            type Error = Error;

            type ContentDeserializer = EmptyDeserializer<'a>;

            #[inline]
            fn src(&self) -> &'a str {
                &self.item.name_a
            }

            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'a str> {
                self.item.dst_names.iter().map(|s| &**s)
            }

            #[inline]
            fn desc(&self) -> Option<&'a str> {
                Some(&self.item.desc_a)
            }

            #[inline]
            fn dst_desc(&self) -> Option<impl Iterator<Item = &'a str>> {
                None::<std::iter::Empty<_>>
            }

            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                EmptyDeserializer {
                    namespace_a: self.namespace_a,
                    dst_namespaces: self.dst_namespaces,
                }
            }
        }

        impl<'a> MethodAccess<'a, 'a> for Wrapper<'a> {
            type Error = Error;

            type ContentDeserializer = EmptyDeserializer<'a>;

            #[inline]
            fn src(&self) -> &'a str {
                &self.item.name_a
            }

            #[inline]
            fn dst(&self) -> impl Iterator<Item = &'a str> {
                self.item.dst_names.iter().map(|s| &**s)
            }

            #[inline]
            fn desc(&self) -> Option<&'a str> {
                Some(&self.item.desc_a)
            }

            #[inline]
            fn dst_desc(&self) -> Option<impl Iterator<Item = &'a str>> {
                None::<std::iter::Empty<_>>
            }

            #[inline]
            fn content(self) -> Self::ContentDeserializer {
                EmptyDeserializer {
                    namespace_a: self.namespace_a,
                    dst_namespaces: self.dst_namespaces,
                }
            }
        }

        let access = Wrapper {
            item,
            namespace_a: self.namespace_a,
            dst_namespaces: self.dst_namespaces,
        };

        match item.kind {
            DescribedKind::Field => visitor.visit_field_borrowed(access),
            DescribedKind::Method => visitor.visit_method_borrowed(access),
        }
        .map(Some)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter_contents.size_hint()
    }
}

struct EmptyDeserializer<'a> {
    namespace_a: &'a str,
    dst_namespaces: &'a [SmolStr],
}

impl<'a> Deserializer<'a> for EmptyDeserializer<'a> {
    type Error = Error;

    const FLAT_CLASSES: bool = true;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.namespace_a
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        self.dst_namespaces.iter().map(|s| &**s)
    }

    #[inline]
    fn deserialize_any<V>(&mut self, _visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: mapping_serde::de::Visitor<'a>,
    {
        Ok(None)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

impl Serialize for Index {
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        for (class_src, class) in &self.classes {
            let Some(parts) = &class.parts else {
                continue;
            };
            let mut content_ser = serializer.serialize_class(class_src, &parts.dst_names)?;
            for item in &class.contents {
                match item.kind {
                    DescribedKind::Field => {
                        let _ser = content_ser.serialize_field(
                            &item.name_a,
                            Some(&item.desc_a),
                            &item.dst_names,
                            None::<std::iter::Empty<&str>>,
                        )?;
                    }
                    DescribedKind::Method => {
                        let _ser = content_ser.serialize_method(
                            &item.name_a,
                            Some(&item.desc_a),
                            &item.dst_names,
                            None::<std::iter::Empty<&str>>,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}

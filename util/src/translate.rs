use std::{boxed::Box, vec::Vec};

use io_util::SmolCowStr;
use mapping_serde::de::{ClassAccess, FieldAccess, MethodAccess, MethodArgAccess, MethodVarAccess};
use smallvec::SmallVec;

macro_rules! push_contents {
    ($v:ident,$f:expr$(,)?) => {
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
                    access, $f,
                )?));
            Ok(())
        }

        fn visit_field_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::FieldAccess<'de, 'de>,
        {
            self.$v
                .push(Content::Described(Described::from_field_access_borrowed(
                    access, $f,
                )?));
            Ok(())
        }

        fn visit_method<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodAccess<'de, 'b>,
        {
            self.$v
                .push(Content::Described(Described::from_method_access(
                    access, $f,
                )?));
            Ok(())
        }

        fn visit_method_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodAccess<'de, 'de>,
        {
            self.$v
                .push(Content::Described(Described::from_method_access_borrowed(
                    access, $f,
                )?));
            Ok(())
        }

        fn visit_method_arg<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodArgAccess<'de, 'b>,
        {
            self.$v
                .push(Content::MethodArg(MethodArg::from_access(access, $f)?));
            Ok(())
        }

        fn visit_method_arg_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodArgAccess<'de, 'de>,
        {
            self.$v
                .push(Content::MethodArg(MethodArg::from_access_borrowed(
                    access, $f,
                )?));
            Ok(())
        }

        fn visit_method_var<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodVarAccess<'de, 'b>,
        {
            self.$v
                .push(Content::MethodVar(MethodVar::from_access(access, $f)?));
            Ok(())
        }

        fn visit_method_var_borrowed<A>(self, access: A) -> Result<Self::Value, A::Error>
        where
            A: mapping_serde::de::MethodVarAccess<'de, 'de>,
        {
            self.$v
                .push(Content::MethodVar(MethodVar::from_access_borrowed(
                    access, $f,
                )?));
            Ok(())
        }
    };
}

pub(crate) mod flat2tree;
pub(crate) mod tree2flat;

const DST_INLINE: usize = 2;

#[inline]
fn make_owned<'a>(val: &str) -> SmolCowStr<'a> {
    SmolCowStr::Owned(val.into())
}

#[inline]
fn make_borrow(val: &str) -> SmolCowStr<'_> {
    SmolCowStr::Borrowed(val)
}

struct Class<'a> {
    src: SmolCowStr<'a>,
    dst: SmallVec<[SmolCowStr<'a>; DST_INLINE]>,
    content: Box<[Content<'a>]>,
}

impl<'a> Class<'a> {
    fn from_access<'s, A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: ClassAccess<'a, 's>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            src: make_owned(access.src()),
            dst: access.dst().map(make_owned).collect(),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }

    fn from_access_borrowed<A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: ClassAccess<'a, 'a>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            src: make_borrow(access.src()),
            dst: access.dst().map(make_borrow).collect(),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DescribedKind {
    Method,
    Field,
}

struct Described<'a> {
    kind: DescribedKind,
    src: SmolCowStr<'a>,
    dst: SmallVec<[SmolCowStr<'a>; DST_INLINE]>,
    desc: Option<SmolCowStr<'a>>,
    dst_desc: Option<SmallVec<[SmolCowStr<'a>; DST_INLINE]>>,
    content: Box<[Content<'a>]>,
}

impl<'a> Described<'a> {
    fn from_method_access<'s, A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: MethodAccess<'a, 's>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            kind: DescribedKind::Method,
            src: make_owned(access.src()),
            dst: access.dst().map(make_owned).collect(),
            desc: access.desc().map(make_owned),
            dst_desc: access.dst_desc().map(|it| it.map(make_owned).collect()),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }

    fn from_method_access_borrowed<A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: MethodAccess<'a, 'a>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            kind: DescribedKind::Method,
            src: make_borrow(access.src()),
            dst: access.dst().map(make_borrow).collect(),
            desc: access.desc().map(make_borrow),
            dst_desc: access.dst_desc().map(|it| it.map(make_borrow).collect()),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }

    fn from_field_access<'s, A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: FieldAccess<'a, 's>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            kind: DescribedKind::Field,
            src: make_owned(access.src()),
            dst: access.dst().map(make_owned).collect(),
            desc: access.desc().map(make_owned),
            dst_desc: access.dst_desc().map(|it| it.map(make_owned).collect()),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }

    fn from_field_access_borrowed<A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: FieldAccess<'a, 'a>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            kind: DescribedKind::Field,
            src: make_borrow(access.src()),
            dst: access.dst().map(make_borrow).collect(),
            desc: access.desc().map(make_borrow),
            dst_desc: access.dst_desc().map(|it| it.map(make_borrow).collect()),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }
}

struct MethodArg<'a> {
    src: Option<SmolCowStr<'a>>,
    dst: Option<SmallVec<[SmolCowStr<'a>; 2]>>,
    pos: Option<usize>,
    lv_index: Option<usize>,
    content: Box<[Content<'a>]>,
}

impl<'a> MethodArg<'a> {
    fn from_access<'s, A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: MethodArgAccess<'a, 's>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            src: access.src().map(make_owned),
            dst: access.dst().map(|it| it.map(make_owned).collect()),
            pos: access.pos(),
            lv_index: access.lv_index(),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }

    fn from_access_borrowed<A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: MethodArgAccess<'a, 'a>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            src: access.src().map(make_borrow),
            dst: access.dst().map(|it| it.map(make_borrow).collect()),
            pos: access.pos(),
            lv_index: access.lv_index(),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }
}

struct MethodVar<'a> {
    src: Option<SmolCowStr<'a>>,
    dst: Option<SmallVec<[SmolCowStr<'a>; 2]>>,
    lv_index: Option<usize>,
    lvt_row_index: Option<usize>,
    op_idx: Option<(usize, Option<usize>)>,
    content: Box<[Content<'a>]>,
}

impl<'a> MethodVar<'a> {
    fn from_access<'s, A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: MethodVarAccess<'a, 's>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            src: access.src().map(make_owned),
            dst: access.dst().map(|it| it.map(make_owned).collect()),
            lv_index: access.lv_index(),
            lvt_row_index: access.lvt_row_index(),
            op_idx: access.op_idx(),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }

    fn from_access_borrowed<A, F>(access: A, f: F) -> Result<Self, A::Error>
    where
        A: MethodVarAccess<'a, 'a>,
        F: FnOnce(&mut Vec<Content<'a>>, A::ContentDeserializer) -> Result<(), A::Error>,
    {
        Ok(Self {
            src: access.src().map(make_borrow),
            dst: access.dst().map(|it| it.map(make_borrow).collect()),
            lv_index: access.lv_index(),
            lvt_row_index: access.lvt_row_index(),
            op_idx: access.op_idx(),
            content: {
                let mut v = Vec::new();
                f(&mut v, access.content())?;
                v.into_boxed_slice()
            },
        })
    }
}

enum Content<'a> {
    Comment(SmolCowStr<'a>),
    Class(Class<'a>),
    Described(Described<'a>),
    MethodArg(MethodArg<'a>),
    MethodVar(MethodVar<'a>),
}

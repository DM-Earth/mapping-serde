use core::ops::Range;

use alloc::boxed::Box;
use mapping_serde::{
    Deserialize, Serialize,
    de::{self, MethodAccess, MethodArgAccess, MethodVarAccess},
};
use smol_str::{SmolStr, ToSmolStr};

use crate::Element;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub src: SmolStr,
    pub dst: Box<[SmolStr]>,
    pub desc: Option<SmolStr>,
    pub dst_desc: Option<Box<[SmolStr]>>,
    pub content: Box<[Element]>,
}

impl Method {
    pub(crate) fn from_access<'de, 's, A>(access: A) -> Result<Self, A::Error>
    where
        A: MethodAccess<'de, 's>,
    {
        Ok(Self {
            src: access.src().to_smolstr(),
            dst: access.dst().map(|s| s.to_smolstr()).collect(),
            desc: access.desc().map(ToSmolStr::to_smolstr),
            dst_desc: access
                .dst_desc()
                .map(|dd| dd.map(|s| s.to_smolstr()).collect()),
            content: Box::deserialize(access.content())?.unwrap_or_default(),
        })
    }
}

impl Serialize for Method {
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: mapping_serde::Serializer,
    {
        let method_ser = serializer.serialize_method(
            &self.src,
            self.desc.as_deref(),
            self.dst.iter().map(|s| &**s),
            self.dst_desc.as_ref().map(|s| s.iter().map(|s| &**s)),
        )?;
        self.content.serialize(method_ser)
    }
}

impl<'de> Deserialize<'de> for Method {
    #[inline]
    fn deserialize<D>(mut deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: mapping_serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Method;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "a method")
            }

            #[inline]
            fn visit_method<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: MethodAccess<'de, 'b>,
            {
                Method::from_access(access)
            }
        }

        deserializer.deserialize_method(Visitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodArg {
    pub src: Option<SmolStr>,
    pub dst: Option<Box<[SmolStr]>>,
    pub pos: Option<usize>,
    pub lv_index: Option<usize>,
    pub content: Box<[Element]>,
}

impl MethodArg {
    pub(crate) fn from_access<'de, 's, A>(access: A) -> Result<Self, A::Error>
    where
        A: MethodArgAccess<'de, 's>,
    {
        Ok(Self {
            src: access.src().map(ToSmolStr::to_smolstr),
            dst: access.dst().map(|dd| dd.map(|s| s.to_smolstr()).collect()),
            pos: access.pos(),
            lv_index: access.lv_index(),
            content: Box::deserialize(access.content())?.unwrap_or_default(),
        })
    }
}

impl Serialize for MethodArg {
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: mapping_serde::Serializer,
    {
        let arg_ser = serializer.serialize_method_arg(
            self.src.as_deref(),
            self.dst.as_ref().map(|d| d.iter().map(|s| &**s)),
            self.pos,
            self.lv_index,
        )?;
        self.content.serialize(arg_ser)
    }
}

impl<'de> Deserialize<'de> for MethodArg {
    #[inline]
    fn deserialize<D>(mut deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = MethodArg;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "a method argument")
            }

            #[inline]
            fn visit_method_arg<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: MethodArgAccess<'de, 'b>,
            {
                MethodArg::from_access(access)
            }
        }

        deserializer.deserialize_method_arg(Visitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodVar {
    pub src: Option<SmolStr>,
    pub dst: Option<Box<[SmolStr]>>,
    pub lv_index: Option<usize>,
    pub lvt_row_index: Option<usize>,
    pub op_idx: Option<Range<usize>>,
    pub content: Box<[Element]>,
}

impl MethodVar {
    pub(crate) fn from_access<'de, 's, A>(access: A) -> Result<Self, A::Error>
    where
        A: MethodVarAccess<'de, 's>,
    {
        Ok(Self {
            src: access.src().map(ToSmolStr::to_smolstr),
            dst: access.dst().map(|dd| dd.map(|s| s.to_smolstr()).collect()),
            lvt_row_index: access.lvt_row_index(),
            lv_index: access.lv_index(),
            op_idx: access.op_idx(),
            content: Box::deserialize(access.content())?.unwrap_or_default(),
        })
    }
}

impl Serialize for MethodVar {
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: mapping_serde::Serializer,
    {
        let var_ser = serializer.serialize_method_var(
            self.src.as_deref(),
            self.dst.as_ref().map(|d| d.iter().map(|s| &**s)),
            self.lv_index,
            self.lvt_row_index,
            self.op_idx.clone(),
        )?;
        self.content.serialize(var_ser)
    }
}

impl<'de> Deserialize<'de> for MethodVar {
    #[inline]
    fn deserialize<D>(mut deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = MethodVar;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "a method variable")
            }

            #[inline]
            fn visit_method_var<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: MethodVarAccess<'de, 'b>,
            {
                MethodVar::from_access(access)
            }
        }

        deserializer.deserialize_method_var(Visitor)
    }
}

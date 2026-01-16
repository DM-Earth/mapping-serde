use alloc::boxed::Box;
use mapping_serde::{
    Deserialize, Serialize,
    de::{self, FieldAccess},
};
use smol_str::{SmolStr, ToSmolStr};

use crate::Element;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub src: SmolStr,
    pub dst: Box<[SmolStr]>,
    pub desc: Option<SmolStr>,
    pub dst_desc: Option<Box<[SmolStr]>>,
    pub content: Box<[Element]>,
}

impl Field {
    pub(crate) fn from_access<'de, 's, A>(access: A) -> Result<Self, A::Error>
    where
        A: FieldAccess<'de, 's>,
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

impl Serialize for Field {
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: mapping_serde::Serializer,
    {
        let field_ser = serializer.serialize_field(
            &self.src,
            self.desc.as_deref(),
            self.dst.iter().map(|s| &**s),
            self.dst_desc.as_ref().map(|s| s.iter().map(|s| &**s)),
        )?;
        self.content.serialize(field_ser)
    }
}

impl<'de> Deserialize<'de> for Field {
    #[inline]
    fn deserialize<D>(mut deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: mapping_serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Field;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "a field")
            }

            #[inline]
            fn visit_field<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: FieldAccess<'de, 'b>,
            {
                Field::from_access(access)
            }
        }

        deserializer.deserialize_field(Visitor)
    }
}

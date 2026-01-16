use alloc::boxed::Box;
use mapping_serde::{
    Deserialize, Serialize,
    de::{self, ClassAccess},
};
use smol_str::{SmolStr, ToSmolStr as _};

use crate::Element;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub src: SmolStr,
    pub dst: Box<[SmolStr]>,
    pub content: Box<[Element]>,
}

impl Class {
    pub(crate) fn from_access<'de, 's, A>(access: A) -> Result<Self, A::Error>
    where
        A: ClassAccess<'de, 's>,
    {
        Ok(Self {
            src: access.src().to_smolstr(),
            dst: access.dst().map(|s| s.to_smolstr()).collect(),
            content: Box::deserialize(access.content())?.unwrap_or_default(),
        })
    }
}

impl Serialize for Class {
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: mapping_serde::Serializer,
    {
        let class_ser = serializer.serialize_class(&self.src, self.dst.iter().map(|s| &**s))?;
        self.content.serialize(class_ser)
    }
}

impl<'de> Deserialize<'de> for Class {
    #[inline]
    fn deserialize<D>(mut deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: mapping_serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Class;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "a class")
            }

            #[inline]
            fn visit_class<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: ClassAccess<'de, 'b>,
            {
                Class::from_access(access)
            }
        }

        deserializer.deserialize_class(Visitor)
    }
}

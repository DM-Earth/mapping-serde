//! Generalized and static representation of Java deobfuscation mappings.
//!
//! See [`Element`] for the main entry.
//! See `java-mapping-serde`'s documentation for more details about the data structure.

#![no_std]
#![allow(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use mapping_serde::{Deserialize, Serialize, de};

mod class;
mod field;
mod method;

pub use class::Class;
pub use field::Field;
pub use method::{Method, MethodArg, MethodVar};

#[allow(clippy::exhaustive_enums)]
#[doc(alias = "Value")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    Class(Class),
    Field(Field),
    Method(Method),
    MethodArg(MethodArg),
    MethodVar(MethodVar),
    Comment(Box<str>),
}

impl Serialize for Element {
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: mapping_serde::Serializer,
    {
        match self {
            Self::Class(class) => class.serialize(serializer),
            Self::Field(field) => field.serialize(serializer),
            Self::Method(method) => method.serialize(serializer),
            Self::MethodArg(method_arg) => method_arg.serialize(serializer),
            Self::MethodVar(method_var) => method_var.serialize(serializer),
            Self::Comment(comment) => serializer.serialize_comment(comment),
        }
    }
}

impl<'de> Deserialize<'de> for Element {
    #[inline]
    fn deserialize<D>(mut deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: mapping_serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Element;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "any element")
            }

            #[inline]
            fn visit_class<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: de::ClassAccess<'de, 'b>,
            {
                Class::from_access(access).map(Element::Class)
            }

            #[inline]
            fn visit_field<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: de::FieldAccess<'de, 'b>,
            {
                Field::from_access(access).map(Element::Field)
            }

            #[inline]
            fn visit_method<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: de::MethodAccess<'de, 'b>,
            {
                Method::from_access(access).map(Element::Method)
            }

            #[inline]
            fn visit_method_arg<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: de::MethodArgAccess<'de, 'b>,
            {
                MethodArg::from_access(access).map(Element::MethodArg)
            }

            #[inline]
            fn visit_method_var<'b, A>(self, access: A) -> Result<Self::Value, A::Error>
            where
                A: de::MethodVarAccess<'de, 'b>,
            {
                MethodVar::from_access(access).map(Element::MethodVar)
            }

            fn visit_comment<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Element::Comment(value.into()))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

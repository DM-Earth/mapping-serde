use crate::{Deserialize, Deserializer};

impl<'de> Deserialize<'de> for () {
    const IS_CONDITIONAL: bool = false;

    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        drop(deserializer);
        Ok(Some(()))
    }
}

#[cfg(feature = "alloc")]
impl<'de, T> Deserialize<'de> for alloc::vec::Vec<T>
where
    T: Deserialize<'de>,
{
    const IS_CONDITIONAL: bool = false;

    fn deserialize<D>(mut deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use core::marker::PhantomData;

        struct __TypeCheck<T>(PhantomData<T>);

        impl<'de, T> __TypeCheck<T>
        where
            T: Deserialize<'de>,
        {
            const __VALID: () = assert!(
                T::IS_CONDITIONAL,
                "unconditional type is not supported by collections"
            );
        }

        let _: () = __TypeCheck::<T>::__VALID;

        // capactiy is unknown here, even with hints
        let mut vec = Self::new();
        while let Some(element) = T::deserialize(&mut deserializer)? {
            vec.push(element);
        }

        Ok(Some(vec))
    }
}

#[cfg(feature = "alloc")]
impl<'de, T> Deserialize<'de> for alloc::boxed::Box<[T]>
where
    T: Deserialize<'de>,
{
    const IS_CONDITIONAL: bool = alloc::vec::Vec::<T>::IS_CONDITIONAL;

    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Option<Self>, D::Error>
    where
        D: Deserializer<'de>,
    {
        alloc::vec::Vec::deserialize(deserializer).map(|o| o.map(alloc::vec::Vec::into_boxed_slice))
    }
}

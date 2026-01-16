use crate::{Serialize, Serializer};

impl Serialize for () {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        drop(serializer);
        Ok(())
    }
}

impl<T> Serialize for [T]
where
    T: Serialize,
{
    fn serialize<S>(&self, mut serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        for val in self {
            val.serialize(&mut serializer)?;
        }
        Ok(())
    }
}

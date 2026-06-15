use std::{
    fs::{File, ReadDir},
    io::BufReader,
    path::Path,
};

use io_util::{ColumnReadAdapter, IoReader};
use mapping_serde::Deserializer as _;
use mapping_serde_util::RefVisitor;

use crate::{Deserializer, Error};

type ColFile = ColumnReadAdapter<Box<IoReader<BufReader<File>>>>;

/// An Enigma deserializer that walks through all files in a directory and its subdirectories.
#[derive(Debug)]
pub struct DirDeserializer<'a> {
    dirs_stack: Vec<ReadDir>,
    current: Option<Deserializer<'a, ColFile>>,

    src: &'a str,
    dst: &'a str,
}

enum ControlFlow<T, V> {
    Break,
    Return(T),
    Continue(V),
}

impl<'a> DirDeserializer<'a> {
    /// Creates a new directory deserializer from given root directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the given root directory is invalid.
    pub fn new<P>(root: P, src: &'a str, dst: &'a str) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let dir = std::fs::read_dir(root)?;
        Ok(Self {
            dirs_stack: vec![dir],
            current: None,
            src,
            dst,
        })
    }

    fn deserialize_impl<'de, V>(&mut self, visitor: V) -> Result<ControlFlow<V::Value, V>, Error>
    where
        V: mapping_serde::de::Visitor<'de>,
    {
        if let Some(deser) = &mut self.current {
            let mut visitor = RefVisitor::new(visitor);
            if let Some(val) = deser.deserialize_any(&mut visitor)? {
                Ok(ControlFlow::Return(val))
            } else if let Some(visitor) = visitor.into_inner() {
                self.current = None;
                Ok(ControlFlow::Continue(visitor))
            } else {
                Ok(ControlFlow::Break)
            }
        } else {
            let Some(mut dir) = self.dirs_stack.pop() else {
                return Ok(ControlFlow::Break);
            };
            if let Some(entry) = dir.next().transpose()? {
                self.dirs_stack.push(dir);
                if entry.file_type()?.is_dir() {
                    self.dirs_stack.push(std::fs::read_dir(entry.path())?);
                    return Ok(ControlFlow::Continue(visitor));
                }
                let file = File::open(entry.path())?;
                self.current = Some(Deserializer::new(
                    self.src,
                    self.dst,
                    ColumnReadAdapter::new(Box::new(IoReader::new(BufReader::new(file)))),
                ));
            }
            Ok(ControlFlow::Continue(visitor))
        }
    }
}

impl<'de> mapping_serde::Deserializer<'de> for DirDeserializer<'_> {
    type Error = Error;

    const FLAT_CLASSES: bool = false;

    #[inline]
    fn src_namespace(&self) -> &str {
        self.src
    }

    #[inline]
    fn dst_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.dst)
    }

    fn deserialize_any<V>(&mut self, visitor: V) -> Result<Option<V::Value>, Self::Error>
    where
        V: mapping_serde::de::Visitor<'de>,
    {
        let mut visitor = visitor;
        loop {
            match self.deserialize_impl(visitor)? {
                ControlFlow::Break => return Ok(None),
                ControlFlow::Return(val) => return Ok(Some(val)),
                ControlFlow::Continue(v) => visitor = v,
            }
        }
    }
}

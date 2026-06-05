use std::{boxed::Box, vec::Vec};

use io_util::{ColumnReadAdapter, SliceReader};

use crate::DeserializerExt as _;

const FLAT_MAPPING: &str = include_str!("../../testset/flat.tiny");
const TREE_MAPPING: &str = include_str!("../../testset/tree.mappings");

#[test]
fn tree2flat() {
    let deserializer = mapping_serde_enigma::Deserializer::new(
        "source",
        "target",
        ColumnReadAdapter::new(Box::new(SliceReader::new(TREE_MAPPING.as_bytes()))),
    );

    let mut result = Vec::new();
    let serializer = mapping_serde_tiny2::Serializer::new(
        &mut result,
        "source",
        ["target"],
        0,
        [("name", Some("valid"))],
    )
    .unwrap();

    deserializer.flatten().pipe_into(serializer).unwrap();
    let result = str::from_utf8(&result).unwrap();
    assert_eq!(result.trim_end(), FLAT_MAPPING.trim_end());
}

#[test]
fn flat2tree() {
    let deserializer = mapping_serde_tiny2::Deserializer::new(ColumnReadAdapter::new(Box::new(
        SliceReader::new(FLAT_MAPPING.as_bytes()),
    )))
    .unwrap();

    let mut result = Vec::new();
    let serializer = mapping_serde_enigma::Serializer::new(&mut result);
    deserializer.nest().pipe_into(serializer).unwrap();
    let result = str::from_utf8(&result).unwrap();
    assert_eq!(result.trim_end(), TREE_MAPPING.trim_end());
}

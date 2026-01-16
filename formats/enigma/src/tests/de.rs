use crate::{
    de::Deserializer,
    io::{ColumnReadAdapter, SliceReader},
};

const VALID: &[u8] = include_bytes!("../../testset/valid.mappings");

#[test]
fn valid_from_bytes() {
    let mut reader = SliceReader::new(VALID);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new("src", "dst", col_reader);
    let elements = mapping_serde_element::deserialize_from(deserializer).unwrap();
}

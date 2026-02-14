use io_util::{ColumnReadAdapter, SliceReader};

use crate::Deserializer;

const TEST_MAPPING: &[u8] = include_bytes!("../testset/proguard.txt");

#[test]
#[ignore]
fn deserialize_from_slice() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new("mojmaps", "obfuscated", col_reader);
    let elements =
        mapping_serde_element::deserialize_from(deserializer).expect("failed to deserialize");
    dbg!(elements);
}

use io_util::{ColumnReadAdapter, SliceReader};

use crate::{Deserializer, tests::TEST_MAPPING};

#[test]
fn deserialize_from_slice() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new(col_reader).expect("failed to create deserializer");
    let elements =
        mapping_serde_element::deserialize_from(deserializer).expect("failed to deserialize");
    // validate_elements(&elements);
}

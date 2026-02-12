use std::io::Cursor;

use io_util::{ColumnRead, ColumnReadAdapter, IoReader, SliceReader};
use mapping_serde::{Deserializer as _, Serialize as _};

use crate::{Deserializer, Serializer, tests::TEST_MAPPING};

#[test]
fn deserialize_from_slice() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new(col_reader).expect("failed to create deserializer");
    validate_serde(deserializer);
}

#[test]
fn deserialize_from_io() {
    let mut reader = IoReader::new(Cursor::new(TEST_MAPPING));
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new(col_reader).expect("failed to create deserializer");
    validate_serde(deserializer);
}

fn validate_serde<'de, R>(mut deserializer: Deserializer<R>)
where
    R: ColumnRead<'de>,
{
    let elements =
        mapping_serde_element::deserialize_from(&mut deserializer).expect("failed to deserialize");

    let mut buf = Vec::new();
    let serializer = Serializer::new(
        Cursor::new(&mut buf),
        deserializer.src_namespace(),
        deserializer.dst_namespaces(),
        0,
        deserializer.properties(),
    )
    .expect("failed to create serializer");

    elements.serialize(serializer).expect("failed to serialize");

    let lhs = str::from_utf8(&buf).unwrap().trim_ascii_end();
    let rhs = str::from_utf8(TEST_MAPPING).unwrap().trim_ascii_end();
    for (line, (l, r)) in lhs.lines().zip(rhs.lines()).enumerate() {
        assert_eq!(
            l.trim_ascii_end(),
            r.trim_ascii_end(),
            "unmatch in line {}",
            line + 1
        );
    }
}

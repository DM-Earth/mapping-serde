use io_util::{ColumnRead, ColumnReadAdapter, SliceReader};

use crate::{Index, PseudoTreeDeserializer, StreamDeserializer};

const TEST_MAPPING: &[u8] = include_bytes!("../testset/tiny.tiny");

#[test]
#[ignore]
fn deserialize_from_slice_index() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let stream = StreamDeserializer::new(col_reader).expect("failed to create stream");
    validate_index(stream);
}

#[test]
#[ignore]
fn deserialize_from_slice_pseudo_tree() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let stream = StreamDeserializer::new(col_reader).expect("failed to create stream");
    validate_pseudo_tree(stream);
}

fn validate_index<'de, R>(mut stream: StreamDeserializer<R>)
where
    R: ColumnRead<'de>,
{
    let index = Index::from_stream(&mut stream).expect("failed to collect index");
    dbg!(index);
}

fn validate_pseudo_tree<'de, R>(mut stream: StreamDeserializer<R>)
where
    R: ColumnRead<'de>,
{
    let elements =
        mapping_serde_element::deserialize_from(PseudoTreeDeserializer::new(&mut stream))
            .expect("failed to deserialize");
    dbg!(elements);
}

use std::io::Cursor;

use io_util::{ColumnRead, ColumnReadAdapter, SliceReader};
use mapping_serde::{Deserializer, Serialize as _};

use crate::{Index, PseudoTreeDeserializer, Serializer, StreamDeserializer};

const TEST_MAPPING: &[u8] = include_bytes!("../testset/tiny.tiny");

#[test]
fn deserialize_from_slice_index() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let stream = StreamDeserializer::new(col_reader).expect("failed to create stream");
    validate_index(stream);
}

#[test]
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
    validate_serde(index.as_deserializer(), stream.properties())
}

fn validate_pseudo_tree<'de, R>(mut stream: StreamDeserializer<R>)
where
    R: ColumnRead<'de>,
{
    let props: Vec<_> = stream
        .properties()
        .map(|(a, b)| (a.to_owned(), b.map(ToOwned::to_owned)))
        .collect();
    validate_serde(PseudoTreeDeserializer::new(&mut stream), props)
}

fn validate_serde<'de, D, P, PI>(mut deserializer: D, props: P)
where
    D: Deserializer<'de>,
    P: IntoIterator<Item = (PI, Option<PI>)>,
    PI: AsRef<str>,
{
    let elements =
        mapping_serde_element::deserialize_from(&mut deserializer).expect("failed to deserialize");

    let mut buf = Vec::new();
    let serializer = Serializer::new(
        Cursor::new(&mut buf),
        deserializer.src_namespace(),
        deserializer.dst_namespaces(),
        props,
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

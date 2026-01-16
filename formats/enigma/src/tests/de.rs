use mapping_serde_element::Element;

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
    let mut elements = mapping_serde_element::deserialize_from(deserializer)
        .expect("failed to deserialize")
        .into_iter();

    let Element::Class(class1) = elements.next().expect("failed to get class_1") else {
        panic!("class_1 type mismatch")
    };
    assert_eq!(class1.src, "class_1");
    assert_eq!(class1.dst.len(), 1);
    assert_eq!(class1.dst[0], "class1Ns0Rename");

    let mut content = class1.content.iter();
    let Element::Field(field1) = content.next().expect("failed to get field_1") else {
        panic!("field_1 type mismatch")
    };
    assert_eq!(field1.src, "field_1");
    assert_eq!(field1.dst.len(), 1);
    assert_eq!(field1.dst[0], "field1Ns0Rename");
    assert_eq!(field1.desc.as_deref(), Some("I"));
}

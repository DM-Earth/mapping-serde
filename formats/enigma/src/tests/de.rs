use std::io::Cursor;

use io_util::{ColumnReadAdapter, IoReader, SliceReader};
use mapping_serde_element::Element;

use crate::{de::Deserializer, tests::TEST_MAPPING};

fn validate_elements(value: &[Element]) {
    let mut elements = value.iter();

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
    assert!(field1.dst_desc.is_none());

    let Element::Method(method1) = content.next().expect("failed to get method_1") else {
        panic!("method_1 type mismatch")
    };
    assert_eq!(method1.src, "method_1");
    assert_eq!(method1.dst.len(), 1);
    assert_eq!(method1.dst[0], "method1Ns0Rename");
    assert_eq!(method1.desc.as_deref(), Some("()I"));
    assert!(method1.dst_desc.is_none());
    assert_eq!(method1.content.len(), 1);
    let Element::MethodArg(arg1) = &method1.content.first().expect("failed get arg1") else {
        panic!("arg1 type mismatch");
    };
    assert_eq!(arg1.lv_index, Some(1));
    assert_eq!(arg1.pos, None);
    assert_eq!(&arg1.dst.as_ref().unwrap()[0], "param1Ns0Rename");

    assert!(matches!(
        elements.next().expect("failed to get glass_3"),
        Element::Class(_)
    ));
}

#[test]
fn deserialize_from_slice() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new("src", "dst", col_reader);
    let elements =
        mapping_serde_element::deserialize_from(deserializer).expect("failed to deserialize");
    validate_elements(&elements);
}

#[test]
fn deserialize_from_io() {
    let mut reader = IoReader::new(Cursor::new(TEST_MAPPING));
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new("src", "dst", col_reader);
    let elements =
        mapping_serde_element::deserialize_from(deserializer).expect("failed to deserialize");
    validate_elements(&elements);
}

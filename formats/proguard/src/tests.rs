use std::io::Cursor;

use io_util::{ColumnReadAdapter, IoReader, SliceReader};
use mapping_serde_element::Element;

use crate::Deserializer;

const TEST_MAPPING: &[u8] = include_bytes!("../testset/proguard.txt");

#[test]
fn deserialize_from_slice() {
    let mut reader = SliceReader::new(TEST_MAPPING);
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new("mojmaps", "obfuscated", col_reader);
    let elements =
        mapping_serde_element::deserialize_from(deserializer).expect("failed to deserialize");
    validate_elements(&elements);
}

#[test]
fn deserialize_from_io() {
    let mut reader = IoReader::new(Cursor::new(TEST_MAPPING));
    let col_reader = ColumnReadAdapter::new(&mut reader);
    let deserializer = Deserializer::new("mojmaps", "obfuscated", col_reader);
    let elements =
        mapping_serde_element::deserialize_from(deserializer).expect("failed to deserialize");
    validate_elements(&elements);
}

fn validate_elements(elements: &[Element]) {
    let mut elements = elements.iter();

    let Element::Class(class1) = elements.next().unwrap() else {
        panic!()
    };
    assert_eq!(class1.src, "class_1");
    assert_eq!(class1.dst[0], "class1Ns0Rename");
    let mut class1_contents = class1.content.iter();
    let Element::Field(field1) = class1_contents.next().unwrap() else {
        panic!()
    };
    assert_eq!(field1.src, "field_1");
    assert_eq!(field1.dst[0], "field1Ns0Rename");
    assert_eq!(field1.desc.as_deref().unwrap(), "I");
    let Element::Method(method1) = class1_contents.next().unwrap() else {
        panic!()
    };
    assert_eq!(method1.src, "method_1");
    assert_eq!(method1.dst[0], "method1Ns0Rename");
    assert_eq!(method1.desc.as_deref().unwrap(), "()I");
    let Element::Method(method2) = class1_contents.next().unwrap() else {
        panic!()
    };
    assert_eq!(method2.src, "method_2");
    assert_eq!(method2.dst[0], "method2Ns0Rename");
    assert_eq!(method2.desc.as_deref().unwrap(), "(ILcls;Z)V");

    let Element::Class(class2) = elements.next().unwrap() else {
        panic!()
    };
    assert_eq!(class2.src, "class_1$class_2");
    assert_eq!(class2.dst[0], "class1Ns0Rename$class2Ns0Rename");
}

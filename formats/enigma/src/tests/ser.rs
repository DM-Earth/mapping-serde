use std::iter;

use mapping_serde::Serializer as _;

use crate::{Serializer, tests::TEST_MAPPING};

#[test]
fn serialize() {
    let mut vec = Vec::new();
    let mut ser = Serializer::new(&mut vec);

    let mut class1 = ser.serialize_class("class_1", ["class1Ns0Rename"]).unwrap();
    class1
        .serialize_field(
            "field_1",
            Some("I"),
            ["field1Ns0Rename"],
            None::<iter::Empty<&str>>,
        )
        .unwrap();

    let mut method1 = class1
        .serialize_method(
            "method_1",
            Some("()I"),
            ["method1Ns0Rename"],
            None::<iter::Empty<&str>>,
        )
        .unwrap();
    method1
        .serialize_method_arg(None, Some(["param1Ns0Rename"]), None, Some(1))
        .unwrap();

    let mut class2 = class1
        .serialize_class("class_2", ["class2Ns0Rename"])
        .unwrap();
    class2
        .serialize_comment("This is a comment\nAnother line")
        .unwrap();
    class2
        .serialize_field(
            "field_2",
            Some("Lcls;"),
            ["field2Ns0Rename"],
            None::<iter::Empty<&str>>,
        )
        .unwrap();

    ser.serialize_class("class_3", ["package_3/class3Ns0Rename"])
        .unwrap();

    let lhs = str::from_utf8(&vec).unwrap().trim_ascii_end();
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

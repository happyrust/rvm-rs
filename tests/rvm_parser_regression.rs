use rvm_rs::{parse_rvm, ParseError, Store};

fn encode_tag(tag: &str) -> Vec<u8> {
    assert_eq!(tag.chars().count(), 4);
    let mut bytes = Vec::with_capacity(16);
    for ch in tag.chars() {
        bytes.extend_from_slice(&(ch as u32).to_be_bytes());
    }
    bytes
}

fn chunk(tag: &str, data: &[u8], next_offset: usize) -> Vec<u8> {
    let mut bytes = encode_tag(tag);
    bytes.extend_from_slice(&(next_offset as u32).to_be_bytes());
    bytes.extend_from_slice(data);
    bytes
}

fn geometry_payload(kind_id: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(84);
    data.extend_from_slice(&0u32.to_be_bytes()); // version
    data.extend_from_slice(&0u32.to_be_bytes()); // flags
    data.extend_from_slice(&kind_id.to_be_bytes());

    let matrix = [
        1.0f32, 0.0, 0.0, //
        0.0, 1.0, 0.0, //
        0.0, 0.0, 1.0, //
        0.0, 0.0, 0.0,
    ];
    for value in matrix {
        data.extend_from_slice(&value.to_bits().to_be_bytes());
    }

    let bbox = [-1.0f32, -1.0, -1.0, 1.0, 1.0, 1.0];
    for value in bbox {
        data.extend_from_slice(&value.to_bits().to_be_bytes());
    }

    assert_eq!(data.len(), 84);
    data
}

#[test]
fn parse_rvm_rejects_truncated_known_geometry_payload() {
    let prim_data = geometry_payload(8);
    let prim_next = 20 + prim_data.len();
    let input = chunk("PRIM", &prim_data, prim_next);

    let mut store = Store::new();
    let err = parse_rvm(&input, &mut store).expect_err("truncated cylinder should fail");
    assert!(matches!(err, ParseError::UnexpectedEof { .. }));
}

#[test]
fn parse_rvm_rejects_unknown_geometry_kind() {
    let prim_data = geometry_payload(99);
    let prim_next = 20 + prim_data.len();
    let input = chunk("PRIM", &prim_data, prim_next);

    let mut store = Store::new();
    let err = parse_rvm(&input, &mut store).expect_err("unknown geometry kind should fail");
    assert!(matches!(err, ParseError::InvalidGeometryKind { kind: 99 }));
}

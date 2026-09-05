use aic_dex::encode_minimal_dex;
use aic_ir::MinimalClass;
use std::collections::BTreeSet;

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

#[test]
fn generated_fixture_has_a_coherent_independent_structure() {
    let bytes =
        encode_minimal_dex(&MinimalClass::new("Ldev/aic/generated/Minimal;").unwrap()).unwrap();
    let b = bytes.as_slice();
    assert_eq!(&b[..8], b"dex\n035\0");
    assert_eq!(u32_at(b, 32) as usize, b.len());
    assert_eq!(u32_at(b, 36), 0x70);
    assert_eq!(u32_at(b, 40), 0x1234_5678);

    // proto_id.shorty_idx is a string index, while return_type_idx is a type
    // index. Resolve the shorty through string_ids and verify its actual value.
    let string_ids = u32_at(b, 60) as usize;
    let proto_ids = u32_at(b, 76) as usize;
    let shorty_idx = u32_at(b, proto_ids) as usize;
    let shorty_data = u32_at(b, string_ids + shorty_idx * 4) as usize;
    let mut shorty_text = shorty_data;
    while b[shorty_text] & 0x80 != 0 {
        shorty_text += 1;
    }
    shorty_text += 1;
    assert_eq!(&b[shorty_text..shorty_text + 2], b"V\0");

    let map = u32_at(b, 52) as usize;
    assert!(map < b.len() && map.is_multiple_of(4));
    let entries = u32_at(b, map) as usize;
    assert_eq!(entries, 10);
    let mut kinds = BTreeSet::new();
    let mut last = None;
    for i in 0..entries {
        let p = map + 4 + i * 12;
        let kind = u16::from_le_bytes(b[p..p + 2].try_into().unwrap());
        let size = u32_at(b, p + 4);
        let offset = u32_at(b, p + 8);
        assert!(kinds.insert(kind));
        assert!(size > 0);
        assert!((offset as usize) < b.len());
        if let Some(previous) = last {
            assert!(offset > previous);
        }
        last = Some(offset);
    }
    let class_defs = u32_at(b, 100) as usize;
    assert_eq!(u32_at(b, 96), 1);
    let class_data = u32_at(b, class_defs + 24) as usize;
    assert!(class_data > class_defs && class_data < map);
    assert_eq!(&b[class_data..class_data + 5], &[0, 0, 1, 0, 0]);
}

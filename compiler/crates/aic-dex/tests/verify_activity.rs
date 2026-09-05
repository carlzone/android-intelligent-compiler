use aic_dex::encode_activity_dex;
use aic_ir::parse_program;

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn activity_fixture_has_independently_readable_tables() {
    let program = parse_program(include_str!("../../../testdata/hello.aic")).unwrap();
    let bytes = encode_activity_dex(&program).unwrap();
    assert_eq!(&bytes[..8], b"dex\n035\0");
    assert_eq!(u32_at(&bytes, 32) as usize, bytes.len());
    assert_eq!(u32_at(&bytes, 96), 1);
    assert_eq!(u32_at(&bytes, 88), 10);

    let string_count = u32_at(&bytes, 56) as usize;
    let string_ids = u32_at(&bytes, 60) as usize;
    let mut strings = Vec::new();
    for index in 0..string_count {
        let mut at = u32_at(&bytes, string_ids + index * 4) as usize;
        while bytes[at] & 0x80 != 0 {
            at += 1;
        }
        at += 1;
        let end = bytes[at..].iter().position(|b| *b == 0).unwrap() + at;
        strings.push(std::str::from_utf8(&bytes[at..end]).unwrap());
    }
    for expected in [
        "Ldev/aic/generated/hello/MainActivity;",
        "Landroid/app/Activity;",
        "onCreate",
        "Hello from AndroidIntelligentCompiler",
    ] {
        assert!(strings.contains(&expected));
    }
}

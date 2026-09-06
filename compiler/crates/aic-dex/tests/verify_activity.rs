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
    assert!(u32_at(&bytes, 88) >= 10);

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

#[test]
fn m2_contains_runtime_function_and_not_precomputed_result() {
    let program = parse_program(include_str!("../../../testdata/compute.aic")).unwrap();
    let bytes = encode_activity_dex(&program).unwrap();
    let string_count = u32_at(&bytes, 56) as usize;
    let string_ids = u32_at(&bytes, 60) as usize;
    let mut strings = Vec::new();
    for index in 0..string_count {
        let mut at = u32_at(&bytes, string_ids + index * 4) as usize;
        while bytes[at] & 0x80 != 0 {
            at += 1;
        }
        at += 1;
        let end = bytes[at..].iter().position(|byte| *byte == 0).unwrap() + at;
        strings.push(std::str::from_utf8(&bytes[at..end]).unwrap());
    }
    assert!(strings.contains(&"aggregate"));
    assert!(strings.contains(&"Result: "));
    assert!(!strings.contains(&"Result: 36"));
    assert!(bytes.windows(2).any(|word| word[0] == 0x94)); // rem-int
    assert!(bytes.windows(2).any(|word| word == [0x29, 0x00])); // goto/16
}

#[test]
fn m2_lowering_is_driven_by_function_name_argument_and_body() {
    let source = include_str!("../../../testdata/compute.aic");
    let baseline = encode_activity_dex(&parse_program(source).unwrap()).unwrap();
    let renamed = source.replace("aggregate", "calculate");
    let renamed = encode_activity_dex(&parse_program(&renamed).unwrap()).unwrap();
    assert_ne!(baseline, renamed);
    let different_argument = source.replace("aggregate(12)", "aggregate(10)");
    let different_argument =
        encode_activity_dex(&parse_program(&different_argument).unwrap()).unwrap();
    assert_ne!(baseline, different_argument);
    let different_body = source.replace("total = total + 1", "total = total + 2");
    let different_body = encode_activity_dex(&parse_program(&different_body).unwrap()).unwrap();
    assert_ne!(baseline, different_body);
}

#[test]
fn m2_emits_multiple_user_functions_and_cross_calls() {
    let source = include_str!("../../../testdata/compute.aic").replace(
        "  fn aggregate(limit: i32) -> i32 {",
        "  fn twice(value: i32) -> i32 { return value * 2 }\n  fn aggregate(limit: i32) -> i32 {",
    ).replace("return total", "return twice(total)");
    let bytes = encode_activity_dex(&parse_program(&source).unwrap()).unwrap();
    let string_count = u32_at(&bytes, 56) as usize;
    let string_ids = u32_at(&bytes, 60) as usize;
    let mut strings = Vec::new();
    for index in 0..string_count {
        let mut at = u32_at(&bytes, string_ids + index * 4) as usize;
        while bytes[at] & 0x80 != 0 {
            at += 1;
        }
        at += 1;
        let end = bytes[at..].iter().position(|byte| *byte == 0).unwrap() + at;
        strings.push(std::str::from_utf8(&bytes[at..end]).unwrap());
    }
    assert!(strings.contains(&"aggregate"));
    assert!(strings.contains(&"twice"));
    assert!(bytes.windows(2).any(|word| word[0] == 0x71));
}

#[test]
fn on_create_is_ir_driven_and_executes_runtime_strings() {
    let source = r#"aic_version 0.1
app "Dynamic" package "dev.aic.generated.dynamic" {
  fn render(value: i32) -> string {
    return "Value: " + string(value)
  }
  activity MainActivity {
    on_create {
      let number: i32 = 7
      let caption: string = render(number)
      let container = android.linear_layout(orientation: vertical)
      let output = android.text_view(text: caption)
      android.add_view(parent: container, child: output)
      android.set_content_view(container)
    }
  }
}"#;
    let bytes = encode_activity_dex(&parse_program(source).unwrap()).unwrap();
    assert!(bytes.windows(2).any(|word| word[0] == 0x1a)); // const-string
    assert!(bytes.windows(2).any(|word| word[0] == 0x22)); // new-instance
    assert!(bytes.windows(2).any(|word| word[0] == 0x71)); // invoke-static
    assert!(bytes.windows(2).any(|word| word[0] == 0x11)); // return-object
    assert!(!bytes
        .windows("Value: 7".len())
        .any(|value| value == b"Value: 7"));

    let renamed = source
        .replace("number", "seed")
        .replace("caption", "label_text")
        .replace("container", "root_panel")
        .replace("output", "label_view");
    assert_ne!(
        bytes,
        encode_activity_dex(&parse_program(&renamed).unwrap()).unwrap()
    );
}

#[test]
fn m3_emits_activity_fields_and_click_dispatch() {
    let source = include_str!("../../../testdata/counter.aic");
    let program = parse_program(source).unwrap();
    let bytes = encode_activity_dex(&program).unwrap();
    assert_eq!(u32_at(&bytes, 80), 6);
    assert!(bytes.windows(7).any(|value| value == b"onClick"));
    assert!(bytes.windows(5).any(|value| value == b"count"));
    assert!(bytes.windows(2).any(|word| word[0] == 0x52)); // iget
    assert!(bytes.windows(2).any(|word| word[0] == 0x59)); // iput
    assert_eq!(bytes, encode_activity_dex(&program).unwrap());
}

#[test]
fn m3_calculator_contains_runtime_input_operations() {
    let bytes = encode_activity_dex(
        &parse_program(include_str!("../../../testdata/calculator.aic")).unwrap(),
    )
    .unwrap();
    for symbol in [
        "getText",
        "matches",
        "parseInt",
        "Invalid input",
        "Cannot divide by zero",
    ] {
        assert!(
            bytes
                .windows(symbol.len())
                .any(|value| value == symbol.as_bytes()),
            "missing {symbol}"
        );
    }
}

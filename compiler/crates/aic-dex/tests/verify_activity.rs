use aic_dex::encode_activity_dex;
use aic_ir::parse_program;
use aic_opt::{optimize, CompilerOptions};

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn assert_strict_method_id_order(bytes: &[u8]) {
    let count = usize::try_from(u32_at(bytes, 88)).unwrap();
    let offset = usize::try_from(u32_at(bytes, 92)).unwrap();
    let methods = (0..count)
        .map(|index| {
            let at = offset + index * 8;
            let class = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
            let proto = u16::from_le_bytes([bytes[at + 2], bytes[at + 3]]);
            let name = u32_at(bytes, at + 4);
            (class, name, proto)
        })
        .collect::<Vec<_>>();
    assert!(methods.windows(2).all(|pair| pair[0] < pair[1]));
}

fn dex_strings(bytes: &[u8]) -> Vec<String> {
    let count = u32_at(bytes, 56) as usize;
    let ids = u32_at(bytes, 60) as usize;
    (0..count)
        .map(|index| {
            let mut at = u32_at(bytes, ids + index * 4) as usize;
            while bytes[at] & 0x80 != 0 {
                at += 1;
            }
            at += 1;
            let end = bytes[at..].iter().position(|byte| *byte == 0).unwrap() + at;
            std::str::from_utf8(&bytes[at..end]).unwrap().to_owned()
        })
        .collect()
}

#[test]
fn m5_removes_unreachable_pool_entries_and_deduplicates_strings() {
    let source = include_str!("../../../testdata/m5-optimizer.aic");
    let original = parse_program(source).unwrap();
    let optimized = optimize(original.clone(), CompilerOptions::default());
    let o0 = encode_activity_dex(&original).unwrap();
    let o1 = encode_activity_dex(&optimized).unwrap();
    let strings = dex_strings(&o1);
    assert!(o1.len() < o0.len());
    assert!(!strings.iter().any(|value| value == "unused_message"
        || value == "unused helper payload"
        || value == "unreachable resource"));
    assert_eq!(
        strings
            .iter()
            .filter(|value| value.as_str() == "Count: ")
            .count(),
        1
    );
    assert_eq!(o1, encode_activity_dex(&optimized).unwrap());
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
    assert_eq!(u32_at(&bytes, 80), 8);
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

#[test]
fn m4_emits_deterministic_governed_persistence() {
    let program = parse_program(include_str!("../../../testdata/notes.aic")).unwrap();
    let bytes = encode_activity_dex(&program).unwrap();
    assert_eq!(bytes, encode_activity_dex(&program).unwrap());
    for symbol in [
        "Landroid/content/SharedPreferences;",
        "Landroid/database/sqlite/SQLiteDatabase;",
        "Landroid/database/sqlite/SQLiteStatement;",
        "CREATE TABLE IF NOT EXISTS notes",
        "INSERT INTO notes (title,body) VALUES (?,?)",
        "bindString",
        "close",
    ] {
        assert!(
            bytes
                .windows(symbol.len())
                .any(|value| value == symbol.as_bytes()),
            "missing {symbol}"
        );
    }
    let hello =
        encode_activity_dex(&parse_program(include_str!("../../../testdata/hello.aic")).unwrap())
            .unwrap();
    assert!(!hello
        .windows("SQLiteDatabase".len())
        .any(|value| value == b"SQLiteDatabase"));
    assert!(!hello
        .windows("SharedPreferences".len())
        .any(|value| value == b"SharedPreferences"));
}

#[test]
fn m9_emits_all_typed_navigation_extra_overloads() {
    let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    let bytes = encode_activity_dex(&program).unwrap();
    for symbol in ["putExtra", "item_id", "editable", "title"] {
        assert!(
            bytes
                .windows(symbol.len())
                .any(|value| value == symbol.as_bytes()),
            "missing {symbol}"
        );
    }
    assert_eq!(bytes, encode_activity_dex(&program).unwrap());
}

#[test]
fn m9_emits_bounded_dimensions_and_margins() {
    let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    let bytes = encode_activity_dex(&program).unwrap();
    assert!(bytes
        .windows("setMargins".len())
        .any(|value| value == b"setMargins"));
    assert_eq!(bytes, encode_activity_dex(&program).unwrap());
}

#[test]
fn m9_emits_common_input_modes_and_platform_spinner() {
    let mut program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    program.activity = program.activities[2].clone();
    program.activities = vec![program.activity.clone()];
    let bytes = encode_activity_dex(&program).unwrap();
    for symbol in [
        "Landroid/widget/Spinner;",
        "setInputType",
        "setDropDownViewResource",
        "Landroid/widget/SpinnerAdapter;",
        "setFitsSystemWindows",
    ] {
        assert!(
            bytes
                .windows(symbol.len())
                .any(|value| value == symbol.as_bytes()),
            "missing {symbol}"
        );
    }
    for forbidden in ["java/lang/reflect", "androidx/"] {
        assert!(!bytes
            .windows(forbidden.len())
            .any(|value| value == forbidden.as_bytes()));
    }
    assert_eq!(bytes, encode_activity_dex(&program).unwrap());
}

#[test]
fn m9_emits_state_collections_and_typed_selection_listeners_deterministically() {
    let source = include_str!("../../../testdata/m9-navigation.aic");
    let parsed = parse_program(source).unwrap();
    for activity_index in [0, 2] {
        let mut program = parsed.clone();
        program.activity = program.activities[activity_index].clone();
        program.activities = vec![program.activity.clone()];
        for options in [
            CompilerOptions {
                optimization_level: aic_opt::OptimizationLevel::None,
            },
            CompilerOptions::default(),
        ] {
            let optimized = optimize(program.clone(), options);
            let bytes = encode_activity_dex(&optimized).unwrap();
            assert_strict_method_id_order(&bytes);
            let expected = if activity_index == 0 {
                [
                    "[Ljava/lang/String;",
                    "OnItemClickListener",
                    "onItemClick",
                    "getItemAtPosition",
                    "destinations",
                ]
            } else {
                [
                    "[Ljava/lang/String;",
                    "OnItemSelectedListener",
                    "onItemSelected",
                    "onNothingSelected",
                    "selectionReady$choice",
                ]
            };
            for symbol in expected {
                assert!(
                    bytes
                        .windows(symbol.len())
                        .any(|window| window == symbol.as_bytes()),
                    "missing {symbol}"
                );
            }
            assert_eq!(bytes, encode_activity_dex(&optimized).unwrap());
        }
    }
}

#[test]
fn m9_emits_bounded_sp_text_sizes_deterministically() {
    let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    for options in [
        CompilerOptions {
            optimization_level: aic_opt::OptimizationLevel::None,
        },
        CompilerOptions::default(),
    ] {
        let optimized = optimize(program.clone(), options);
        let bytes = encode_activity_dex(&optimized).unwrap();
        assert_strict_method_id_order(&bytes);
        for symbol in ["setTextSize", "IF", "F"] {
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "missing {symbol}"
            );
        }
        for forbidden in ["java/lang/reflect", "scaledDensity"] {
            assert!(!bytes
                .windows(forbidden.len())
                .any(|window| window == forbidden.as_bytes()));
        }
        assert!(bytes
            .windows(4)
            .any(|window| window == 24_f32.to_bits().to_le_bytes()));
        assert_eq!(bytes, encode_activity_dex(&optimized).unwrap());
    }
}

#[test]
fn m9_emits_literal_color_properties_deterministically() {
    let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    for options in [
        CompilerOptions {
            optimization_level: aic_opt::OptimizationLevel::None,
        },
        CompilerOptions::default(),
    ] {
        let optimized = optimize(program.clone(), options);
        let bytes = encode_activity_dex(&optimized).unwrap();
        assert_strict_method_id_order(&bytes);
        for symbol in [
            "Landroid/graphics/Color;",
            "parseColor",
            "setTextColor",
            "setBackgroundColor",
            "#202124",
            "#E8F0FE",
        ] {
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "missing {symbol}"
            );
        }
        assert!(!bytes
            .windows("java/lang/reflect".len())
            .any(|window| window == b"java/lang/reflect"));
        assert_eq!(bytes, encode_activity_dex(&optimized).unwrap());
    }
}

#[test]
fn m9_emits_density_aware_minimum_touch_targets_deterministically() {
    let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    for options in [
        CompilerOptions {
            optimization_level: aic_opt::OptimizationLevel::None,
        },
        CompilerOptions::default(),
    ] {
        let optimized = optimize(program.clone(), options);
        let bytes = encode_activity_dex(&optimized).unwrap();
        assert_strict_method_id_order(&bytes);
        for symbol in [
            "getResources",
            "getDisplayMetrics",
            "densityDpi",
            "platform$minimumTouchTarget",
            "setMinimumWidth",
            "setMinimumHeight",
        ] {
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "missing {symbol}"
            );
        }
        assert!(!bytes
            .windows("java/lang/reflect".len())
            .any(|window| window == b"java/lang/reflect"));
        assert_eq!(bytes, encode_activity_dex(&optimized).unwrap());
    }
}

#[test]
fn m9_emits_input_labels_and_guarded_headings_deterministically() {
    let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    for options in [
        CompilerOptions {
            optimization_level: aic_opt::OptimizationLevel::None,
        },
        CompilerOptions::default(),
    ] {
        let optimized = optimize(program.clone(), options);
        let bytes = encode_activity_dex(&optimized).unwrap();
        assert_strict_method_id_order(&bytes);
        for symbol in [
            "generateViewId",
            "setId",
            "setLabelFor",
            "SDK_INT",
            "setAccessibilityHeading",
        ] {
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "missing {symbol}"
            );
        }
        assert_eq!(bytes, encode_activity_dex(&optimized).unwrap());
    }
}

#[test]
fn m9_emits_accessibility_semantics_deterministically() {
    let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    for options in [
        CompilerOptions {
            optimization_level: aic_opt::OptimizationLevel::None,
        },
        CompilerOptions::default(),
    ] {
        let optimized = optimize(program.clone(), options);
        let bytes = encode_activity_dex(&optimized).unwrap();
        assert!(optimized
            .activities
            .iter()
            .any(
                |activity| activity.on_create.iter().any(|statement| matches!(
                    statement.kind,
                    aic_ir::StatementKind::SetDecorative { .. }
                ))
            ));
        for symbol in ["setContentDescription", "setImportantForAccessibility"] {
            assert!(
                bytes
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes()),
                "missing {symbol}"
            );
        }
        assert_eq!(bytes, encode_activity_dex(&optimized).unwrap());
    }
}

#[test]
fn m9_emits_declared_ui_surface_deterministically() {
    let parsed = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
    for options in [
        CompilerOptions {
            optimization_level: aic_opt::OptimizationLevel::None,
        },
        CompilerOptions::default(),
    ] {
        let optimized = optimize(parsed.clone(), options);
        let dexes = optimized
            .activities
            .iter()
            .map(|activity| {
                let mut program = optimized.clone();
                program.activity = activity.clone();
                program.activities = vec![activity.clone()];
                encode_activity_dex(&program).unwrap()
            })
            .collect::<Vec<_>>();
        for symbol in [
            "Landroid/widget/LinearLayout;",
            "Landroid/widget/ScrollView;",
            "Landroid/widget/FrameLayout;",
            "Landroid/widget/TextView;",
            "Landroid/widget/Button;",
            "Landroid/widget/EditText;",
            "Landroid/widget/CheckBox;",
            "Landroid/widget/Switch;",
            "Landroid/widget/ProgressBar;",
            "Landroid/widget/ImageView;",
            "Landroid/widget/Toolbar;",
            "Landroid/widget/ListView;",
            "Landroid/widget/Spinner;",
            "setPadding",
            "setVisibility",
            "setEnabled",
            "setTextAlignment",
            "PopupMenu",
            "AlertDialog$Builder",
        ] {
            assert!(
                dexes.iter().any(|dex| dex
                    .windows(symbol.len())
                    .any(|window| window == symbol.as_bytes())),
                "missing {symbol}"
            );
        }
        for dex in &dexes {
            assert_strict_method_id_order(dex);
        }
    }
}

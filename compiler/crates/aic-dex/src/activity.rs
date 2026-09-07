use std::collections::{BTreeMap, BTreeSet};

use aic_ir::{
    Capability, Expression, ExpressionKind, Function, Program, Statement, StatementKind, Type,
    Value,
};

use crate::{
    encoding::{encode_mutf8, encode_uleb128, ByteWriter, DexError},
    integrity::{adler32, sha1},
};

const HEADER_SIZE: u32 = 0x70;
const NO_INDEX: u32 = u32::MAX;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Proto {
    ret: &'static str,
    params: Vec<&'static str>,
}
#[derive(Clone, Debug)]
struct Method {
    class: String,
    name: String,
    proto: Proto,
}
#[derive(Clone, Debug)]
struct Field {
    class: String,
    name: String,
    ty: String,
}
#[derive(Clone, Copy)]
struct Section {
    kind: u16,
    count: u32,
    offset: u32,
}

struct Pool {
    strings: Vec<String>,
    string_index: BTreeMap<String, u32>,
    types: Vec<u32>,
    type_index: BTreeMap<String, u16>,
    protos: Vec<Proto>,
    fields: Vec<Field>,
    methods: Vec<Method>,
}

impl Pool {
    #[allow(clippy::too_many_lines)]
    fn build(class: &str, program: &Program) -> Result<Self, DexError> {
        let functions = &program.functions;
        let runtime = !functions.is_empty()
            || !program.activity.state.is_empty()
            || !program.activity.on_click.is_empty()
            || !program.capabilities.is_empty();
        let key_value = program.capabilities.contains(&Capability::KeyValue);
        let sqlite = program.capabilities.contains(&Capability::Sqlite);
        let mut protos = vec![
            Proto {
                ret: "V",
                params: vec![],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/os/Bundle;"],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/content/Context;"],
            },
            Proto {
                ret: "V",
                params: vec!["I"],
            },
            Proto {
                ret: "V",
                params: vec!["Ljava/lang/CharSequence;"],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/view/View;"],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/view/View$OnClickListener;"],
            },
            Proto {
                ret: "V",
                params: vec!["I", "I", "F"],
            },
            Proto {
                ret: "V",
                params: vec!["Landroid/view/ViewGroup$LayoutParams;"],
            },
            Proto {
                ret: "I",
                params: vec!["Ljava/lang/String;"],
            },
        ];
        if runtime {
            protos.extend([
                Proto {
                    ret: "I",
                    params: vec!["I"],
                },
                Proto {
                    ret: "Ljava/lang/String;",
                    params: vec!["I"],
                },
                Proto {
                    ret: "Ljava/lang/StringBuilder;",
                    params: vec!["Ljava/lang/String;"],
                },
                Proto {
                    ret: "Ljava/lang/StringBuilder;",
                    params: vec!["I"],
                },
                Proto {
                    ret: "Ljava/lang/String;",
                    params: vec![],
                },
                Proto {
                    ret: "Z",
                    params: vec!["Ljava/lang/Object;"],
                },
                Proto {
                    ret: "Ljava/lang/String;",
                    params: vec!["Z"],
                },
                Proto {
                    ret: "Ljava/lang/CharSequence;",
                    params: vec![],
                },
                Proto {
                    ret: "Z",
                    params: vec!["Ljava/lang/String;"],
                },
                Proto {
                    ret: "I",
                    params: vec!["Ljava/lang/String;"],
                },
            ]);
            protos.extend(functions.iter().map(function_proto));
        }
        if key_value || sqlite {
            protos.extend([
                Proto {
                    ret: "Landroid/content/SharedPreferences;",
                    params: vec!["Ljava/lang/String;", "I"],
                },
                Proto {
                    ret: "I",
                    params: vec!["Ljava/lang/String;", "I"],
                },
                Proto {
                    ret: "Z",
                    params: vec!["Ljava/lang/String;", "Z"],
                },
                Proto {
                    ret: "Ljava/lang/String;",
                    params: vec!["Ljava/lang/String;", "Ljava/lang/String;"],
                },
                Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec![],
                },
                Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec!["Ljava/lang/String;", "I"],
                },
                Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec!["Ljava/lang/String;", "Z"],
                },
                Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec!["Ljava/lang/String;", "Ljava/lang/String;"],
                },
            ]);
        }
        if key_value || sqlite {
            protos.extend([
                Proto {
                    ret: "Landroid/database/sqlite/SQLiteDatabase;",
                    params: vec![
                        "Ljava/lang/String;",
                        "I",
                        "Landroid/database/sqlite/SQLiteDatabase$CursorFactory;",
                    ],
                },
                Proto {
                    ret: "V",
                    params: vec!["Ljava/lang/String;"],
                },
                Proto {
                    ret: "Landroid/database/sqlite/SQLiteStatement;",
                    params: vec!["Ljava/lang/String;"],
                },
                Proto {
                    ret: "V",
                    params: vec!["I", "Ljava/lang/String;"],
                },
                Proto {
                    ret: "J",
                    params: vec![],
                },
                Proto {
                    ret: "I",
                    params: vec![],
                },
            ]);
        }
        let mut fields = Vec::new();
        for state in &program.activity.state {
            fields.push(Field {
                class: class.into(),
                name: state.name.clone(),
                ty: type_descriptor(state.ty).into(),
            });
        }
        for statement in &program.activity.on_create {
            if let Some((id, ty)) = view_field(statement) {
                fields.push(Field {
                    class: class.into(),
                    name: format!("view${id}"),
                    ty: ty.into(),
                });
            }
        }
        if key_value {
            fields.push(Field {
                class: class.into(),
                name: "platform$preferences".into(),
                ty: "Landroid/content/SharedPreferences;".into(),
            });
        }
        if sqlite {
            fields.push(Field {
                class: class.into(),
                name: "platform$database".into(),
                ty: "Landroid/database/sqlite/SQLiteDatabase;".into(),
            });
        }
        let interactive = !program.activity.on_click.is_empty();
        let mut methods = vec![
            Method {
                class: class.to_owned(),
                name: "<init>".into(),
                proto: protos[0].clone(),
            },
            Method {
                class: class.to_owned(),
                name: "onCreate".into(),
                proto: protos[1].clone(),
            },
            Method {
                class: "Landroid/app/Activity;".into(),
                name: "<init>".into(),
                proto: protos[0].clone(),
            },
            Method {
                class: "Landroid/app/Activity;".into(),
                name: "onCreate".into(),
                proto: protos[1].clone(),
            },
            Method {
                class: "Landroid/app/Activity;".into(),
                name: "setContentView".into(),
                proto: protos[5].clone(),
            },
            Method {
                class: "Landroid/widget/LinearLayout;".into(),
                name: "<init>".into(),
                proto: protos[2].clone(),
            },
            Method {
                class: "Landroid/widget/LinearLayout;".into(),
                name: "setOrientation".into(),
                proto: protos[3].clone(),
            },
            Method {
                class: "Landroid/widget/TextView;".into(),
                name: "<init>".into(),
                proto: protos[2].clone(),
            },
            Method {
                class: "Landroid/widget/TextView;".into(),
                name: "setText".into(),
                proto: protos[4].clone(),
            },
            Method {
                class: "Landroid/widget/Button;".into(),
                name: "<init>".into(),
                proto: protos[2].clone(),
            },
            Method {
                class: "Landroid/widget/EditText;".into(),
                name: "<init>".into(),
                proto: protos[2].clone(),
            },
            Method {
                class: "Landroid/widget/TextView;".into(),
                name: "setHint".into(),
                proto: protos[4].clone(),
            },
            Method {
                class: "Landroid/widget/TextView;".into(),
                name: "setInputType".into(),
                proto: protos[3].clone(),
            },
            Method {
                class: "Landroid/widget/ScrollView;".into(),
                name: "<init>".into(),
                proto: protos[2].clone(),
            },
            Method {
                class: "Landroid/view/View;".into(),
                name: "setOnClickListener".into(),
                proto: protos[6].clone(),
            },
            Method {
                class: "Landroid/widget/LinearLayout$LayoutParams;".into(),
                name: "<init>".into(),
                proto: protos[7].clone(),
            },
            Method {
                class: "Landroid/view/View;".into(),
                name: "setLayoutParams".into(),
                proto: protos[8].clone(),
            },
            Method {
                class: "Landroid/graphics/Color;".into(),
                name: "parseColor".into(),
                proto: Proto {
                    ret: "I",
                    params: vec!["Ljava/lang/String;"],
                },
            },
            Method {
                class: "Landroid/widget/TextView;".into(),
                name: "setTextColor".into(),
                proto: protos[3].clone(),
            },
            Method {
                class: "Landroid/view/View;".into(),
                name: "setBackgroundColor".into(),
                proto: protos[3].clone(),
            },
            Method {
                class: "Landroid/view/ViewGroup;".into(),
                name: "addView".into(),
                proto: protos[5].clone(),
            },
        ];
        if interactive {
            methods.push(Method {
                class: class.into(),
                name: "onClick".into(),
                proto: protos[5].clone(),
            });
        }
        if runtime {
            methods.extend(functions.iter().map(|function| Method {
                class: class.into(),
                name: function.name.clone(),
                proto: function_proto(function),
            }));
            methods.extend([
                Method {
                    class: "Ljava/lang/String;".into(),
                    name: "valueOf".into(),
                    proto: Proto {
                        ret: "Ljava/lang/String;",
                        params: vec!["I"],
                    },
                },
                Method {
                    class: "Landroid/widget/TextView;".into(),
                    name: "getText".into(),
                    proto: Proto {
                        ret: "Ljava/lang/CharSequence;",
                        params: vec![],
                    },
                },
                Method {
                    class: "Ljava/lang/Object;".into(),
                    name: "toString".into(),
                    proto: Proto {
                        ret: "Ljava/lang/String;",
                        params: vec![],
                    },
                },
                Method {
                    class: "Ljava/lang/String;".into(),
                    name: "matches".into(),
                    proto: Proto {
                        ret: "Z",
                        params: vec!["Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Ljava/lang/Integer;".into(),
                    name: "parseInt".into(),
                    proto: Proto {
                        ret: "I",
                        params: vec!["Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Ljava/lang/String;".into(),
                    name: "valueOf".into(),
                    proto: Proto {
                        ret: "Ljava/lang/String;",
                        params: vec!["Z"],
                    },
                },
                Method {
                    class: "Ljava/lang/String;".into(),
                    name: "equals".into(),
                    proto: Proto {
                        ret: "Z",
                        params: vec!["Ljava/lang/Object;"],
                    },
                },
                Method {
                    class: "Ljava/lang/StringBuilder;".into(),
                    name: "<init>".into(),
                    proto: Proto {
                        ret: "V",
                        params: vec![],
                    },
                },
                Method {
                    class: "Ljava/lang/StringBuilder;".into(),
                    name: "append".into(),
                    proto: Proto {
                        ret: "Ljava/lang/StringBuilder;",
                        params: vec!["Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Ljava/lang/StringBuilder;".into(),
                    name: "append".into(),
                    proto: Proto {
                        ret: "Ljava/lang/StringBuilder;",
                        params: vec!["I"],
                    },
                },
                Method {
                    class: "Ljava/lang/StringBuilder;".into(),
                    name: "toString".into(),
                    proto: Proto {
                        ret: "Ljava/lang/String;",
                        params: vec![],
                    },
                },
            ]);
        }
        if key_value || sqlite {
            methods.extend([
                Method {
                    class: "Landroid/app/Activity;".into(),
                    name: "getSharedPreferences".into(),
                    proto: Proto {
                        ret: "Landroid/content/SharedPreferences;",
                        params: vec!["Ljava/lang/String;", "I"],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences;".into(),
                    name: "getInt".into(),
                    proto: Proto {
                        ret: "I",
                        params: vec!["Ljava/lang/String;", "I"],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences;".into(),
                    name: "getBoolean".into(),
                    proto: Proto {
                        ret: "Z",
                        params: vec!["Ljava/lang/String;", "Z"],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences;".into(),
                    name: "getString".into(),
                    proto: Proto {
                        ret: "Ljava/lang/String;",
                        params: vec!["Ljava/lang/String;", "Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences;".into(),
                    name: "edit".into(),
                    proto: Proto {
                        ret: "Landroid/content/SharedPreferences$Editor;",
                        params: vec![],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences$Editor;".into(),
                    name: "putInt".into(),
                    proto: Proto {
                        ret: "Landroid/content/SharedPreferences$Editor;",
                        params: vec!["Ljava/lang/String;", "I"],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences$Editor;".into(),
                    name: "putBoolean".into(),
                    proto: Proto {
                        ret: "Landroid/content/SharedPreferences$Editor;",
                        params: vec!["Ljava/lang/String;", "Z"],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences$Editor;".into(),
                    name: "putString".into(),
                    proto: Proto {
                        ret: "Landroid/content/SharedPreferences$Editor;",
                        params: vec!["Ljava/lang/String;", "Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Landroid/content/SharedPreferences$Editor;".into(),
                    name: "apply".into(),
                    proto: Proto {
                        ret: "V",
                        params: vec![],
                    },
                },
            ]);
        }
        if key_value || sqlite {
            methods.extend([
                Method {
                    class: "Landroid/app/Activity;".into(),
                    name: "openOrCreateDatabase".into(),
                    proto: Proto {
                        ret: "Landroid/database/sqlite/SQLiteDatabase;",
                        params: vec![
                            "Ljava/lang/String;",
                            "I",
                            "Landroid/database/sqlite/SQLiteDatabase$CursorFactory;",
                        ],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteDatabase;".into(),
                    name: "execSQL".into(),
                    proto: Proto {
                        ret: "V",
                        params: vec!["Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteDatabase;".into(),
                    name: "compileStatement".into(),
                    proto: Proto {
                        ret: "Landroid/database/sqlite/SQLiteStatement;",
                        params: vec!["Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteStatement;".into(),
                    name: "bindString".into(),
                    proto: Proto {
                        ret: "V",
                        params: vec!["I", "Ljava/lang/String;"],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteStatement;".into(),
                    name: "executeInsert".into(),
                    proto: Proto {
                        ret: "J",
                        params: vec![],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteStatement;".into(),
                    name: "simpleQueryForLong".into(),
                    proto: Proto {
                        ret: "J",
                        params: vec![],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteStatement;".into(),
                    name: "simpleQueryForString".into(),
                    proto: Proto {
                        ret: "Ljava/lang/String;",
                        params: vec![],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteStatement;".into(),
                    name: "executeUpdateDelete".into(),
                    proto: Proto {
                        ret: "I",
                        params: vec![],
                    },
                },
                Method {
                    class: "Landroid/database/sqlite/SQLiteStatement;".into(),
                    name: "close".into(),
                    proto: Proto {
                        ret: "V",
                        params: vec![],
                    },
                },
            ]);
        }
        let mut strings = BTreeSet::new();
        for value in [
            class,
            "Landroid/app/Activity;",
            "Landroid/content/Context;",
            "Landroid/os/Bundle;",
            "Landroid/view/View;",
            "Landroid/view/ViewGroup;",
            "Landroid/view/View$OnClickListener;",
            "Landroid/view/ViewGroup$LayoutParams;",
            "Landroid/widget/LinearLayout$LayoutParams;",
            "Landroid/graphics/Color;",
            "Landroid/widget/LinearLayout;",
            "Landroid/widget/TextView;",
            "Landroid/widget/Button;",
            "Landroid/widget/EditText;",
            "Landroid/widget/ScrollView;",
            "Ljava/lang/CharSequence;",
            "Ljava/lang/String;",
            "I",
            "V",
            "Z",
            "<init>",
            "onCreate",
            "setContentView",
            "setOrientation",
            "setText",
            "setHint",
            "setInputType",
            "addView",
            "onClick",
            "setOnClickListener",
            "setLayoutParams",
            "parseColor",
            "setTextColor",
            "setBackgroundColor",
            "F",
        ] {
            strings.insert(value.to_owned());
        }
        if runtime {
            for value in [
                "Ljava/lang/Object;",
                "Ljava/lang/String;",
                "Ljava/lang/StringBuilder;",
                "Ljava/lang/Integer;",
                "append",
                "equals",
                "toString",
                "valueOf",
                "getText",
                "matches",
                "parseInt",
                crate::lower::VALID_I32_PATTERN,
            ] {
                strings.insert(value.into());
            }
            strings.extend(functions.iter().map(|function| function.name.clone()));
        }
        if key_value || sqlite {
            for value in [
                "Landroid/content/SharedPreferences;",
                "Landroid/content/SharedPreferences$Editor;",
                "platform$preferences",
                "getSharedPreferences",
                "getInt",
                "getBoolean",
                "getString",
                "edit",
                "putInt",
                "putBoolean",
                "putString",
                "apply",
                "aic.preferences",
            ] {
                strings.insert(value.into());
            }
        }
        if key_value || sqlite {
            for value in [
                "Landroid/database/sqlite/SQLiteDatabase;",
                "Landroid/database/sqlite/SQLiteDatabase$CursorFactory;",
                "Landroid/database/sqlite/SQLiteStatement;",
                "J",
                "platform$database",
                "openOrCreateDatabase",
                "execSQL",
                "compileStatement",
                "bindString",
                "executeInsert",
                "simpleQueryForLong",
                "simpleQueryForString",
                "executeUpdateDelete",
                "close",
                "aic.db",
            ] {
                strings.insert(value.into());
            }
        }
        if let Some(database) = &program.database {
            for table in &database.tables {
                strings.insert(crate::lower::create_table_sql(table));
            }
        }
        collect_persistence_sql(&program.activity.on_create, program, &mut strings);
        for handler in &program.activity.on_click {
            collect_persistence_sql(&handler.body, program, &mut strings);
        }
        collect_program_strings(program, &mut strings);
        strings.extend(fields.iter().map(|field| field.name.clone()));
        for proto in &protos {
            strings.insert(shorty(proto));
        }
        let mut strings: Vec<_> = strings.into_iter().collect();
        strings.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        let string_index: BTreeMap<String, u32> = strings
            .iter()
            .enumerate()
            .map(|(i, s)| {
                Ok((
                    s.clone(),
                    u32::try_from(i).map_err(|_| DexError::IndexOverflow("string"))?,
                ))
            })
            .collect::<Result<_, DexError>>()?;
        let mut descriptors = BTreeSet::new();
        for method in &methods {
            descriptors.insert(method.class.clone());
            descriptors.insert(method.proto.ret.into());
            for p in &method.proto.params {
                descriptors.insert((*p).into());
            }
        }
        for field in &fields {
            descriptors.insert(field.class.clone());
            descriptors.insert(field.ty.clone());
        }
        let mut types: Vec<u32> = descriptors.iter().map(|d| string_index[d]).collect();
        types.sort_unstable();
        let type_index: BTreeMap<String, u16> = types
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let descriptor = strings[usize::try_from(*s).unwrap()].clone();
                Ok((
                    descriptor,
                    u16::try_from(i).map_err(|_| DexError::IndexOverflow("type"))?,
                ))
            })
            .collect::<Result<_, DexError>>()?;
        protos.sort_by_key(|p| {
            (
                type_index[p.ret],
                p.params.iter().map(|v| type_index[*v]).collect::<Vec<_>>(),
            )
        });
        protos.dedup();
        let proto_index = |p: &Proto| {
            protos
                .iter()
                .position(|candidate| candidate == p)
                .ok_or(DexError::InvalidInput("missing prototype"))
        };
        methods.sort_by_key(|m| {
            (
                type_index[&m.class],
                string_index[&m.name],
                proto_index(&m.proto).unwrap(),
            )
        });
        fields.sort_by_key(|f| {
            (
                type_index[&f.class],
                string_index[&f.name],
                type_index[&f.ty],
            )
        });
        Ok(Self {
            strings,
            string_index,
            types,
            type_index,
            protos,
            fields,
            methods,
        })
    }
    fn field(&self, class: &str, name: &str) -> Result<u16, DexError> {
        self.fields
            .iter()
            .position(|f| f.class == class && f.name == name)
            .ok_or(DexError::InvalidInput("missing field"))
            .and_then(|i| u16::try_from(i).map_err(|_| DexError::IndexOverflow("field")))
    }
    fn method(&self, class: &str, name: &str, proto: &Proto) -> Result<u16, DexError> {
        self.methods
            .iter()
            .position(|m| m.class == class && m.name == name && &m.proto == proto)
            .ok_or(DexError::InvalidInput("missing method"))
            .and_then(|i| u16::try_from(i).map_err(|_| DexError::IndexOverflow("method")))
    }
    fn proto_index(&self, proto: &Proto) -> Result<u16, DexError> {
        self.protos
            .iter()
            .position(|p| p == proto)
            .ok_or(DexError::InvalidInput("missing prototype"))
            .and_then(|i| u16::try_from(i).map_err(|_| DexError::IndexOverflow("prototype")))
    }
}
fn type_descriptor(ty: Type) -> &'static str {
    match ty {
        Type::I32 => "I",
        Type::Bool => "Z",
        Type::String => "Ljava/lang/String;",
    }
}
fn view_id(statement: &Statement) -> Option<&str> {
    match &statement.kind {
        StatementKind::LinearLayout { id, .. }
        | StatementKind::TextView { id, .. }
        | StatementKind::Button { id, .. }
        | StatementKind::EditText { id, .. }
        | StatementKind::TextInput { id, .. }
        | StatementKind::ScrollView { id } => Some(id),
        _ => None,
    }
}
fn view_field(statement: &Statement) -> Option<(&str, &'static str)> {
    match &statement.kind {
        StatementKind::LinearLayout { id, .. } => Some((id, "Landroid/widget/LinearLayout;")),
        StatementKind::TextView { id, .. } => Some((id, "Landroid/widget/TextView;")),
        StatementKind::Button { id, .. } => Some((id, "Landroid/widget/Button;")),
        StatementKind::EditText { id, .. } | StatementKind::TextInput { id, .. } => {
            Some((id, "Landroid/widget/EditText;"))
        }
        StatementKind::ScrollView { id } => Some((id, "Landroid/widget/ScrollView;")),
        _ => None,
    }
}

fn collect_program_strings(program: &Program, strings: &mut BTreeSet<String>) {
    for function in &program.functions {
        collect_statement_strings(&function.body, strings);
    }
    collect_statement_strings(&program.activity.on_create, strings);
    for handler in &program.activity.on_click {
        collect_statement_strings(&handler.body, strings);
    }
    for state in &program.activity.state {
        collect_expression_strings(&state.initial, strings);
    }
    for preference in &program.preferences {
        if let Value::String(value) = &preference.default {
            strings.insert(value.clone());
        }
    }
}

fn collect_statement_strings(statements: &[Statement], strings: &mut BTreeSet<String>) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Declare { value, .. }
            | StatementKind::Assign { value, .. }
            | StatementKind::Return(value)
            | StatementKind::TextView { text: value, .. }
            | StatementKind::Button { text: value, .. }
            | StatementKind::EditText { hint: value, .. }
            | StatementKind::TextInput { hint: value, .. }
            | StatementKind::SetText { text: value, .. } => {
                collect_expression_strings(value, strings);
            }
            StatementKind::SetTextColor { color, .. }
            | StatementKind::SetBackgroundColor { color, .. } => {
                strings.insert(color.clone());
            }
            StatementKind::PreferenceSet { key, value } => {
                strings.insert(key.clone());
                collect_expression_strings(value, strings);
            }
            StatementKind::DatabaseUpdate { table, id, values } => {
                strings.insert(table.clone());
                collect_expression_strings(id, strings);
                for (column, value) in values {
                    strings.insert(column.clone());
                    collect_expression_strings(value, strings);
                }
            }
            StatementKind::DatabaseDelete { table, id } => {
                strings.insert(table.clone());
                collect_expression_strings(id, strings);
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expression_strings(condition, strings);
                collect_statement_strings(then_body, strings);
                collect_statement_strings(else_body, strings);
            }
            StatementKind::For {
                start, end, body, ..
            } => {
                collect_expression_strings(start, strings);
                collect_expression_strings(end, strings);
                collect_statement_strings(body, strings);
            }
            StatementKind::LinearLayout { .. }
            | StatementKind::ScrollView { .. }
            | StatementKind::AddView { .. }
            | StatementKind::SetContentView { .. }
            | StatementKind::SetLayout { .. } => {}
        }
    }
}

fn collect_expression_strings(expression: &Expression, strings: &mut BTreeSet<String>) {
    match &expression.kind {
        ExpressionKind::Literal(Value::String(value)) => {
            strings.insert(value.clone());
        }
        ExpressionKind::Unary { value, .. } => collect_expression_strings(value, strings),
        ExpressionKind::Binary { left, right, .. } => {
            collect_expression_strings(left, strings);
            collect_expression_strings(right, strings);
        }
        ExpressionKind::Call { args, .. } => {
            for argument in args {
                collect_expression_strings(argument, strings);
            }
        }
        ExpressionKind::AndroidText { .. }
        | ExpressionKind::Literal(_)
        | ExpressionKind::Name(_) => {}
        ExpressionKind::PreferenceGet { key } => {
            strings.insert(key.clone());
        }
        ExpressionKind::DatabaseInsert { table, values } => {
            strings.insert(table.clone());
            for (column, value) in values {
                strings.insert(column.clone());
                collect_expression_strings(value, strings);
            }
        }
        ExpressionKind::DatabaseExists { table, id } => {
            strings.insert(table.clone());
            collect_expression_strings(id, strings);
        }
        ExpressionKind::DatabaseGet {
            table,
            id,
            column,
            default,
        } => {
            strings.insert(table.clone());
            strings.insert(column.clone());
            collect_expression_strings(id, strings);
            collect_expression_strings(default, strings);
        }
    }
}
fn primary(program: &Program, table: &str) -> String {
    program
        .database
        .as_ref()
        .and_then(|d| d.tables.iter().find(|t| t.name == table))
        .and_then(|t| t.columns.iter().find(|c| c.primary_key))
        .map_or_else(|| "id".into(), |c| c.name.clone())
}
#[allow(clippy::too_many_lines)]
fn collect_persistence_sql(
    statements: &[Statement],
    program: &Program,
    strings: &mut BTreeSet<String>,
) {
    fn expression(value: &Expression, program: &Program, strings: &mut BTreeSet<String>) {
        match &value.kind {
            ExpressionKind::DatabaseInsert { table, values } => {
                strings.insert(format!(
                    "INSERT INTO {table} ({}) VALUES ({})",
                    values
                        .iter()
                        .map(|v| v.0.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                    vec!["?"; values.len()].join(",")
                ));
                for (_, v) in values {
                    expression(v, program, strings);
                }
            }
            ExpressionKind::DatabaseExists { table, id } => {
                strings.insert(format!(
                    "SELECT COUNT(*) FROM {table} WHERE {} = ?",
                    primary(program, table)
                ));
                expression(id, program, strings);
            }
            ExpressionKind::DatabaseGet {
                table,
                id,
                column,
                default,
            } => {
                strings.insert(format!(
                    "SELECT COALESCE((SELECT {column} FROM {table} WHERE {} = ?), ?)",
                    primary(program, table)
                ));
                expression(id, program, strings);
                expression(default, program, strings);
            }
            ExpressionKind::Unary { value, .. } => expression(value, program, strings),
            ExpressionKind::Binary { left, right, .. } => {
                expression(left, program, strings);
                expression(right, program, strings);
            }
            ExpressionKind::Call { args, .. } => {
                for arg in args {
                    expression(arg, program, strings);
                }
            }
            _ => {}
        }
    }
    for statement in statements {
        match &statement.kind {
            StatementKind::DatabaseUpdate { table, id, values } => {
                strings.insert(format!(
                    "UPDATE {table} SET {} WHERE {} = ?",
                    values
                        .iter()
                        .map(|v| format!("{} = ?", v.0))
                        .collect::<Vec<_>>()
                        .join(","),
                    primary(program, table)
                ));
                expression(id, program, strings);
                for (_, value) in values {
                    expression(value, program, strings);
                }
            }
            StatementKind::DatabaseDelete { table, id } => {
                strings.insert(format!(
                    "DELETE FROM {table} WHERE {} = ?",
                    primary(program, table)
                ));
                expression(id, program, strings);
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                expression(condition, program, strings);
                collect_persistence_sql(then_body, program, strings);
                collect_persistence_sql(else_body, program, strings);
            }
            StatementKind::For {
                start, end, body, ..
            } => {
                expression(start, program, strings);
                expression(end, program, strings);
                collect_persistence_sql(body, program, strings);
            }
            StatementKind::Declare { value, .. }
            | StatementKind::Assign { value, .. }
            | StatementKind::Return(value)
            | StatementKind::TextView { text: value, .. }
            | StatementKind::Button { text: value, .. }
            | StatementKind::EditText { hint: value, .. }
            | StatementKind::TextInput { hint: value, .. }
            | StatementKind::SetText { text: value, .. }
            | StatementKind::PreferenceSet { value, .. } => expression(value, program, strings),
            _ => {}
        }
    }
}

fn descriptor(ty: Type) -> &'static str {
    match ty {
        Type::I32 | Type::Bool => "I",
        Type::String => "Ljava/lang/String;",
    }
}
fn function_proto(function: &Function) -> Proto {
    Proto {
        ret: descriptor(function.return_type),
        params: function
            .params
            .iter()
            .map(|parameter| descriptor(parameter.ty))
            .collect(),
    }
}

fn shorty(proto: &Proto) -> String {
    let mut s = String::from(shorty_type(proto.ret));
    for p in &proto.params {
        s.push_str(shorty_type(p));
    }
    s
}
fn shorty_type(value: &str) -> &str {
    if value.starts_with('L') || value.starts_with('[') {
        "L"
    } else {
        value
    }
}
fn u32_len(value: usize) -> Result<u32, DexError> {
    u32::try_from(value).map_err(|_| DexError::ArithmeticOverflow)
}
fn align4(value: u32) -> Result<u32, DexError> {
    value
        .checked_add(3)
        .map(|v| v & !3)
        .ok_or(DexError::ArithmeticOverflow)
}

/// Lowers a verified M1 program to a deterministic DEX 035 Activity.
///
/// # Errors
/// Returns a typed DEX error if the program is outside the M1 lowering shape or
/// if any index, offset, or encoded size cannot be represented safely.
pub fn encode_activity_dex(program: &Program) -> Result<Vec<u8>, DexError> {
    let class = format!(
        "L{}/{};",
        program.package.replace('.', "/"),
        program.activity.name
    );
    let pool = Pool::build(&class, program)?;
    encode(&pool, &class, program)
}

#[allow(clippy::too_many_lines)]
fn encode(pool: &Pool, class: &str, program: &Program) -> Result<Vec<u8>, DexError> {
    let functions = &program.functions;
    let runtime = !functions.is_empty()
        || !program.activity.state.is_empty()
        || !program.activity.on_click.is_empty();
    let p0 = Proto {
        ret: "V",
        params: vec![],
    };
    let pb = Proto {
        ret: "V",
        params: vec!["Landroid/os/Bundle;"],
    };
    let pc = Proto {
        ret: "V",
        params: vec!["Landroid/content/Context;"],
    };
    let pi = Proto {
        ret: "V",
        params: vec!["I"],
    };
    let ps = Proto {
        ret: "V",
        params: vec!["Ljava/lang/CharSequence;"],
    };
    let pv = Proto {
        ret: "V",
        params: vec!["Landroid/view/View;"],
    };
    let pss = Proto {
        ret: "Ljava/lang/StringBuilder;",
        params: vec!["Ljava/lang/String;"],
    };
    let pstring = Proto {
        ret: "Ljava/lang/String;",
        params: vec![],
    };
    let p_string_i = Proto {
        ret: "Ljava/lang/String;",
        params: vec!["I"],
    };
    let p_equals = Proto {
        ret: "Z",
        params: vec!["Ljava/lang/Object;"],
    };
    let p_string_bool = Proto {
        ret: "Ljava/lang/String;",
        params: vec!["Z"],
    };
    let own_init = pool.method(class, "<init>", &p0)?;
    let own_create = pool.method(class, "onCreate", &pb)?;
    let ctor = vec![
        0x1070,
        pool.method("Landroid/app/Activity;", "<init>", &p0)?,
        0,
        0x000e,
    ];
    let string_lowering = if runtime {
        Some(crate::StringLowering {
            value_of_i32: pool.method("Ljava/lang/String;", "valueOf", &p_string_i)?,
            value_of_bool: pool.method("Ljava/lang/String;", "valueOf", &p_string_bool)?,
            builder_type: pool.type_index["Ljava/lang/StringBuilder;"],
            builder_init: pool.method("Ljava/lang/StringBuilder;", "<init>", &p0)?,
            builder_append: pool.method("Ljava/lang/StringBuilder;", "append", &pss)?,
            builder_to_string: pool.method("Ljava/lang/StringBuilder;", "toString", &pstring)?,
            equals: pool.method("Ljava/lang/String;", "equals", &p_equals)?,
            text_view_get_text: pool.method(
                "Landroid/widget/TextView;",
                "getText",
                &Proto {
                    ret: "Ljava/lang/CharSequence;",
                    params: vec![],
                },
            )?,
            object_to_string: pool.method("Ljava/lang/Object;", "toString", &pstring)?,
            string_matches: pool.method(
                "Ljava/lang/String;",
                "matches",
                &Proto {
                    ret: "Z",
                    params: vec!["Ljava/lang/String;"],
                },
            )?,
            integer_parse_int: pool.method(
                "Ljava/lang/Integer;",
                "parseInt",
                &Proto {
                    ret: "I",
                    params: vec!["Ljava/lang/String;"],
                },
            )?,
        })
    } else {
        None
    };
    let lowered = functions
        .iter()
        .map(|function| {
            crate::lower_function_typed(
                function,
                &|name| {
                    let target = functions
                        .iter()
                        .find(|candidate| candidate.name == name)
                        .ok_or(DexError::InvalidInput("unresolved lowered function"))?;
                    Ok(crate::FunctionTarget {
                        method: pool.method(class, name, &function_proto(target))?,
                        params: target.params.iter().map(|parameter| parameter.ty).collect(),
                        result: target.return_type,
                    })
                },
                &|value| {
                    u16::try_from(pool.string_index[value])
                        .map_err(|_| DexError::IndexOverflow("string"))
                },
                string_lowering.ok_or(DexError::InvalidInput("missing runtime string methods"))?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let resolve_target = |name: &str| {
        let target = functions
            .iter()
            .find(|candidate| candidate.name == name)
            .ok_or(DexError::InvalidInput("unresolved lowered function"))?;
        Ok(crate::FunctionTarget {
            method: pool.method(class, name, &function_proto(target))?,
            params: target.params.iter().map(|parameter| parameter.ty).collect(),
            result: target.return_type,
        })
    };
    let resolve_string = |value: &str| {
        pool.string_index
            .get(value)
            .copied()
            .ok_or(DexError::InvalidInput("missing collected string literal"))
            .and_then(|index| u16::try_from(index).map_err(|_| DexError::IndexOverflow("string")))
    };
    let persistence_lowering = if program.capabilities.is_empty() {
        None
    } else {
        Some(crate::lower::PersistenceLowering {
            preferences_field: program
                .capabilities
                .contains(&Capability::KeyValue)
                .then(|| pool.field(class, "platform$preferences"))
                .transpose()?,
            database_field: program
                .capabilities
                .contains(&Capability::Sqlite)
                .then(|| pool.field(class, "platform$database"))
                .transpose()?,
            get_shared_preferences: pool.method(
                "Landroid/app/Activity;",
                "getSharedPreferences",
                &Proto {
                    ret: "Landroid/content/SharedPreferences;",
                    params: vec!["Ljava/lang/String;", "I"],
                },
            )?,
            pref_get_i32: pool.method(
                "Landroid/content/SharedPreferences;",
                "getInt",
                &Proto {
                    ret: "I",
                    params: vec!["Ljava/lang/String;", "I"],
                },
            )?,
            pref_get_bool: pool.method(
                "Landroid/content/SharedPreferences;",
                "getBoolean",
                &Proto {
                    ret: "Z",
                    params: vec!["Ljava/lang/String;", "Z"],
                },
            )?,
            pref_get_string: pool.method(
                "Landroid/content/SharedPreferences;",
                "getString",
                &Proto {
                    ret: "Ljava/lang/String;",
                    params: vec!["Ljava/lang/String;", "Ljava/lang/String;"],
                },
            )?,
            pref_edit: pool.method(
                "Landroid/content/SharedPreferences;",
                "edit",
                &Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec![],
                },
            )?,
            editor_put_i32: pool.method(
                "Landroid/content/SharedPreferences$Editor;",
                "putInt",
                &Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec!["Ljava/lang/String;", "I"],
                },
            )?,
            editor_put_bool: pool.method(
                "Landroid/content/SharedPreferences$Editor;",
                "putBoolean",
                &Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec!["Ljava/lang/String;", "Z"],
                },
            )?,
            editor_put_string: pool.method(
                "Landroid/content/SharedPreferences$Editor;",
                "putString",
                &Proto {
                    ret: "Landroid/content/SharedPreferences$Editor;",
                    params: vec!["Ljava/lang/String;", "Ljava/lang/String;"],
                },
            )?,
            editor_apply: pool.method(
                "Landroid/content/SharedPreferences$Editor;",
                "apply",
                &p0,
            )?,
            open_database: pool.method(
                "Landroid/app/Activity;",
                "openOrCreateDatabase",
                &Proto {
                    ret: "Landroid/database/sqlite/SQLiteDatabase;",
                    params: vec![
                        "Ljava/lang/String;",
                        "I",
                        "Landroid/database/sqlite/SQLiteDatabase$CursorFactory;",
                    ],
                },
            )?,
            database_exec_sql: pool.method(
                "Landroid/database/sqlite/SQLiteDatabase;",
                "execSQL",
                &Proto {
                    ret: "V",
                    params: vec!["Ljava/lang/String;"],
                },
            )?,
            database_compile: pool.method(
                "Landroid/database/sqlite/SQLiteDatabase;",
                "compileStatement",
                &Proto {
                    ret: "Landroid/database/sqlite/SQLiteStatement;",
                    params: vec!["Ljava/lang/String;"],
                },
            )?,
            statement_bind_string: pool.method(
                "Landroid/database/sqlite/SQLiteStatement;",
                "bindString",
                &Proto {
                    ret: "V",
                    params: vec!["I", "Ljava/lang/String;"],
                },
            )?,
            statement_execute_insert: pool.method(
                "Landroid/database/sqlite/SQLiteStatement;",
                "executeInsert",
                &Proto {
                    ret: "J",
                    params: vec![],
                },
            )?,
            statement_simple_long: pool.method(
                "Landroid/database/sqlite/SQLiteStatement;",
                "simpleQueryForLong",
                &Proto {
                    ret: "J",
                    params: vec![],
                },
            )?,
            statement_simple_string: pool.method(
                "Landroid/database/sqlite/SQLiteStatement;",
                "simpleQueryForString",
                &pstring,
            )?,
            statement_execute_update_delete: pool.method(
                "Landroid/database/sqlite/SQLiteStatement;",
                "executeUpdateDelete",
                &Proto {
                    ret: "I",
                    params: vec![],
                },
            )?,
            statement_close: pool.method(
                "Landroid/database/sqlite/SQLiteStatement;",
                "close",
                &p0,
            )?,
        })
    };
    let preferences = &program.preferences;
    let tables = program
        .database
        .as_ref()
        .map_or(&[][..], |database| database.tables.as_slice());
    let create = crate::lower_on_create(
        &program.activity.on_create,
        &resolve_target,
        &resolve_string,
        string_lowering,
        crate::UiLowering {
            activity_on_create: pool.method("Landroid/app/Activity;", "onCreate", &pb)?,
            linear_layout_type: pool.type_index["Landroid/widget/LinearLayout;"],
            linear_layout_init: pool.method("Landroid/widget/LinearLayout;", "<init>", &pc)?,
            linear_layout_orientation: pool.method(
                "Landroid/widget/LinearLayout;",
                "setOrientation",
                &pi,
            )?,
            text_view_type: pool.type_index["Landroid/widget/TextView;"],
            text_view_init: pool.method("Landroid/widget/TextView;", "<init>", &pc)?,
            text_view_set_text: pool.method("Landroid/widget/TextView;", "setText", &ps)?,
            button_type: pool.type_index["Landroid/widget/Button;"],
            button_init: pool.method("Landroid/widget/Button;", "<init>", &pc)?,
            edit_text_type: pool.type_index["Landroid/widget/EditText;"],
            edit_text_init: pool.method("Landroid/widget/EditText;", "<init>", &pc)?,
            edit_text_set_hint: pool.method("Landroid/widget/TextView;", "setHint", &ps)?,
            edit_text_set_input_type: pool.method(
                "Landroid/widget/TextView;",
                "setInputType",
                &pi,
            )?,
            scroll_view_type: pool.type_index["Landroid/widget/ScrollView;"],
            scroll_view_init: pool.method("Landroid/widget/ScrollView;", "<init>", &pc)?,
            set_on_click_listener: pool.method(
                "Landroid/view/View;",
                "setOnClickListener",
                &Proto {
                    ret: "V",
                    params: vec!["Landroid/view/View$OnClickListener;"],
                },
            )?,
            layout_params_type: pool.type_index["Landroid/widget/LinearLayout$LayoutParams;"],
            layout_params_init: pool.method(
                "Landroid/widget/LinearLayout$LayoutParams;",
                "<init>",
                &Proto {
                    ret: "V",
                    params: vec!["I", "I", "F"],
                },
            )?,
            set_layout_params: pool.method(
                "Landroid/view/View;",
                "setLayoutParams",
                &Proto {
                    ret: "V",
                    params: vec!["Landroid/view/ViewGroup$LayoutParams;"],
                },
            )?,
            color_parse: pool.method(
                "Landroid/graphics/Color;",
                "parseColor",
                &Proto {
                    ret: "I",
                    params: vec!["Ljava/lang/String;"],
                },
            )?,
            set_text_color: pool.method("Landroid/widget/TextView;", "setTextColor", &pi)?,
            set_background_color: pool.method("Landroid/view/View;", "setBackgroundColor", &pi)?,
            add_view: pool.method("Landroid/view/ViewGroup;", "addView", &pv)?,
            set_content_view: pool.method("Landroid/app/Activity;", "setContentView", &pv)?,
        },
        program
            .activity
            .state
            .iter()
            .map(|state| {
                Ok((
                    state.name.clone(),
                    (pool.field(class, &state.name)?, state.ty),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, DexError>>()?,
        program
            .activity
            .on_create
            .iter()
            .filter_map(|statement| view_id(statement).map(str::to_owned))
            .map(|id| Ok((id.clone(), pool.field(class, &format!("view${id}"))?)))
            .collect::<Result<BTreeMap<_, _>, DexError>>()?,
        &program.activity.state,
        persistence_lowering,
        preferences,
        tables,
    )?;
    let click = if program.activity.on_click.is_empty() {
        None
    } else {
        Some(crate::lower_on_click(
            &program.activity.on_click,
            &resolve_target,
            &resolve_string,
            string_lowering,
            crate::UiLowering {
                activity_on_create: pool.method("Landroid/app/Activity;", "onCreate", &pb)?,
                linear_layout_type: pool.type_index["Landroid/widget/LinearLayout;"],
                linear_layout_init: pool.method("Landroid/widget/LinearLayout;", "<init>", &pc)?,
                linear_layout_orientation: pool.method(
                    "Landroid/widget/LinearLayout;",
                    "setOrientation",
                    &pi,
                )?,
                text_view_type: pool.type_index["Landroid/widget/TextView;"],
                text_view_init: pool.method("Landroid/widget/TextView;", "<init>", &pc)?,
                text_view_set_text: pool.method("Landroid/widget/TextView;", "setText", &ps)?,
                button_type: pool.type_index["Landroid/widget/Button;"],
                button_init: pool.method("Landroid/widget/Button;", "<init>", &pc)?,
                edit_text_type: pool.type_index["Landroid/widget/EditText;"],
                edit_text_init: pool.method("Landroid/widget/EditText;", "<init>", &pc)?,
                edit_text_set_hint: pool.method("Landroid/widget/TextView;", "setHint", &ps)?,
                edit_text_set_input_type: pool.method(
                    "Landroid/widget/TextView;",
                    "setInputType",
                    &pi,
                )?,
                scroll_view_type: pool.type_index["Landroid/widget/ScrollView;"],
                scroll_view_init: pool.method("Landroid/widget/ScrollView;", "<init>", &pc)?,
                set_on_click_listener: pool.method(
                    "Landroid/view/View;",
                    "setOnClickListener",
                    &Proto {
                        ret: "V",
                        params: vec!["Landroid/view/View$OnClickListener;"],
                    },
                )?,
                layout_params_type: pool.type_index["Landroid/widget/LinearLayout$LayoutParams;"],
                layout_params_init: pool.method(
                    "Landroid/widget/LinearLayout$LayoutParams;",
                    "<init>",
                    &Proto {
                        ret: "V",
                        params: vec!["I", "I", "F"],
                    },
                )?,
                set_layout_params: pool.method(
                    "Landroid/view/View;",
                    "setLayoutParams",
                    &Proto {
                        ret: "V",
                        params: vec!["Landroid/view/ViewGroup$LayoutParams;"],
                    },
                )?,
                color_parse: pool.method(
                    "Landroid/graphics/Color;",
                    "parseColor",
                    &Proto {
                        ret: "I",
                        params: vec!["Ljava/lang/String;"],
                    },
                )?,
                set_text_color: pool.method("Landroid/widget/TextView;", "setTextColor", &pi)?,
                set_background_color: pool.method(
                    "Landroid/view/View;",
                    "setBackgroundColor",
                    &pi,
                )?,
                add_view: pool.method("Landroid/view/ViewGroup;", "addView", &pv)?,
                set_content_view: pool.method("Landroid/app/Activity;", "setContentView", &pv)?,
            },
            program
                .activity
                .state
                .iter()
                .map(|state| {
                    Ok((
                        state.name.clone(),
                        (pool.field(class, &state.name)?, state.ty),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, DexError>>()?,
            program
                .activity
                .on_create
                .iter()
                .filter_map(|statement| view_id(statement).map(str::to_owned))
                .map(|id| Ok((id.clone(), pool.field(class, &format!("view${id}"))?)))
                .collect::<Result<BTreeMap<_, _>, DexError>>()?,
            persistence_lowering,
            preferences,
            tables,
        )?)
    };

    let string_ids = HEADER_SIZE;
    let type_ids = string_ids + u32_len(pool.strings.len())? * 4;
    let proto_ids = type_ids + u32_len(pool.types.len())? * 4;
    let field_ids = proto_ids + u32_len(pool.protos.len())? * 12;
    let method_ids = field_ids + u32_len(pool.fields.len())? * 8;
    let class_defs = method_ids + u32_len(pool.methods.len())? * 8;
    let data = class_defs + 32;
    let mut cursor = data;
    let mut type_lists = Vec::new();
    for proto in &pool.protos {
        if !proto.params.is_empty() {
            cursor = align4(cursor)?;
            let off = cursor;
            cursor += 4 + u32_len(proto.params.len())? * 2;
            if proto.params.len() % 2 != 0 {
                cursor += 2;
            }
            type_lists.push((proto.clone(), off));
        }
    }
    let interfaces_off = if program.activity.on_click.is_empty() {
        0
    } else {
        cursor = align4(cursor)?;
        let offset = cursor;
        cursor += 8;
        offset
    };
    let string_data = cursor;
    let mut string_offsets = Vec::new();
    for value in &pool.strings {
        string_offsets.push(cursor);
        cursor += u32_len(
            encode_uleb128(u32_len(value.encode_utf16().count())?).len()
                + encode_mutf8(value).len(),
        )?;
    }
    cursor = align4(cursor)?;
    let ctor_code = cursor;
    cursor += 16 + u32_len(ctor.len())? * 2;
    cursor = align4(cursor)?;
    let create_code = cursor;
    cursor += 16 + u32_len(create.code.len())? * 2;
    let click_code = if let Some(method) = &click {
        cursor = align4(cursor)?;
        let offset = cursor;
        cursor += 16 + u32_len(method.code.len())? * 2;
        Some(offset)
    } else {
        None
    };
    let mut function_code_offsets = Vec::new();
    for method in &lowered {
        cursor = align4(cursor)?;
        function_code_offsets.push(cursor);
        cursor += 16 + u32_len(method.code.len())? * 2;
    }
    let class_data = cursor;
    let class_data_bytes = class_data_item(
        &(0..pool.fields.len())
            .map(|index| u16::try_from(index).unwrap())
            .collect::<Vec<_>>(),
        own_init,
        ctor_code,
        own_create,
        create_code,
        click_code.map(|offset| (pool.method(class, "onClick", &pv).unwrap(), offset)),
        &functions
            .iter()
            .zip(&function_code_offsets)
            .map(|(function, offset)| {
                (
                    pool.method(class, &function.name, &function_proto(function))
                        .unwrap(),
                    *offset,
                )
            })
            .collect::<Vec<_>>(),
    );
    cursor += u32_len(class_data_bytes.len())?;
    cursor = align4(cursor)?;
    let map = cursor;
    let map_count = 12_u32;
    let file_size = map + 4 + map_count * 12;

    let mut out = ByteWriter::new();
    out.write_bytes(b"dex\n035\0");
    out.write_u32(0);
    out.write_bytes(&[0; 20]);
    out.write_u32(file_size);
    out.write_u32(HEADER_SIZE);
    out.write_u32(0x1234_5678);
    out.write_u32(0);
    out.write_u32(0);
    out.write_u32(map);
    size_off(&mut out, pool.strings.len(), string_ids)?;
    size_off(&mut out, pool.types.len(), type_ids)?;
    size_off(&mut out, pool.protos.len(), proto_ids)?;
    size_off(&mut out, pool.fields.len(), field_ids)?;
    size_off(&mut out, pool.methods.len(), method_ids)?;
    size_off(&mut out, 1, class_defs)?;
    out.write_u32(file_size - data);
    out.write_u32(data);
    for offset in &string_offsets {
        out.write_u32(*offset);
    }
    for value in &pool.types {
        out.write_u32(*value);
    }
    for proto in &pool.protos {
        out.write_u32(pool.string_index[&shorty(proto)]);
        out.write_u32(u32::from(pool.type_index[proto.ret]));
        out.write_u32(
            type_lists
                .iter()
                .find(|(p, _)| p == proto)
                .map_or(0, |v| v.1),
        );
    }
    for field in &pool.fields {
        out.write_u16(pool.type_index[&field.class]);
        out.write_u16(pool.type_index[&field.ty]);
        out.write_u32(pool.string_index[&field.name]);
    }
    for method in &pool.methods {
        out.write_u16(pool.type_index[&method.class]);
        out.write_u16(pool.proto_index(&method.proto)?);
        out.write_u32(pool.string_index[&method.name]);
    }
    out.write_u32(u32::from(pool.type_index[class]));
    out.write_u32(1);
    out.write_u32(u32::from(pool.type_index["Landroid/app/Activity;"]));
    out.write_u32(interfaces_off);
    out.write_u32(NO_INDEX);
    out.write_u32(0);
    out.write_u32(class_data);
    out.write_u32(0);
    for (proto, expected) in &type_lists {
        out.align(4)?;
        debug_assert_eq!(u32_len(out.position())?, *expected);
        out.write_u32(u32_len(proto.params.len())?);
        for p in &proto.params {
            out.write_u16(pool.type_index[*p]);
        }
        if proto.params.len() % 2 != 0 {
            out.write_u16(0);
        }
    }
    if interfaces_off != 0 {
        out.align(4)?;
        debug_assert_eq!(u32_len(out.position())?, interfaces_off);
        out.write_u32(1);
        out.write_u16(pool.type_index["Landroid/view/View$OnClickListener;"]);
        out.write_u16(0);
    }
    for (value, expected) in pool.strings.iter().zip(&string_offsets) {
        debug_assert_eq!(u32_len(out.position())?, *expected);
        out.write_bytes(&encode_uleb128(u32_len(value.encode_utf16().count())?));
        out.write_bytes(&encode_mutf8(value));
    }
    out.align(4)?;
    write_code(&mut out, 1, 1, 1, &ctor);
    out.align(4)?;
    write_code(
        &mut out,
        create.registers,
        create.ins,
        create.outs,
        &create.code,
    );
    if let (Some(method), Some(expected)) = (&click, click_code) {
        out.align(4)?;
        debug_assert_eq!(u32_len(out.position())?, expected);
        write_code(
            &mut out,
            method.registers,
            method.ins,
            method.outs,
            &method.code,
        );
    }
    for (method, expected) in lowered.iter().zip(&function_code_offsets) {
        out.align(4)?;
        debug_assert_eq!(u32_len(out.position())?, *expected);
        write_code(
            &mut out,
            method.registers,
            method.ins,
            method.outs,
            &method.code,
        );
    }
    out.write_bytes(&class_data_bytes);
    out.align(4)?;
    let sections = [
        Section {
            kind: 0x0000,
            count: 1,
            offset: 0,
        },
        Section {
            kind: 0x0001,
            count: u32_len(pool.strings.len())?,
            offset: string_ids,
        },
        Section {
            kind: 0x0002,
            count: u32_len(pool.types.len())?,
            offset: type_ids,
        },
        Section {
            kind: 0x0003,
            count: u32_len(pool.protos.len())?,
            offset: proto_ids,
        },
        Section {
            kind: 0x0004,
            count: u32_len(pool.fields.len())?,
            offset: field_ids,
        },
        Section {
            kind: 0x0005,
            count: u32_len(pool.methods.len())?,
            offset: method_ids,
        },
        Section {
            kind: 0x0006,
            count: 1,
            offset: class_defs,
        },
        Section {
            kind: 0x1001,
            count: u32_len(type_lists.len())? + u32::from(interfaces_off != 0),
            offset: type_lists[0].1,
        },
        Section {
            kind: 0x2002,
            count: u32_len(pool.strings.len())?,
            offset: string_data,
        },
        Section {
            kind: 0x2001,
            count: 2 + u32_len(lowered.len())? + u32::from(click.is_some()),
            offset: ctor_code,
        },
        Section {
            kind: 0x2000,
            count: 1,
            offset: class_data,
        },
        Section {
            kind: 0x1000,
            count: 1,
            offset: map,
        },
    ];
    out.write_u32(map_count);
    for s in sections {
        out.write_u16(s.kind);
        out.write_u16(0);
        out.write_u32(s.count);
        out.write_u32(s.offset);
    }
    debug_assert_eq!(u32_len(out.position())?, file_size);
    let signature = sha1(&out.bytes()[32..]);
    out.bytes[12..32].copy_from_slice(&signature);
    let checksum = adler32(&out.bytes()[12..]);
    out.patch_u32(8, checksum)?;
    Ok(out.into_bytes())
}

fn class_data_item(
    fields: &[u16],
    init: u16,
    init_code: u32,
    create: u16,
    create_code: u32,
    click: Option<(u16, u32)>,
    functions: &[(u16, u32)],
) -> Vec<u8> {
    let mut direct = vec![(init, 0x1_0001, init_code)];
    direct.extend(
        functions
            .iter()
            .map(|(method, code)| (*method, 0x0a, *code)),
    );
    direct.sort_by_key(|entry| entry.0);
    let mut v = Vec::new();
    v.extend(encode_uleb128(0));
    v.extend(encode_uleb128(u32::try_from(fields.len()).unwrap()));
    v.extend(encode_uleb128(u32::try_from(direct.len()).unwrap()));
    v.extend(encode_uleb128(1 + u32::from(click.is_some())));
    let mut previous_field = 0_u16;
    for field in fields {
        v.extend(encode_uleb128(u32::from(*field - previous_field)));
        v.extend(encode_uleb128(0x2));
        previous_field = *field;
    }
    let mut previous = 0_u16;
    for (index, flags, code) in direct {
        v.extend(encode_uleb128(u32::from(index - previous)));
        v.extend(encode_uleb128(flags));
        v.extend(encode_uleb128(code));
        previous = index;
    }
    let mut virtuals = vec![(create, 0x4, create_code)];
    if let Some(entry) = click {
        virtuals.push((entry.0, 0x1, entry.1));
    }
    virtuals.sort_by_key(|entry| entry.0);
    let mut previous_virtual = 0_u16;
    for (method, flags, code) in virtuals {
        v.extend(encode_uleb128(u32::from(method - previous_virtual)));
        v.extend(encode_uleb128(flags));
        v.extend(encode_uleb128(code));
        previous_virtual = method;
    }
    v
}
fn write_code(out: &mut ByteWriter, registers: u16, ins: u16, outs: u16, code: &[u16]) {
    out.write_u16(registers);
    out.write_u16(ins);
    out.write_u16(outs);
    out.write_u16(0);
    out.write_u32(0);
    out.write_u32(u32::try_from(code.len()).unwrap());
    for word in code {
        out.write_u16(*word);
    }
}
fn size_off(out: &mut ByteWriter, size: usize, offset: u32) -> Result<(), DexError> {
    out.write_u32(u32_len(size)?);
    out.write_u32(if size == 0 { 0 } else { offset });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aic_ir::parse_program;
    const IR: &str = "aic_version 0.1\napp \"AIC Hello\" package \"dev.aic.generated.hello\" {\nactivity MainActivity {\non_create {\nlet root = android.linear_layout(orientation: vertical)\nlet message = android.text_view(text: \"Hello from AndroidIntelligentCompiler\")\nandroid.add_view(parent: root, child: message)\nandroid.set_content_view(root)\n}\n}\n}\n";
    #[test]
    fn deterministic_activity_dex() {
        let p = parse_program(IR).unwrap();
        let a = encode_activity_dex(&p).unwrap();
        assert_eq!(a, encode_activity_dex(&p).unwrap());
        assert!(a
            .windows(37)
            .any(|w| w == b"Hello from AndroidIntelligentCompiler"));
    }
}

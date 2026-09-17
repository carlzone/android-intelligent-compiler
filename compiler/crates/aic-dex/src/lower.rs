#![allow(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::semicolon_if_nothing_returned,
    clippy::type_complexity,
    clippy::too_many_lines
)]
use crate::{
    lir::{assemble, Instruction, Label, Register, ValueKind},
    DexError,
};
use aic_ir::{
    BinaryOp, CollectionItems, Expression, ExpressionKind, Function, Preference, Statement,
    StatementKind, Table, Type, UnaryOp, Value,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const VALID_I32_PATTERN: &str = "(?:[+-]?0*[0-9]{1,9}|[+]?0*(?:1[0-9]{9}|20[0-9]{8}|21[0-3][0-9]{7}|214[0-6][0-9]{6}|2147[0-3][0-9]{5}|21474[0-7][0-9]{4}|214748[0-2][0-9]{3}|2147483[0-5][0-9]{2}|21474836[0-3][0-9]|214748364[0-7])|-0*(?:1[0-9]{9}|20[0-9]{8}|21[0-3][0-9]{7}|214[0-6][0-9]{6}|2147[0-3][0-9]{5}|21474[0-7][0-9]{4}|214748[0-2][0-9]{3}|2147483[0-5][0-9]{2}|21474836[0-3][0-9]|214748364[0-8]))";

pub struct LoweredMethod {
    pub code: Vec<u16>,
    pub registers: u16,
    pub ins: u16,
    pub outs: u16,
}
pub struct LoweredEvents {
    pub click: Option<LoweredMethod>,
    pub list_select: Option<LoweredMethod>,
    pub spinner_select: Option<LoweredMethod>,
    pub nothing_selected: Option<LoweredMethod>,
}

#[allow(clippy::too_many_arguments)]
pub fn lower_events(
    clicks: &[aic_ir::ClickHandler],
    selects: &[aic_ir::SelectHandler],
    target: &dyn Fn(&str) -> Result<FunctionTarget, DexError>,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    strings: Option<StringLowering>,
    ui: UiLowering,
    state_fields: BTreeMap<String, (u16, Type)>,
    view_fields: BTreeMap<String, u16>,
    ready_fields: &BTreeMap<String, u16>,
    persistence: Option<PersistenceLowering>,
    preferences: &[Preference],
    tables: &[Table],
) -> Result<LoweredEvents, DexError> {
    let click = (!clicks.is_empty())
        .then(|| {
            lower_on_click(
                clicks,
                target,
                string_index,
                strings,
                ui,
                state_fields.clone(),
                view_fields.clone(),
                persistence,
                preferences,
                tables,
            )
        })
        .transpose()?;
    let has_list = selects.iter().any(|h| !ready_fields.contains_key(&h.view));
    let has_spinner = selects.iter().any(|h| ready_fields.contains_key(&h.view));
    let list_select = has_list
        .then(|| {
            lower_on_select(
                selects,
                false,
                target,
                string_index,
                strings,
                ui,
                state_fields.clone(),
                view_fields.clone(),
                ready_fields,
                persistence,
                preferences,
                tables,
            )
        })
        .transpose()?;
    let spinner_select = has_spinner
        .then(|| {
            lower_on_select(
                selects,
                true,
                target,
                string_index,
                strings,
                ui,
                state_fields,
                view_fields,
                ready_fields,
                persistence,
                preferences,
                tables,
            )
        })
        .transpose()?;
    let nothing_selected = has_spinner.then(|| LoweredMethod {
        code: assemble(&[Instruction::ReturnVoid]).expect("return-void assembles"),
        registers: 2,
        ins: 2,
        outs: 0,
    });
    Ok(LoweredEvents {
        click,
        list_select,
        spinner_select,
        nothing_selected,
    })
}
#[derive(Clone, Debug)]
pub struct FunctionTarget {
    pub method: u16,
    pub params: Vec<Type>,
    pub result: Type,
}
#[derive(Clone, Copy, Debug)]
pub struct StringLowering {
    pub value_of_i32: u16,
    pub value_of_bool: u16,
    pub builder_type: u16,
    pub builder_init: u16,
    pub builder_append: u16,
    pub builder_to_string: u16,
    pub equals: u16,
    pub text_view_get_text: u16,
    pub object_to_string: u16,
    pub string_matches: u16,
    pub integer_parse_int: u16,
    pub context_get_string: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct UiLowering {
    pub activity_on_create: u16,
    pub context_get_resources: u16,
    pub resources_get_configuration: u16,
    pub configuration_orientation: Option<u16>,
    pub configuration_screen_width_dp: Option<u16>,
    pub linear_layout_type: u16,
    pub linear_layout_init: u16,
    pub linear_layout_orientation: u16,
    pub text_view_type: u16,
    pub text_view_init: u16,
    pub text_view_set_text: u16,
    pub text_view_set_text_size: u16,
    pub text_view_set_freezes_text: u16,
    pub button_type: u16,
    pub button_init: u16,
    pub edit_text_type: u16,
    pub edit_text_init: u16,
    pub edit_text_set_hint: u16,
    pub edit_text_set_input_type: u16,
    pub scroll_view_type: u16,
    pub scroll_view_init: u16,
    pub frame_layout_type: u16,
    pub frame_layout_init: u16,
    pub check_box_type: u16,
    pub check_box_init: u16,
    pub switch_type: u16,
    pub switch_init: u16,
    pub progress_bar_type: u16,
    pub progress_bar_init: u16,
    pub image_view_type: u16,
    pub image_view_init: u16,
    pub image_view_set_resource: u16,
    pub toolbar_type: u16,
    pub toolbar_init: u16,
    pub toolbar_set_title: u16,
    pub list_view_type: u16,
    pub list_view_init: u16,
    pub spinner_type: u16,
    pub spinner_init: u16,
    pub string_array_type: u16,
    pub array_adapter_type: u16,
    pub array_adapter_init: u16,
    pub array_adapter_set_drop_down_view_resource: u16,
    pub list_view_set_adapter: u16,
    pub spinner_set_adapter: u16,
    pub set_on_click_listener: u16,
    pub list_view_set_on_item_click_listener: u16,
    pub spinner_set_on_item_selected_listener: u16,
    pub adapter_view_get_item_at_position: u16,
    pub object_to_string: u16,
    pub layout_params_type: u16,
    pub layout_params_init: u16,
    pub layout_params_set_margins: u16,
    pub set_layout_params: u16,
    pub color_parse: u16,
    pub set_text_color: u16,
    pub set_background_color: u16,
    pub add_view: u16,
    pub set_fits_system_windows: u16,
    pub set_content_view: u16,
    pub intent_type: u16,
    pub intent_init: u16,
    pub intent_set_class_name: u16,
    pub intent_put_i32: u16,
    pub intent_put_bool: u16,
    pub intent_put_string: u16,
    pub start_activity: u16,
    pub finish_activity: u16,
    pub set_padding: u16,
    pub set_visibility: u16,
    pub set_enabled: u16,
    pub set_content_description: u16,
    pub set_important_for_accessibility: u16,
    pub resources_get_color: u16,
    pub resources_get_display_metrics: u16,
    pub display_metrics_density_dpi: u16,
    pub density_dpi_field: u16,
    pub minimum_touch_target_field: u16,
    pub set_minimum_width: u16,
    pub set_minimum_height: u16,
    pub set_view_id: u16,
    pub set_save_enabled: u16,
    pub text_view_set_label_for: u16,
    pub set_accessibility_heading: u16,
    pub sdk_int_field: Option<u16>,
    pub set_text_alignment: u16,
    pub dialog_builder_type: u16,
    pub dialog_builder_init: u16,
    pub dialog_set_title: u16,
    pub dialog_set_message: u16,
    pub dialog_show: u16,
    pub popup_menu_type: u16,
    pub popup_menu_init: u16,
    pub popup_menu_get_menu: u16,
    pub menu_add: u16,
    pub popup_menu_show: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct PersistenceLowering {
    pub preferences_field: Option<u16>,
    pub database_field: Option<u16>,
    pub get_shared_preferences: u16,
    pub pref_get_i32: u16,
    pub pref_get_bool: u16,
    pub pref_get_string: u16,
    pub pref_edit: u16,
    pub editor_put_i32: u16,
    pub editor_put_bool: u16,
    pub editor_put_string: u16,
    pub editor_apply: u16,
    pub open_database: u16,
    pub database_exec_sql: u16,
    pub database_compile: u16,
    pub statement_bind_string: u16,
    pub statement_execute_insert: u16,
    pub statement_simple_long: u16,
    pub statement_simple_string: u16,
    pub statement_execute_update_delete: u16,
    pub statement_close: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct LifecycleLowering {
    pub bundle_contains_key: u16,
    pub bundle_get_i32: u16,
    pub bundle_get_bool: u16,
    pub bundle_get_string: u16,
    pub bundle_put_i32: u16,
    pub bundle_put_bool: u16,
    pub bundle_put_string: u16,
    pub activity_on_save_instance_state: u16,
}
#[derive(Clone, Copy)]
struct Binding {
    register: Register,
    ty: Type,
}
struct Lowerer<'a> {
    code: Vec<Instruction>,
    locals: BTreeMap<String, Binding>,
    views: BTreeMap<String, Register>,
    state_fields: BTreeMap<String, (u16, Type)>,
    collection_fields: BTreeMap<String, u16>,
    selection_views: BTreeMap<String, bool>,
    touch_target_views: BTreeSet<String>,
    view_fields: BTreeMap<String, u16>,
    this: Option<Register>,
    next: u8,
    limit: u8,
    label: u16,
    resolve: &'a dyn Fn(&str) -> Result<u16, DexError>,
    target: Option<&'a dyn Fn(&str) -> Result<FunctionTarget, DexError>>,
    string_index: Option<&'a dyn Fn(&str) -> Result<u16, DexError>>,
    strings: Option<StringLowering>,
    persistence: Option<PersistenceLowering>,
    preferences: BTreeMap<String, Preference>,
    tables: BTreeMap<String, Table>,
    outs: u16,
}
fn kind(t: Type) -> ValueKind {
    if t == Type::String {
        ValueKind::Reference
    } else if t == Type::Bool {
        ValueKind::Bool
    } else {
        ValueKind::I32
    }
}
impl Lowerer<'_> {
    fn alloc(&mut self, t: Type) -> Result<Register, DexError> {
        if self.next >= self.limit {
            return Err(DexError::InvalidInput("M2 register allocation exhausted"));
        }
        let r = Register {
            index: self.next,
            kind: kind(t),
        };
        self.next += 1;
        Ok(r)
    }
    fn label(&mut self) -> Label {
        let l = Label(self.label);
        self.label += 1;
        l
    }
    fn release_temporaries(&mut self, candidate: u8) {
        let live = self
            .locals
            .values()
            .map(|binding| binding.register.index.saturating_add(1))
            .chain(
                self.views
                    .values()
                    .map(|register| register.index.saturating_add(1)),
            )
            .max()
            .unwrap_or(0);
        self.next = candidate.max(live);
    }

    fn initialize_minimum_touch_target(
        &mut self,
        context: Register,
        ui: UiLowering,
    ) -> Result<(), DexError> {
        let target = self.alloc(Type::I32)?;
        let resources = self.alloc(Type::String)?;
        let metrics = self.alloc(Type::String)?;
        let scratch = self.alloc(Type::I32)?;
        self.code.push(Instruction::InvokeVirtual {
            method: ui.context_get_resources,
            args: vec![context],
        });
        self.code
            .push(Instruction::MoveResultObject { dst: resources });
        self.code.push(Instruction::InvokeVirtual {
            method: ui.resources_get_display_metrics,
            args: vec![resources],
        });
        self.code
            .push(Instruction::MoveResultObject { dst: metrics });
        self.code.push(Instruction::IGet {
            dst: scratch,
            object: metrics,
            field: ui.display_metrics_density_dpi,
        });
        let this = self
            .this
            .ok_or(DexError::InvalidInput("missing Activity receiver"))?;
        self.code.push(Instruction::IPut {
            src: scratch,
            object: this,
            field: ui.density_dpi_field,
        });
        self.code.push(Instruction::Const16 {
            dst: target,
            value: 48,
        });
        self.code.push(Instruction::MulInt {
            dst: target,
            left: target,
            right: scratch,
        });
        self.code.push(Instruction::Const16 {
            dst: scratch,
            value: 159,
        });
        self.code.push(Instruction::AddInt {
            dst: target,
            left: target,
            right: scratch,
        });
        self.code.push(Instruction::Const16 {
            dst: scratch,
            value: 160,
        });
        self.code.push(Instruction::DivInt {
            dst: target,
            left: target,
            right: scratch,
        });
        self.code.push(Instruction::IPut {
            src: target,
            object: this,
            field: ui.minimum_touch_target_field,
        });
        self.outs = self.outs.max(2);
        self.next = 0;
        Ok(())
    }

    fn apply_minimum_touch_target(
        &mut self,
        view: Register,
        ui: UiLowering,
    ) -> Result<(), DexError> {
        let target = self.alloc(Type::I32)?;
        let this = self
            .this
            .ok_or(DexError::InvalidInput("missing Activity receiver"))?;
        self.code.push(Instruction::IGet {
            dst: target,
            object: this,
            field: ui.minimum_touch_target_field,
        });
        for method in [ui.set_minimum_width, ui.set_minimum_height] {
            self.code.push(Instruction::InvokeVirtual {
                method,
                args: vec![view, target],
            });
        }
        self.outs = self.outs.max(2);
        self.release_temporaries(view.index + 1);
        Ok(())
    }
    fn assign_view_id(
        &mut self,
        name: &str,
        view: Register,
        ui: UiLowering,
    ) -> Result<(), DexError> {
        let id = self.alloc(Type::I32)?;
        let field = *self
            .view_fields
            .get(name)
            .ok_or(DexError::InvalidInput("missing stable view field"))?;
        self.code.push(Instruction::Const32 {
            dst: id,
            value: 0x00a1_0000 | i32::from(field),
        });
        self.code.push(Instruction::InvokeVirtual {
            method: ui.set_view_id,
            args: vec![view, id],
        });
        self.outs = self.outs.max(2);
        self.release_temporaries(view.index + 1);
        Ok(())
    }
    fn enable_text_state(&mut self, view: Register, ui: UiLowering) -> Result<(), DexError> {
        let enabled = self.alloc(Type::Bool)?;
        self.code.push(Instruction::Const4 {
            dst: enabled,
            value: 1,
        });
        self.code.push(Instruction::InvokeVirtual {
            method: ui.text_view_set_freezes_text,
            args: vec![view, enabled],
        });
        self.outs = self.outs.max(2);
        self.release_temporaries(view.index + 1);
        Ok(())
    }
    fn ty(&self, e: &Expression) -> Result<Type, DexError> {
        match &e.kind {
            ExpressionKind::Literal(v) => Ok(v.ty()),
            ExpressionKind::Name(n) => self
                .locals
                .get(n)
                .map(|b| b.ty)
                .or_else(|| self.state_fields.get(n).map(|entry| entry.1))
                .ok_or(DexError::InvalidInput("unknown lowered local")),
            ExpressionKind::Unary { op, .. } => Ok(if *op == UnaryOp::Not {
                Type::Bool
            } else {
                Type::I32
            }),
            ExpressionKind::Binary { op, left, .. } => Ok(match op {
                BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::And
                | BinaryOp::Or => Type::Bool,
                BinaryOp::Add if self.ty(left)? == Type::String => Type::String,
                _ => Type::I32,
            }),
            ExpressionKind::Call { name, .. } if name == "string" => Ok(Type::String),
            ExpressionKind::Call { name, .. } if name == "valid_i32" => Ok(Type::Bool),
            ExpressionKind::Call { name, .. } if name == "i32" => Ok(Type::I32),
            ExpressionKind::Call { name, .. } => self
                .target
                .map_or(Ok(Type::I32), |resolve| Ok(resolve(name)?.result)),
            ExpressionKind::AndroidText { .. } | ExpressionKind::ResourceString { .. } => {
                Ok(Type::String)
            }
            ExpressionKind::PreferenceGet { key } => self
                .preferences
                .get(key)
                .map(|p| p.ty)
                .ok_or(DexError::InvalidInput("unknown preference")),
            ExpressionKind::DatabaseInsert { .. } | ExpressionKind::ResourceColor { .. } => {
                Ok(Type::I32)
            }
            ExpressionKind::DatabaseExists { .. } => Ok(Type::Bool),
            ExpressionKind::DatabaseGet { table, column, .. } => self
                .tables
                .get(table)
                .and_then(|t| t.columns.iter().find(|c| c.name == *column))
                .map(|c| c.ty)
                .ok_or(DexError::InvalidInput("unknown database column")),
        }
    }
    fn mov(&mut self, d: Register, s: Register) -> Result<(), DexError> {
        if d.kind != s.kind {
            return Err(DexError::InvalidInput("mismatched move register kinds"));
        }
        self.code.push(if d.kind == ValueKind::Reference {
            Instruction::MoveObject { dst: d, src: s }
        } else {
            Instruction::Move { dst: d, src: s }
        });
        Ok(())
    }
    fn expr(&mut self, e: &Expression, d: Register) -> Result<(), DexError> {
        if kind(self.ty(e)?) != d.kind {
            return Err(DexError::InvalidInput(
                "expression destination kind mismatch",
            ));
        }
        let n = self.next;
        let r = self.expr0(e, d);
        self.next = n;
        r
    }
    fn expr0(&mut self, e: &Expression, d: Register) -> Result<(), DexError> {
        match &e.kind {
            ExpressionKind::Literal(Value::I32(v)) => {
                let v = i16::try_from(*v).map_err(|_| {
                    DexError::InvalidInput("integer constant exceeds current const/16 lowering")
                })?;
                self.code.push(if (-8..=7).contains(&v) {
                    Instruction::Const4 {
                        dst: d,
                        value: v as i8,
                    }
                } else {
                    Instruction::Const16 { dst: d, value: v }
                })
            }
            ExpressionKind::Literal(Value::Bool(v)) => self.code.push(Instruction::Const4 {
                dst: d,
                value: i8::from(*v),
            }),
            ExpressionKind::Literal(Value::String(value)) => {
                let index = self.string_index.ok_or(DexError::InvalidInput(
                    "string literal requires indexed lowering context",
                ))?(value)?;
                self.code.push(Instruction::ConstString {
                    dst: d,
                    string: index,
                });
            }
            ExpressionKind::ResourceString { id, .. } => {
                let this = self.this.ok_or(DexError::InvalidInput(
                    "resource string requires Activity receiver",
                ))?;
                let resource = self.alloc(Type::I32)?;
                self.code.push(Instruction::Const32 {
                    dst: resource,
                    value: (*id).cast_signed(),
                });
                let strings = self.strings.ok_or(DexError::InvalidInput(
                    "resource string lowering unavailable",
                ))?;
                self.code.push(Instruction::InvokeVirtual {
                    method: strings.context_get_string,
                    args: vec![this, resource],
                });
                self.code.push(Instruction::MoveResultObject { dst: d });
                self.outs = self.outs.max(2);
            }
            ExpressionKind::ResourceColor { id, .. } => self.code.push(Instruction::Const32 {
                dst: d,
                value: (*id).cast_signed(),
            }),
            ExpressionKind::Name(n) => {
                if let Some(local) = self.locals.get(n) {
                    self.mov(d, local.register)?;
                } else {
                    let (field, _) = *self
                        .state_fields
                        .get(n)
                        .ok_or(DexError::InvalidInput("unknown lowered local"))?;
                    self.code.push(Instruction::IGet {
                        dst: d,
                        object: self
                            .this
                            .ok_or(DexError::InvalidInput("state outside activity"))?,
                        field,
                    });
                }
            }
            ExpressionKind::Unary { op, value } => {
                let s = self.alloc(self.ty(value)?)?;
                self.expr(value, s)?;
                let z = self.alloc(Type::I32)?;
                self.code.push(Instruction::Const4 {
                    dst: z,
                    value: i8::from(*op == UnaryOp::Not),
                });
                self.code.push(Instruction::SubInt {
                    dst: d,
                    left: z,
                    right: s,
                })
            }
            ExpressionKind::Binary { op, left, right } => {
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    self.expr(left, d)?;
                    let end = self.label();
                    self.code.push(if *op == BinaryOp::And {
                        Instruction::IfEqz {
                            value: d,
                            target: end,
                        }
                    } else {
                        Instruction::IfNez {
                            value: d,
                            target: end,
                        }
                    });
                    self.expr(right, d)?;
                    self.code.push(Instruction::Label(end));
                    return Ok(());
                }
                if self.ty(left)? == Type::String {
                    return match op {
                        BinaryOp::Add => self.concat(left, right, d),
                        BinaryOp::Equal | BinaryOp::NotEqual => {
                            self.string_equal(*op, left, right, d)
                        }
                        _ => Err(DexError::InvalidInput("invalid string lowering operator")),
                    };
                }
                let a = self.alloc(self.ty(left)?)?;
                let b = self.alloc(self.ty(right)?)?;
                self.expr(left, a)?;
                self.expr(right, b)?;
                match op {
                    BinaryOp::Add => self.code.push(Instruction::AddInt {
                        dst: d,
                        left: a,
                        right: b,
                    }),
                    BinaryOp::Subtract => self.code.push(Instruction::SubInt {
                        dst: d,
                        left: a,
                        right: b,
                    }),
                    BinaryOp::Multiply => self.code.push(Instruction::MulInt {
                        dst: d,
                        left: a,
                        right: b,
                    }),
                    BinaryOp::Divide => self.code.push(Instruction::DivInt {
                        dst: d,
                        left: a,
                        right: b,
                    }),
                    BinaryOp::Remainder => self.code.push(Instruction::RemInt {
                        dst: d,
                        left: a,
                        right: b,
                    }),
                    BinaryOp::And | BinaryOp::Or => unreachable!(),
                    x => self.compare(*x, a, b, d)?,
                }
            }
            ExpressionKind::Call { name, args } if name == "string" => {
                let source = self.alloc(self.ty(&args[0])?)?;
                self.expr(&args[0], source)?;
                let strings = self.strings.ok_or(DexError::InvalidInput(
                    "string conversion requires platform methods",
                ))?;
                self.code.push(Instruction::InvokeStatic {
                    method: if self.ty(&args[0])? == Type::Bool {
                        strings.value_of_bool
                    } else {
                        strings.value_of_i32
                    },
                    args: vec![source],
                });
                self.code.push(Instruction::MoveResultObject { dst: d });
                self.outs = self.outs.max(1);
            }
            ExpressionKind::Call { name, args } if name == "valid_i32" => {
                let source = self.alloc(Type::String)?;
                self.expr(&args[0], source)?;
                let pattern = self.alloc(Type::String)?;
                let index = self
                    .string_index
                    .ok_or(DexError::InvalidInput("missing string index"))?(
                    VALID_I32_PATTERN
                )?;
                self.code.push(Instruction::ConstString {
                    dst: pattern,
                    string: index,
                });
                let methods = self
                    .strings
                    .ok_or(DexError::InvalidInput("missing input methods"))?;
                self.code.push(Instruction::InvokeVirtual {
                    method: methods.string_matches,
                    args: vec![source, pattern],
                });
                self.code.push(Instruction::MoveResult { dst: d });
                self.outs = self.outs.max(2);
            }
            ExpressionKind::Call { name, args } if name == "i32" => {
                let source = self.alloc(Type::String)?;
                self.expr(&args[0], source)?;
                let methods = self
                    .strings
                    .ok_or(DexError::InvalidInput("missing input methods"))?;
                self.code.push(Instruction::InvokeStatic {
                    method: methods.integer_parse_int,
                    args: vec![source],
                });
                self.code.push(Instruction::MoveResult { dst: d });
                self.outs = self.outs.max(1);
            }
            ExpressionKind::Call { name, args } => {
                let target = self.target.map(|resolve| resolve(name)).transpose()?;
                if target
                    .as_ref()
                    .is_some_and(|target| target.params.len() != args.len())
                {
                    return Err(DexError::InvalidInput("mismatched lowered call arguments"));
                }
                let mut rs = Vec::new();
                for (index, a) in args.iter().enumerate() {
                    if let Some(target) = &target {
                        if self.ty(a)? != target.params[index] {
                            return Err(DexError::InvalidInput(
                                "mismatched lowered call argument kind",
                            ));
                        }
                    }
                    let r = self.alloc(self.ty(a)?)?;
                    self.expr(a, r)?;
                    rs.push(r)
                }
                self.outs = self.outs.max(rs.len() as u16);
                self.code.push(Instruction::InvokeStatic {
                    method: target
                        .as_ref()
                        .map_or_else(|| (self.resolve)(name), |target| Ok(target.method))?,
                    args: rs,
                });
                self.code.push(if self.ty(e)? == Type::String {
                    Instruction::MoveResultObject { dst: d }
                } else {
                    Instruction::MoveResult { dst: d }
                });
            }
            ExpressionKind::AndroidText { view } => {
                let source = if let Some(source) = self.views.get(view) {
                    *source
                } else {
                    let register = self.alloc(Type::String)?;
                    let field = *self
                        .view_fields
                        .get(view)
                        .ok_or(DexError::InvalidInput("unknown input view"))?;
                    self.code.push(Instruction::IGet {
                        dst: register,
                        object: self
                            .this
                            .ok_or(DexError::InvalidInput("input outside activity"))?,
                        field,
                    });
                    register
                };
                let methods = self
                    .strings
                    .ok_or(DexError::InvalidInput("missing input methods"))?;
                let temporary = self.alloc(Type::String)?;
                self.code.push(Instruction::InvokeVirtual {
                    method: methods.text_view_get_text,
                    args: vec![source],
                });
                self.code
                    .push(Instruction::MoveResultObject { dst: temporary });
                self.code.push(Instruction::InvokeVirtual {
                    method: methods.object_to_string,
                    args: vec![temporary],
                });
                self.code.push(Instruction::MoveResultObject { dst: d });
                self.outs = self.outs.max(1);
            }
            ExpressionKind::PreferenceGet { key } => self.preference_get(key, d)?,
            ExpressionKind::DatabaseInsert { table, values } => {
                self.database_insert(table, values, d)?
            }
            ExpressionKind::DatabaseExists { table, id } => {
                self.database_query(table, id, None, None, d)?
            }
            ExpressionKind::DatabaseGet {
                table,
                id,
                column,
                default,
            } => self.database_query(table, id, Some(column), Some(default), d)?,
        }
        Ok(())
    }
    fn preference_get(&mut self, key: &str, dst: Register) -> Result<(), DexError> {
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        let preference = self
            .preferences
            .get(key)
            .cloned()
            .ok_or(DexError::InvalidInput("unknown preference"))?;
        let this = self
            .this
            .ok_or(DexError::InvalidInput("preference outside activity"))?;
        let object = self.alloc(Type::String)?;
        let key_register = self.alloc(Type::String)?;
        let default = self.alloc(preference.ty)?;
        self.code.push(Instruction::IGet {
            dst: object,
            object: this,
            field: p
                .preferences_field
                .ok_or(DexError::InvalidInput("missing preferences field"))?,
        });
        self.code.push(Instruction::ConstString {
            dst: key_register,
            string: (self
                .string_index
                .ok_or(DexError::InvalidInput("missing string pool"))?)(key)?,
        });
        self.literal(&preference.default, default)?;
        self.code.push(Instruction::InvokeInterface {
            method: match preference.ty {
                Type::I32 => p.pref_get_i32,
                Type::Bool => p.pref_get_bool,
                Type::String => p.pref_get_string,
            },
            args: vec![object, key_register, default],
        });
        self.code.push(if preference.ty == Type::String {
            Instruction::MoveResultObject { dst }
        } else {
            Instruction::MoveResult { dst }
        });
        self.outs = self.outs.max(3);
        Ok(())
    }
    fn literal(&mut self, value: &Value, dst: Register) -> Result<(), DexError> {
        match value {
            Value::I32(v) => self.code.push(Instruction::Const32 { dst, value: *v }),
            Value::Bool(v) => self.code.push(Instruction::Const4 {
                dst,
                value: i8::from(*v),
            }),
            Value::String(v) => self.code.push(Instruction::ConstString {
                dst,
                string: (self
                    .string_index
                    .ok_or(DexError::InvalidInput("missing string pool"))?)(
                    v
                )?,
            }),
        }
        Ok(())
    }
    fn alloc_wide(&mut self) -> Result<u8, DexError> {
        if self.next + 1 >= self.limit {
            return Err(DexError::InvalidInput("wide register allocation exhausted"));
        }
        let result = self.next;
        self.next += 2;
        Ok(result)
    }
    fn primary_key(&self, table: &str) -> Result<String, DexError> {
        self.tables
            .get(table)
            .and_then(|t| t.columns.iter().find(|c| c.primary_key))
            .map(|c| c.name.clone())
            .ok_or(DexError::InvalidInput("missing primary key"))
    }
    fn compile_statement(&mut self, sql: &str) -> Result<Register, DexError> {
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        let this = self
            .this
            .ok_or(DexError::InvalidInput("database outside activity"))?;
        let db = self.alloc(Type::String)?;
        let query = self.alloc(Type::String)?;
        let statement = self.alloc(Type::String)?;
        self.code.push(Instruction::IGet {
            dst: db,
            object: this,
            field: p
                .database_field
                .ok_or(DexError::InvalidInput("missing database field"))?,
        });
        self.code.push(Instruction::ConstString {
            dst: query,
            string: (self
                .string_index
                .ok_or(DexError::InvalidInput("missing query string"))?)(sql)?,
        });
        self.code.push(Instruction::InvokeVirtual {
            method: p.database_compile,
            args: vec![db, query],
        });
        self.code
            .push(Instruction::MoveResultObject { dst: statement });
        self.outs = self.outs.max(2);
        Ok(statement)
    }
    fn bind_value(
        &mut self,
        statement: Register,
        index: i32,
        value: &Expression,
    ) -> Result<(), DexError> {
        let saved = self.next;
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        let slot = self.alloc(Type::I32)?;
        self.code.push(Instruction::Const32 {
            dst: slot,
            value: index,
        });
        let rendered = self.alloc(Type::String)?;
        if self.ty(value)? == Type::String {
            self.expr(value, rendered)?;
        } else {
            let ty = self.ty(value)?;
            let scalar = self.alloc(ty)?;
            self.expr(value, scalar)?;
            let strings = self
                .strings
                .ok_or(DexError::InvalidInput("missing string conversions"))?;
            self.code.push(Instruction::InvokeStatic {
                method: if ty == Type::Bool {
                    strings.value_of_bool
                } else {
                    strings.value_of_i32
                },
                args: vec![scalar],
            });
            self.code
                .push(Instruction::MoveResultObject { dst: rendered });
        }
        self.code.push(Instruction::InvokeVirtual {
            method: p.statement_bind_string,
            args: vec![statement, slot, rendered],
        });
        self.outs = self.outs.max(3);
        self.next = saved;
        Ok(())
    }
    fn close_statement(&mut self, statement: Register) -> Result<(), DexError> {
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        self.code.push(Instruction::InvokeVirtual {
            method: p.statement_close,
            args: vec![statement],
        });
        Ok(())
    }
    fn database_insert(
        &mut self,
        table: &str,
        values: &[(String, Expression)],
        dst: Register,
    ) -> Result<(), DexError> {
        let columns = values
            .iter()
            .map(|v| v.0.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "INSERT INTO {table} ({columns}) VALUES ({})",
            vec!["?"; values.len()].join(",")
        );
        let statement = self.compile_statement(&sql)?;
        for (index, (_, value)) in values.iter().enumerate() {
            self.bind_value(
                statement,
                i32::try_from(index + 1).map_err(|_| DexError::ArithmeticOverflow)?,
                value,
            )?;
        }
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        self.code.push(Instruction::InvokeVirtual {
            method: p.statement_execute_insert,
            args: vec![statement],
        });
        let wide = self.alloc_wide()?;
        self.code.push(Instruction::MoveResultWide { dst: wide });
        self.code.push(Instruction::LongToInt { dst, src: wide });
        self.close_statement(statement)
    }
    fn database_query(
        &mut self,
        table: &str,
        id: &Expression,
        column: Option<&String>,
        default: Option<&Expression>,
        dst: Register,
    ) -> Result<(), DexError> {
        let primary = self.primary_key(table)?;
        let sql = column.map_or_else(
            || format!("SELECT COUNT(*) FROM {table} WHERE {primary} = ?"),
            |column| {
                format!("SELECT COALESCE((SELECT {column} FROM {table} WHERE {primary} = ?), ?)")
            },
        );
        let statement = self.compile_statement(&sql)?;
        self.bind_value(statement, 1, id)?;
        if let Some(default) = default {
            self.bind_value(statement, 2, default)?;
        }
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        if column.is_some() && dst.kind == ValueKind::Reference {
            self.code.push(Instruction::InvokeVirtual {
                method: p.statement_simple_string,
                args: vec![statement],
            });
            self.code.push(Instruction::MoveResultObject { dst });
        } else {
            self.code.push(Instruction::InvokeVirtual {
                method: p.statement_simple_long,
                args: vec![statement],
            });
            let wide = self.alloc_wide()?;
            self.code.push(Instruction::MoveResultWide { dst: wide });
            self.code.push(Instruction::LongToInt { dst, src: wide });
        }
        self.close_statement(statement)
    }
    fn concat(
        &mut self,
        left: &Expression,
        right: &Expression,
        dst: Register,
    ) -> Result<(), DexError> {
        let strings = self.strings.ok_or(DexError::InvalidInput(
            "string concatenation requires platform methods",
        ))?;
        let left_register = self.alloc(Type::String)?;
        let right_register = self.alloc(Type::String)?;
        let builder = self.alloc(Type::String)?;
        self.expr(left, left_register)?;
        self.expr(right, right_register)?;
        self.code.push(Instruction::NewInstance {
            dst: builder,
            ty: strings.builder_type,
        });
        self.code.push(Instruction::InvokeDirect {
            method: strings.builder_init,
            args: vec![builder],
        });
        for value in [left_register, right_register] {
            self.code.push(Instruction::InvokeVirtual {
                method: strings.builder_append,
                args: vec![builder, value],
            });
            self.code
                .push(Instruction::MoveResultObject { dst: builder });
        }
        self.code.push(Instruction::InvokeVirtual {
            method: strings.builder_to_string,
            args: vec![builder],
        });
        self.code.push(Instruction::MoveResultObject { dst });
        self.outs = self.outs.max(2);
        Ok(())
    }
    fn string_equal(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
        dst: Register,
    ) -> Result<(), DexError> {
        let strings = self.strings.ok_or(DexError::InvalidInput(
            "string equality requires platform methods",
        ))?;
        let left_register = self.alloc(Type::String)?;
        let right_register = self.alloc(Type::String)?;
        self.expr(left, left_register)?;
        self.expr(right, right_register)?;
        self.code.push(Instruction::InvokeVirtual {
            method: strings.equals,
            args: vec![left_register, right_register],
        });
        self.code.push(Instruction::MoveResult { dst });
        if op == BinaryOp::NotEqual {
            let one = self.alloc(Type::Bool)?;
            self.code.push(Instruction::Const4 { dst: one, value: 1 });
            self.code.push(Instruction::SubInt {
                dst,
                left: one,
                right: dst,
            });
        }
        self.outs = self.outs.max(2);
        Ok(())
    }
    fn compare(
        &mut self,
        op: BinaryOp,
        a: Register,
        b: Register,
        d: Register,
    ) -> Result<(), DexError> {
        let yes = self.label();
        let end = self.label();
        self.code.push(Instruction::Const4 { dst: d, value: 0 });
        match op {
            BinaryOp::Less => self.code.push(Instruction::IfLt {
                left: a,
                right: b,
                target: yes,
            }),
            BinaryOp::LessEqual => self.code.push(Instruction::IfLe {
                left: a,
                right: b,
                target: yes,
            }),
            BinaryOp::Greater => self.code.push(Instruction::IfGt {
                left: a,
                right: b,
                target: yes,
            }),
            BinaryOp::GreaterEqual => self.code.push(Instruction::IfGe {
                left: a,
                right: b,
                target: yes,
            }),
            BinaryOp::Equal | BinaryOp::NotEqual => {
                let x = self.alloc(Type::I32)?;
                self.code.push(Instruction::SubInt {
                    dst: x,
                    left: a,
                    right: b,
                });
                self.code.push(if op == BinaryOp::Equal {
                    Instruction::IfEqz {
                        value: x,
                        target: yes,
                    }
                } else {
                    Instruction::IfNez {
                        value: x,
                        target: yes,
                    }
                })
            }
            _ => return Err(DexError::InvalidInput("invalid comparison operator")),
        }
        self.code.push(Instruction::Goto16 { target: end });
        self.code.push(Instruction::Label(yes));
        self.code.push(Instruction::Const4 { dst: d, value: 1 });
        self.code.push(Instruction::Label(end));
        Ok(())
    }
    fn stmts(&mut self, body: &[Statement]) -> Result<(), DexError> {
        for s in body {
            match &s.kind {
                StatementKind::Declare {
                    name, ty, value, ..
                } => {
                    let t = ty.unwrap_or(self.ty(value)?);
                    let r = self.alloc(t)?;
                    self.expr(value, r)?;
                    self.locals
                        .insert(name.clone(), Binding { register: r, ty: t });
                }
                StatementKind::Assign { name, value } => {
                    let b = *self.locals.get(name).ok_or(DexError::InvalidInput(
                        "assignment to unknown lowered local",
                    ))?;
                    self.expr(value, b.register)?
                }
                StatementKind::Return(v) => {
                    let t = self.ty(v)?;
                    let r = self.alloc(t)?;
                    self.expr(v, r)?;
                    self.code.push(if t == Type::String {
                        Instruction::ReturnObject { value: r }
                    } else {
                        Instruction::Return { value: r }
                    })
                }
                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let r = self.alloc(Type::Bool)?;
                    self.expr(condition, r)?;
                    let other = self.label();
                    let end = self.label();
                    self.code.push(Instruction::IfEqz {
                        value: r,
                        target: other,
                    });
                    self.stmts(then_body)?;
                    self.code.push(Instruction::Goto16 { target: end });
                    self.code.push(Instruction::Label(other));
                    self.stmts(else_body)?;
                    self.code.push(Instruction::Label(end))
                }
                StatementKind::For {
                    variable,
                    start,
                    end,
                    body,
                } => {
                    let i = self.alloc(Type::I32)?;
                    let bound = self.alloc(Type::I32)?;
                    self.expr(start, i)?;
                    self.expr(end, bound)?;
                    self.locals.insert(
                        variable.clone(),
                        Binding {
                            register: i,
                            ty: Type::I32,
                        },
                    );
                    let top = self.label();
                    let done = self.label();
                    self.code.push(Instruction::Label(top));
                    self.code.push(Instruction::IfGe {
                        left: i,
                        right: bound,
                        target: done,
                    });
                    self.stmts(body)?;
                    self.code.push(Instruction::AddIntLit8 {
                        dst: i,
                        src: i,
                        value: 1,
                    });
                    self.code.push(Instruction::Goto16 { target: top });
                    self.code.push(Instruction::Label(done));
                    self.locals.remove(variable);
                }
                _ => {
                    return Err(DexError::InvalidInput(
                        "Android operation inside user function",
                    ))
                }
            }
        }
        Ok(())
    }
}
pub fn lower_function(
    function: &Function,
    resolve: &dyn Fn(&str) -> Result<u16, DexError>,
) -> Result<LoweredMethod, DexError> {
    if function.params.len() > 5 {
        return Err(DexError::InvalidInput(
            "M2 function exceeds five parameters",
        ));
    }
    let first = 16 - function.params.len() as u8;
    let mut l = Lowerer {
        code: vec![],
        locals: BTreeMap::new(),
        views: BTreeMap::new(),
        state_fields: BTreeMap::new(),
        collection_fields: BTreeMap::new(),
        selection_views: BTreeMap::new(),
        touch_target_views: BTreeSet::new(),
        view_fields: BTreeMap::new(),
        this: None,
        next: 0,
        limit: first,
        label: 0,
        resolve,
        target: None,
        string_index: None,
        strings: None,
        persistence: None,
        preferences: BTreeMap::new(),
        tables: BTreeMap::new(),
        outs: 0,
    };
    for (i, p) in function.params.iter().enumerate() {
        l.locals.insert(
            p.name.clone(),
            Binding {
                register: Register {
                    index: first + i as u8,
                    kind: kind(p.ty),
                },
                ty: p.ty,
            },
        );
    }
    l.stmts(&function.body)?;
    Ok(LoweredMethod {
        code: assemble(&l.code)?,
        registers: 16,
        ins: function.params.len() as u16,
        outs: l.outs,
    })
}

pub fn lower_function_typed(
    function: &Function,
    target: &dyn Fn(&str) -> Result<FunctionTarget, DexError>,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    strings: StringLowering,
) -> Result<LoweredMethod, DexError> {
    if function.params.len() > 5 {
        return Err(DexError::InvalidInput(
            "M2 function exceeds five parameters",
        ));
    }
    let first = 16 - function.params.len() as u8;
    let fallback = |name: &str| Ok(target(name)?.method);
    let mut lowerer = Lowerer {
        code: vec![],
        locals: BTreeMap::new(),
        views: BTreeMap::new(),
        state_fields: BTreeMap::new(),
        collection_fields: BTreeMap::new(),
        selection_views: BTreeMap::new(),
        touch_target_views: BTreeSet::new(),
        view_fields: BTreeMap::new(),
        this: None,
        next: 0,
        limit: first,
        label: 0,
        resolve: &fallback,
        target: Some(target),
        string_index: Some(string_index),
        strings: Some(strings),
        persistence: None,
        preferences: BTreeMap::new(),
        tables: BTreeMap::new(),
        outs: 0,
    };
    for (index, parameter) in function.params.iter().enumerate() {
        lowerer.locals.insert(
            parameter.name.clone(),
            Binding {
                register: Register {
                    index: first + index as u8,
                    kind: kind(parameter.ty),
                },
                ty: parameter.ty,
            },
        );
    }
    lowerer.stmts(&function.body)?;
    Ok(LoweredMethod {
        code: assemble(&lowerer.code)?,
        registers: 16,
        ins: function.params.len() as u16,
        outs: lowerer.outs,
    })
}

impl Lowerer<'_> {
    fn fill_string_array(
        &mut self,
        items: &[Expression],
        array: Register,
        array_type: u16,
    ) -> Result<(), DexError> {
        let size = self.alloc(Type::I32)?;
        self.code.push(Instruction::Const16 {
            dst: size,
            value: i16::try_from(items.len()).map_err(|_| DexError::ArithmeticOverflow)?,
        });
        self.code.push(Instruction::NewArray {
            dst: array,
            size,
            ty: array_type,
        });
        for (offset, item) in items.iter().enumerate() {
            let value = self.alloc(Type::String)?;
            let index = self.alloc(Type::I32)?;
            self.expr(item, value)?;
            self.code.push(Instruction::Const16 {
                dst: index,
                value: i16::try_from(offset).map_err(|_| DexError::ArithmeticOverflow)?,
            });
            self.code.push(Instruction::AputObject {
                value,
                array,
                index,
            });
            self.release_temporaries(array.index + 1);
        }
        Ok(())
    }

    fn on_create_statements(
        &mut self,
        statements: &[Statement],
        this: Register,
        ui: UiLowering,
    ) -> Result<(), DexError> {
        for statement in statements {
            match &statement.kind {
                StatementKind::Declare {
                    name, ty, value, ..
                } => {
                    let value_type = ty.unwrap_or(self.ty(value)?);
                    let register = self.alloc(value_type)?;
                    self.expr(value, register)?;
                    self.locals.insert(
                        name.clone(),
                        Binding {
                            register,
                            ty: value_type,
                        },
                    );
                }
                StatementKind::Assign { name, value } => {
                    if let Some(binding) = self.locals.get(name).copied() {
                        self.expr(value, binding.register)?;
                    } else {
                        let (field, ty) = *self.state_fields.get(name).ok_or(
                            DexError::InvalidInput("assignment to unknown activity value"),
                        )?;
                        let register = self.alloc(ty)?;
                        self.expr(value, register)?;
                        self.code.push(Instruction::IPut {
                            src: register,
                            object: this,
                            field,
                        });
                        self.release_temporaries(register.index);
                    }
                }
                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let condition_register = self.alloc(Type::Bool)?;
                    self.expr(condition, condition_register)?;
                    let otherwise = self.label();
                    let end = self.label();
                    self.code.push(Instruction::IfEqz {
                        value: condition_register,
                        target: otherwise,
                    });
                    self.on_create_statements(then_body, this, ui)?;
                    self.code.push(Instruction::Goto16 { target: end });
                    self.code.push(Instruction::Label(otherwise));
                    self.on_create_statements(else_body, this, ui)?;
                    self.code.push(Instruction::Label(end));
                }
                StatementKind::For {
                    variable,
                    start,
                    end,
                    body,
                } => {
                    let index = self.alloc(Type::I32)?;
                    let bound = self.alloc(Type::I32)?;
                    self.expr(start, index)?;
                    self.expr(end, bound)?;
                    self.locals.insert(
                        variable.clone(),
                        Binding {
                            register: index,
                            ty: Type::I32,
                        },
                    );
                    let top = self.label();
                    let done = self.label();
                    self.code.push(Instruction::Label(top));
                    self.code.push(Instruction::IfGe {
                        left: index,
                        right: bound,
                        target: done,
                    });
                    self.on_create_statements(body, this, ui)?;
                    self.code.push(Instruction::AddIntLit8 {
                        dst: index,
                        src: index,
                        value: 1,
                    });
                    self.code.push(Instruction::Goto16 { target: top });
                    self.code.push(Instruction::Label(done));
                    self.locals.remove(variable);
                }
                StatementKind::LinearLayout {
                    id,
                    orientation: direction,
                } => {
                    let view = self.alloc(Type::String)?;
                    let orientation = self.alloc(Type::I32)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.linear_layout_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.linear_layout_init,
                        args: vec![view, this],
                    });
                    self.code.push(Instruction::Const4 {
                        dst: orientation,
                        value: i8::from(*direction == aic_ir::Orientation::Vertical),
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.linear_layout_orientation,
                        args: vec![view, orientation],
                    });
                    self.outs = self.outs.max(2);
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::TextView { id, text } => {
                    let view = self.alloc(Type::String)?;
                    let rendered = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.text_view_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.text_view_init,
                        args: vec![view, this],
                    });
                    self.expr(text, rendered)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.text_view_set_text,
                        args: vec![view, rendered],
                    });
                    self.outs = self.outs.max(2);
                    self.enable_text_state(view, ui)?;
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::Button { id, text } => {
                    let view = self.alloc(Type::String)?;
                    let rendered = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.button_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.button_init,
                        args: vec![view, this],
                    });
                    self.expr(text, rendered)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.text_view_set_text,
                        args: vec![view, rendered],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(rendered.index);
                    self.apply_minimum_touch_target(view, ui)?;
                    self.enable_text_state(view, ui)?;
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_on_click_listener,
                        args: vec![view, this],
                    });
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::EditText { id, hint }
                | StatementKind::TextInput {
                    id,
                    hint,
                    input_type: _,
                } => {
                    let view = self.alloc(Type::String)?;
                    let rendered = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.edit_text_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.edit_text_init,
                        args: vec![view, this],
                    });
                    self.expr(hint, rendered)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.edit_text_set_hint,
                        args: vec![view, rendered],
                    });
                    let input_type = self.alloc(Type::I32)?;
                    let input_type_value = match &statement.kind {
                        StatementKind::EditText { .. }
                        | StatementKind::TextInput {
                            input_type: aic_ir::InputType::Integer,
                            ..
                        } => 0x0000_0002,
                        StatementKind::TextInput {
                            input_type: aic_ir::InputType::Text,
                            ..
                        } => 0x0000_0001,
                        StatementKind::TextInput {
                            input_type: aic_ir::InputType::Email,
                            ..
                        } => 0x0000_0021,
                        StatementKind::TextInput {
                            input_type: aic_ir::InputType::Password,
                            ..
                        } => 0x0000_0081,
                        StatementKind::TextInput {
                            input_type: aic_ir::InputType::Phone,
                            ..
                        } => 0x0000_0003,
                        _ => unreachable!(),
                    };
                    self.code.push(Instruction::Const32 {
                        dst: input_type,
                        value: input_type_value,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.edit_text_set_input_type,
                        args: vec![view, input_type],
                    });
                    if matches!(
                        &statement.kind,
                        StatementKind::TextInput {
                            input_type: aic_ir::InputType::Password,
                            ..
                        }
                    ) {
                        let disabled = self.alloc(Type::Bool)?;
                        self.code.push(Instruction::Const4 {
                            dst: disabled,
                            value: 0,
                        });
                        self.code.push(Instruction::InvokeVirtual {
                            method: ui.set_save_enabled,
                            args: vec![view, disabled],
                        });
                    }
                    self.outs = self.outs.max(2);
                    self.apply_minimum_touch_target(view, ui)?;
                    self.enable_text_state(view, ui)?;
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::ScrollView { id } => {
                    let view = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.scroll_view_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.scroll_view_init,
                        args: vec![view, this],
                    });
                    self.outs = self.outs.max(2);
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                }
                StatementKind::FrameLayout { id }
                | StatementKind::ProgressBar { id }
                | StatementKind::ImageView { id, .. } => {
                    let view = self.alloc(Type::String)?;
                    let (ty, init) = match &statement.kind {
                        StatementKind::FrameLayout { .. } => {
                            (ui.frame_layout_type, ui.frame_layout_init)
                        }
                        StatementKind::ProgressBar { .. } => {
                            (ui.progress_bar_type, ui.progress_bar_init)
                        }
                        _ => (ui.image_view_type, ui.image_view_init),
                    };
                    self.code.push(Instruction::NewInstance { dst: view, ty });
                    self.code.push(Instruction::InvokeDirect {
                        method: init,
                        args: vec![view, this],
                    });
                    if let StatementKind::ImageView { source, .. } = &statement.kind {
                        let resource = self.alloc(Type::I32)?;
                        self.code.push(Instruction::Const32 {
                            dst: resource,
                            value: match source {
                                aic_ir::ImageSource::Builtin(aic_ir::BuiltinIcon::Info) => {
                                    0x0108_009b
                                }
                                aic_ir::ImageSource::Builtin(aic_ir::BuiltinIcon::Warning) => {
                                    0x0108_0027
                                }
                                aic_ir::ImageSource::Builtin(aic_ir::BuiltinIcon::Delete) => {
                                    0x0108_0040
                                }
                                aic_ir::ImageSource::Resource { id, .. } => (*id).cast_signed(),
                            },
                        });
                        self.code.push(Instruction::InvokeVirtual {
                            method: ui.image_view_set_resource,
                            args: vec![view, resource],
                        });
                    }
                    self.outs = self.outs.max(2);
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::CheckBox { id, text } | StatementKind::Switch { id, text } => {
                    let view = self.alloc(Type::String)?;
                    let rendered = self.alloc(Type::String)?;
                    let (ty, init) = if matches!(statement.kind, StatementKind::CheckBox { .. }) {
                        (ui.check_box_type, ui.check_box_init)
                    } else {
                        (ui.switch_type, ui.switch_init)
                    };
                    self.code.push(Instruction::NewInstance { dst: view, ty });
                    self.code.push(Instruction::InvokeDirect {
                        method: init,
                        args: vec![view, this],
                    });
                    self.expr(text, rendered)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.text_view_set_text,
                        args: vec![view, rendered],
                    });
                    self.outs = self.outs.max(2);
                    self.apply_minimum_touch_target(view, ui)?;
                    self.enable_text_state(view, ui)?;
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::Toolbar { id, title } => {
                    let view = self.alloc(Type::String)?;
                    let rendered = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.toolbar_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.toolbar_init,
                        args: vec![view, this],
                    });
                    self.expr(title, rendered)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.toolbar_set_title,
                        args: vec![view, rendered],
                    });
                    self.outs = self.outs.max(2);
                    self.apply_minimum_touch_target(view, ui)?;
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::ListView { id, items } => {
                    let view = self.alloc(Type::String)?;
                    let array = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.list_view_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.list_view_init,
                        args: vec![view, this],
                    });
                    match items {
                        CollectionItems::Inline(items) => {
                            self.fill_string_array(items, array, ui.string_array_type)?
                        }
                        CollectionItems::State(name) => self.code.push(Instruction::IGet {
                            dst: array,
                            object: this,
                            field: self.collection_fields[name],
                        }),
                    }
                    let adapter = self.alloc(Type::String)?;
                    let layout = self.alloc(Type::I32)?;
                    self.code.push(Instruction::NewInstance {
                        dst: adapter,
                        ty: ui.array_adapter_type,
                    });
                    self.code.push(Instruction::Const32 {
                        dst: layout,
                        value: 0x0109_0006,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.array_adapter_init,
                        args: vec![adapter, this, layout, array],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.list_view_set_adapter,
                        args: vec![view, adapter],
                    });
                    self.outs = self.outs.max(4);
                    self.apply_minimum_touch_target(view, ui)?;
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    if self.selection_views.get(id) == Some(&false) {
                        self.code.push(Instruction::InvokeVirtual {
                            method: ui.list_view_set_on_item_click_listener,
                            args: vec![view, this],
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::Spinner { id, items } => {
                    let view = self.alloc(Type::String)?;
                    let array = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: view,
                        ty: ui.spinner_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.spinner_init,
                        args: vec![view, this],
                    });
                    match items {
                        CollectionItems::Inline(items) => {
                            self.fill_string_array(items, array, ui.string_array_type)?
                        }
                        CollectionItems::State(name) => self.code.push(Instruction::IGet {
                            dst: array,
                            object: this,
                            field: self.collection_fields[name],
                        }),
                    }
                    let adapter = self.alloc(Type::String)?;
                    let layout = self.alloc(Type::I32)?;
                    self.code.push(Instruction::NewInstance {
                        dst: adapter,
                        ty: ui.array_adapter_type,
                    });
                    self.code.push(Instruction::Const32 {
                        dst: layout,
                        value: 0x0109_0008,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.array_adapter_init,
                        args: vec![adapter, this, layout, array],
                    });
                    self.code.push(Instruction::Const32 {
                        dst: layout,
                        value: 0x0109_0009,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.array_adapter_set_drop_down_view_resource,
                        args: vec![adapter, layout],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.spinner_set_adapter,
                        args: vec![view, adapter],
                    });
                    self.outs = self.outs.max(4);
                    self.apply_minimum_touch_target(view, ui)?;
                    self.views.insert(id.clone(), view);
                    self.assign_view_id(id, view, ui)?;
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    if self.selection_views.get(id) == Some(&true) {
                        self.code.push(Instruction::InvokeVirtual {
                            method: ui.spinner_set_on_item_selected_listener,
                            args: vec![view, this],
                        });
                    }
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::AddView { parent, child } => {
                    let parent = *self
                        .views
                        .get(parent)
                        .ok_or(DexError::InvalidInput("missing Android parent view ID"))?;
                    let child = *self
                        .views
                        .get(child)
                        .ok_or(DexError::InvalidInput("missing Android child view ID"))?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.add_view,
                        args: vec![parent, child],
                    });
                    self.outs = self.outs.max(2);
                }
                StatementKind::SetContentView { view } => {
                    let view = *self
                        .views
                        .get(view)
                        .ok_or(DexError::InvalidInput("missing Android content view ID"))?;
                    let enabled = self.alloc(Type::Bool)?;
                    self.code.push(Instruction::Const4 {
                        dst: enabled,
                        value: 1,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_fits_system_windows,
                        args: vec![view, enabled],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_content_view,
                        args: vec![this, view],
                    });
                    self.outs = self.outs.max(2);
                }
                StatementKind::SetText { view, text } => {
                    let view = if let Some(register) = self.views.get(view) {
                        *register
                    } else {
                        let register = self.alloc(Type::String)?;
                        let field = *self
                            .view_fields
                            .get(view)
                            .ok_or(DexError::InvalidInput("missing Android text view ID"))?;
                        self.code.push(Instruction::IGet {
                            dst: register,
                            object: this,
                            field,
                        });
                        register
                    };
                    let rendered = self.alloc(Type::String)?;
                    self.expr(text, rendered)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.text_view_set_text,
                        args: vec![view, rendered],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(rendered.index);
                }
                StatementKind::SetTextSize { view, size_sp } => {
                    let view = self.load_view(view, this)?;
                    let unit = self.alloc(Type::I32)?;
                    let size = self.alloc(Type::I32)?;
                    let size_bits = f32::from(
                        i16::try_from(*size_sp).map_err(|_| DexError::ArithmeticOverflow)?,
                    )
                    .to_bits()
                    .cast_signed();
                    self.code.push(Instruction::Const4 {
                        dst: unit,
                        value: 2,
                    });
                    self.code.push(Instruction::Const32 {
                        dst: size,
                        value: size_bits,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.text_view_set_text_size,
                        args: vec![view, unit, size],
                    });
                    self.outs = self.outs.max(3);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetLayout {
                    view,
                    width,
                    height,
                    weight,
                    margins,
                } => {
                    let enforces_touch_target = self.touch_target_views.contains(view);
                    let view = *self
                        .views
                        .get(view)
                        .ok_or(DexError::InvalidInput("missing layout view"))?;
                    let params = self.alloc(Type::String)?;
                    let w = self.alloc(Type::I32)?;
                    let h = self.alloc(Type::I32)?;
                    let weight_register = self.alloc(Type::I32)?;
                    self.code.push(Instruction::NewInstance {
                        dst: params,
                        ty: ui.layout_params_type,
                    });
                    for (register, size) in [(w, width), (h, height)] {
                        match size {
                            aic_ir::LayoutSize::MatchParent => {
                                self.code.push(Instruction::Const4 {
                                    dst: register,
                                    value: -1,
                                })
                            }
                            aic_ir::LayoutSize::WrapContent => {
                                self.code.push(Instruction::Const4 {
                                    dst: register,
                                    value: -2,
                                })
                            }
                            aic_ir::LayoutSize::Dp(value) => {
                                self.code.push(Instruction::Const32 {
                                    dst: register,
                                    value: *value,
                                });
                                self.code.push(Instruction::IGet {
                                    dst: weight_register,
                                    object: this,
                                    field: ui.density_dpi_field,
                                });
                                self.code.push(Instruction::MulInt {
                                    dst: register,
                                    left: register,
                                    right: weight_register,
                                });
                                self.code.push(Instruction::Const16 {
                                    dst: weight_register,
                                    value: 159,
                                });
                                self.code.push(Instruction::AddInt {
                                    dst: register,
                                    left: register,
                                    right: weight_register,
                                });
                                self.code.push(Instruction::Const16 {
                                    dst: weight_register,
                                    value: 160,
                                });
                                self.code.push(Instruction::DivInt {
                                    dst: register,
                                    left: register,
                                    right: weight_register,
                                });
                                if enforces_touch_target {
                                    let done = self.label();
                                    self.code.push(Instruction::IGet {
                                        dst: weight_register,
                                        object: this,
                                        field: ui.minimum_touch_target_field,
                                    });
                                    self.code.push(Instruction::IfGe {
                                        left: register,
                                        right: weight_register,
                                        target: done,
                                    });
                                    self.code.push(Instruction::Move {
                                        dst: register,
                                        src: weight_register,
                                    });
                                    self.code.push(Instruction::Label(done));
                                }
                            }
                        }
                    }
                    self.code.push(Instruction::Const32 {
                        dst: weight_register,
                        value: if *weight == 0 { 0 } else { 0x3f80_0000 },
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.layout_params_init,
                        args: vec![params, w, h, weight_register],
                    });
                    if margins.iter().any(|margin| *margin != 0) {
                        let mut margin_args = vec![params];
                        let density = self.alloc(Type::I32)?;
                        for margin in margins {
                            let register = self.alloc(Type::I32)?;
                            self.code.push(Instruction::Const32 {
                                dst: register,
                                value: *margin,
                            });
                            self.code.push(Instruction::IGet {
                                dst: density,
                                object: this,
                                field: ui.density_dpi_field,
                            });
                            self.code.push(Instruction::MulInt {
                                dst: register,
                                left: register,
                                right: density,
                            });
                            self.code.push(Instruction::Const16 {
                                dst: density,
                                value: 159,
                            });
                            self.code.push(Instruction::AddInt {
                                dst: register,
                                left: register,
                                right: density,
                            });
                            self.code.push(Instruction::Const16 {
                                dst: density,
                                value: 160,
                            });
                            self.code.push(Instruction::DivInt {
                                dst: register,
                                left: register,
                                right: density,
                            });
                            margin_args.push(register);
                        }
                        self.code.push(Instruction::InvokeVirtual {
                            method: ui.layout_params_set_margins,
                            args: margin_args,
                        });
                    }
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_layout_params,
                        args: vec![view, params],
                    });
                    self.outs = self.outs.max(5);
                    self.release_temporaries(params.index);
                }
                StatementKind::SetTextColor { view, color }
                | StatementKind::SetBackgroundColor { view, color } => {
                    let view = *self
                        .views
                        .get(view)
                        .ok_or(DexError::InvalidInput("missing color view"))?;
                    let text = self.alloc(Type::String)?;
                    let parsed = self.alloc(Type::I32)?;
                    self.code.push(Instruction::ConstString {
                        dst: text,
                        string: self
                            .string_index
                            .ok_or(DexError::InvalidInput("missing color string index"))?(
                            color
                        )?,
                    });
                    self.code.push(Instruction::InvokeStatic {
                        method: ui.color_parse,
                        args: vec![text],
                    });
                    self.code.push(Instruction::MoveResult { dst: parsed });
                    self.code.push(Instruction::InvokeVirtual {
                        method: if matches!(&statement.kind, StatementKind::SetTextColor { .. }) {
                            ui.set_text_color
                        } else {
                            ui.set_background_color
                        },
                        args: vec![view, parsed],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(text.index);
                }
                StatementKind::StartActivity { activity, extras } => {
                    let intent = self.alloc(Type::String)?;
                    let target = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: intent,
                        ty: ui.intent_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.intent_init,
                        args: vec![intent],
                    });
                    self.code.push(Instruction::ConstString {
                        dst: target,
                        string: self
                            .string_index
                            .ok_or(DexError::InvalidInput("missing activity string index"))?(
                            activity,
                        )?,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.intent_set_class_name,
                        args: vec![intent, this, target],
                    });
                    for (key, value) in extras {
                        let key_register = self.alloc(Type::String)?;
                        self.code.push(Instruction::ConstString {
                            dst: key_register,
                            string: self.string_index.ok_or(DexError::InvalidInput(
                                "missing navigation extra key index",
                            ))?(key)?,
                        });
                        let value_register = self.alloc(self.ty(value)?)?;
                        self.expr(value, value_register)?;
                        self.code.push(Instruction::InvokeVirtual {
                            method: match self.ty(value)? {
                                Type::I32 => ui.intent_put_i32,
                                Type::Bool => ui.intent_put_bool,
                                Type::String => ui.intent_put_string,
                            },
                            args: vec![intent, key_register, value_register],
                        });
                        self.outs = self.outs.max(3);
                        self.release_temporaries(key_register.index);
                    }
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.start_activity,
                        args: vec![this, intent],
                    });
                    self.outs = self.outs.max(3);
                    self.release_temporaries(intent.index);
                }
                StatementKind::FinishActivity => {
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.finish_activity,
                        args: vec![this],
                    });
                    self.outs = self.outs.max(1);
                }
                StatementKind::SetPadding {
                    view,
                    left,
                    top,
                    right,
                    bottom,
                } => {
                    let view = self.load_view(view, this)?;
                    let mut args = vec![view];
                    for value in [left, top, right, bottom] {
                        let r = self.alloc(Type::I32)?;
                        self.code.push(Instruction::Const32 {
                            dst: r,
                            value: *value,
                        });
                        args.push(r);
                    }
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_padding,
                        args,
                    });
                    self.outs = self.outs.max(5);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetTextResourceColor { view, id, .. }
                | StatementKind::SetBackgroundResourceColor { view, id, .. } => {
                    let view = *self
                        .views
                        .get(view)
                        .ok_or(DexError::InvalidInput("unknown color target"))?;
                    let resources = self.alloc(Type::String)?;
                    let resource_id = self.alloc(Type::I32)?;
                    let color = self.alloc(Type::I32)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.context_get_resources,
                        args: vec![this],
                    });
                    self.code
                        .push(Instruction::MoveResultObject { dst: resources });
                    self.code.push(Instruction::Const32 {
                        dst: resource_id,
                        value: (*id).cast_signed(),
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.resources_get_color,
                        args: vec![resources, resource_id],
                    });
                    self.code.push(Instruction::MoveResult { dst: color });
                    self.code.push(Instruction::InvokeVirtual {
                        method: if matches!(
                            &statement.kind,
                            StatementKind::SetTextResourceColor { .. }
                        ) {
                            ui.set_text_color
                        } else {
                            ui.set_background_color
                        },
                        args: vec![view, color],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetVisibility { view, visibility } => {
                    let view = self.load_view(view, this)?;
                    let value = self.alloc(Type::I32)?;
                    self.code.push(Instruction::Const32 {
                        dst: value,
                        value: match visibility {
                            aic_ir::Visibility::Visible => 0,
                            aic_ir::Visibility::Invisible => 4,
                            aic_ir::Visibility::Gone => 8,
                        },
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_visibility,
                        args: vec![view, value],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetEnabled { view, enabled } => {
                    let view = self.load_view(view, this)?;
                    let value = self.alloc(Type::Bool)?;
                    self.expr(enabled, value)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_enabled,
                        args: vec![view, value],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetContentDescription { view, text } => {
                    let view = self.load_view(view, this)?;
                    let value = self.alloc(Type::String)?;
                    self.expr(text, value)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_content_description,
                        args: vec![view, value],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetDecorative { view } => {
                    let view = self.load_view(view, this)?;
                    let value = self.alloc(Type::I32)?;
                    self.code.push(Instruction::Const32 {
                        dst: value,
                        value: 2,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_important_for_accessibility,
                        args: vec![view, value],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetInputLabel { label, input } => {
                    let label = self.load_view(label, this)?;
                    let input_name = input;
                    let input = self.load_view(input_name, this)?;
                    let id = self.alloc(Type::I32)?;
                    let field = *self
                        .view_fields
                        .get(input_name)
                        .ok_or(DexError::InvalidInput("missing stable input field"))?;
                    self.code.push(Instruction::Const32 {
                        dst: id,
                        value: 0x00a1_0000 | i32::from(field),
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_view_id,
                        args: vec![input, id],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.text_view_set_label_for,
                        args: vec![label, id],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(label.index + 1);
                }
                StatementKind::SetHeading { view } => {
                    let view = self.load_view(view, this)?;
                    let enabled = self.alloc(Type::Bool)?;
                    let sdk = self.alloc(Type::I32)?;
                    let api_28 = self.alloc(Type::I32)?;
                    let done = self.label();
                    self.code.push(Instruction::SGet {
                        dst: sdk,
                        field: ui
                            .sdk_int_field
                            .ok_or(DexError::InvalidInput("missing Android SDK level field"))?,
                    });
                    self.code.push(Instruction::Const16 {
                        dst: api_28,
                        value: 28,
                    });
                    self.code.push(Instruction::IfLt {
                        left: sdk,
                        right: api_28,
                        target: done,
                    });
                    self.code.push(Instruction::Const4 {
                        dst: enabled,
                        value: 1,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_accessibility_heading,
                        args: vec![view, enabled],
                    });
                    self.code.push(Instruction::Label(done));
                    self.outs = self.outs.max(2);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::SetGravity { view, gravity } => {
                    let view = self.load_view(view, this)?;
                    let value = self.alloc(Type::I32)?;
                    self.code.push(Instruction::Const4 {
                        dst: value,
                        value: match gravity {
                            aic_ir::Gravity::Start => 2,
                            aic_ir::Gravity::Center => 4,
                            aic_ir::Gravity::End => 6,
                        },
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_text_alignment,
                        args: vec![view, value],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(view.index + 1);
                }
                StatementKind::ShowDialog { title, message } => {
                    let builder = self.alloc(Type::String)?;
                    let title_value = self.alloc(Type::String)?;
                    let message_value = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: builder,
                        ty: ui.dialog_builder_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.dialog_builder_init,
                        args: vec![builder, this],
                    });
                    self.expr(title, title_value)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.dialog_set_title,
                        args: vec![builder, title_value],
                    });
                    self.expr(message, message_value)?;
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.dialog_set_message,
                        args: vec![builder, message_value],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.dialog_show,
                        args: vec![builder],
                    });
                    self.outs = self.outs.max(2);
                    self.release_temporaries(builder.index);
                }
                StatementKind::ShowMenu { anchor, item } => {
                    let anchor = self.load_view(anchor, this)?;
                    let popup = self.alloc(Type::String)?;
                    let menu = self.alloc(Type::String)?;
                    let item_value = self.alloc(Type::String)?;
                    self.code.push(Instruction::NewInstance {
                        dst: popup,
                        ty: ui.popup_menu_type,
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.popup_menu_init,
                        args: vec![popup, this, anchor],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.popup_menu_get_menu,
                        args: vec![popup],
                    });
                    self.code.push(Instruction::MoveResultObject { dst: menu });
                    self.expr(item, item_value)?;
                    self.code.push(Instruction::InvokeInterface {
                        method: ui.menu_add,
                        args: vec![menu, item_value],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.popup_menu_show,
                        args: vec![popup],
                    });
                    self.outs = self.outs.max(3);
                    self.release_temporaries(anchor.index + 1);
                }
                StatementKind::PreferenceSet { key, value } => {
                    let saved = self.next;
                    self.preference_set(key, value)?;
                    self.next = saved;
                }
                StatementKind::DatabaseUpdate { table, id, values } => {
                    let saved = self.next;
                    self.database_mutation(table, Some(id), values)?;
                    self.next = saved;
                }
                StatementKind::DatabaseDelete { table, id } => {
                    let saved = self.next;
                    self.database_mutation(table, Some(id), &[])?;
                    self.next = saved;
                }
                StatementKind::Return(_) => {
                    return Err(DexError::InvalidInput("return inside onCreate"))
                }
            }
        }
        Ok(())
    }
    fn load_view(&mut self, name: &str, this: Register) -> Result<Register, DexError> {
        if let Some(view) = self.views.get(name) {
            return Ok(*view);
        }
        let view = self.alloc(Type::String)?;
        let field = *self
            .view_fields
            .get(name)
            .ok_or(DexError::InvalidInput("missing Android view ID"))?;
        self.code.push(Instruction::IGet {
            dst: view,
            object: this,
            field,
        });
        Ok(view)
    }
    fn preference_set(&mut self, key: &str, value: &Expression) -> Result<(), DexError> {
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        let this = self
            .this
            .ok_or(DexError::InvalidInput("preference outside activity"))?;
        let preferences = self.alloc(Type::String)?;
        let editor = self.alloc(Type::String)?;
        let key_register = self.alloc(Type::String)?;
        let ty = self.ty(value)?;
        let value_register = self.alloc(ty)?;
        self.code.push(Instruction::IGet {
            dst: preferences,
            object: this,
            field: p
                .preferences_field
                .ok_or(DexError::InvalidInput("missing preferences field"))?,
        });
        self.code.push(Instruction::InvokeInterface {
            method: p.pref_edit,
            args: vec![preferences],
        });
        self.code
            .push(Instruction::MoveResultObject { dst: editor });
        self.code.push(Instruction::ConstString {
            dst: key_register,
            string: (self
                .string_index
                .ok_or(DexError::InvalidInput("missing string pool"))?)(key)?,
        });
        self.expr(value, value_register)?;
        self.code.push(Instruction::InvokeInterface {
            method: match ty {
                Type::I32 => p.editor_put_i32,
                Type::Bool => p.editor_put_bool,
                Type::String => p.editor_put_string,
            },
            args: vec![editor, key_register, value_register],
        });
        self.code
            .push(Instruction::MoveResultObject { dst: editor });
        self.code.push(Instruction::InvokeInterface {
            method: p.editor_apply,
            args: vec![editor],
        });
        self.outs = self.outs.max(3);
        Ok(())
    }
    fn database_mutation(
        &mut self,
        table: &str,
        id: Option<&Expression>,
        values: &[(String, Expression)],
    ) -> Result<(), DexError> {
        let primary = self.primary_key(table)?;
        let sql = if values.is_empty() {
            format!("DELETE FROM {table} WHERE {primary} = ?")
        } else {
            format!(
                "UPDATE {table} SET {} WHERE {primary} = ?",
                values
                    .iter()
                    .map(|v| format!("{} = ?", v.0))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let statement = self.compile_statement(&sql)?;
        for (index, (_, value)) in values.iter().enumerate() {
            self.bind_value(
                statement,
                i32::try_from(index + 1).map_err(|_| DexError::ArithmeticOverflow)?,
                value,
            )?;
        }
        if let Some(id) = id {
            self.bind_value(
                statement,
                i32::try_from(values.len() + 1).map_err(|_| DexError::ArithmeticOverflow)?,
                id,
            )?;
        }
        let p = self
            .persistence
            .ok_or(DexError::InvalidInput("missing persistence lowering"))?;
        self.code.push(Instruction::InvokeVirtual {
            method: p.statement_execute_update_delete,
            args: vec![statement],
        });
        self.close_statement(statement)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn lower_on_click(
    handlers: &[aic_ir::ClickHandler],
    target: &dyn Fn(&str) -> Result<FunctionTarget, DexError>,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    strings: Option<StringLowering>,
    ui: UiLowering,
    state_fields: BTreeMap<String, (u16, Type)>,
    view_fields: BTreeMap<String, u16>,
    persistence: Option<PersistenceLowering>,
    preferences: &[Preference],
    tables: &[Table],
) -> Result<LoweredMethod, DexError> {
    let fallback = |name: &str| Ok(target(name)?.method);
    let this = Register {
        index: 14,
        kind: ValueKind::Reference,
    };
    let clicked = Register {
        index: 15,
        kind: ValueKind::Reference,
    };
    let mut lowerer = Lowerer {
        code: vec![],
        locals: BTreeMap::new(),
        views: BTreeMap::new(),
        state_fields,
        collection_fields: BTreeMap::new(),
        selection_views: BTreeMap::new(),
        touch_target_views: BTreeSet::new(),
        view_fields,
        this: Some(this),
        next: 0,
        limit: 14,
        label: 0,
        resolve: &fallback,
        target: Some(target),
        string_index: Some(string_index),
        strings,
        persistence,
        preferences: preferences
            .iter()
            .map(|p| (p.name.clone(), p.clone()))
            .collect(),
        tables: tables.iter().map(|t| (t.name.clone(), t.clone())).collect(),
        outs: 0,
    };
    for handler in handlers {
        lowerer.next = 0;
        let next = lowerer.label();
        let target_view = lowerer.alloc(Type::String)?;
        lowerer.code.push(Instruction::IGet {
            dst: target_view,
            object: this,
            field: lowerer.view_fields[&handler.view],
        });
        lowerer.code.push(Instruction::IfNe {
            left: clicked,
            right: target_view,
            target: next,
        });
        lowerer.on_create_statements(&handler.body, this, ui)?;
        lowerer.code.push(Instruction::ReturnVoid);
        lowerer.code.push(Instruction::Label(next));
    }
    lowerer.code.push(Instruction::ReturnVoid);
    Ok(LoweredMethod {
        code: assemble(&lowerer.code)?,
        registers: 16,
        ins: 2,
        outs: lowerer.outs,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn lower_on_select(
    handlers: &[aic_ir::SelectHandler],
    spinner: bool,
    target: &dyn Fn(&str) -> Result<FunctionTarget, DexError>,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    strings: Option<StringLowering>,
    ui: UiLowering,
    state_fields: BTreeMap<String, (u16, Type)>,
    view_fields: BTreeMap<String, u16>,
    ready_fields: &BTreeMap<String, u16>,
    persistence: Option<PersistenceLowering>,
    preferences: &[Preference],
    tables: &[Table],
) -> Result<LoweredMethod, DexError> {
    let fallback = |name: &str| Ok(target(name)?.method);
    let this = Register {
        index: 10,
        kind: ValueKind::Reference,
    };
    let parent = Register {
        index: 11,
        kind: ValueKind::Reference,
    };
    let position = Register {
        index: 13,
        kind: ValueKind::I32,
    };
    let mut lowerer = Lowerer {
        code: vec![],
        locals: BTreeMap::new(),
        views: BTreeMap::new(),
        state_fields,
        collection_fields: BTreeMap::new(),
        selection_views: BTreeMap::new(),
        touch_target_views: BTreeSet::new(),
        view_fields,
        this: Some(this),
        next: 0,
        limit: 10,
        label: 0,
        resolve: &fallback,
        target: Some(target),
        string_index: Some(string_index),
        strings,
        persistence,
        preferences: preferences
            .iter()
            .map(|p| (p.name.clone(), p.clone()))
            .collect(),
        tables: tables.iter().map(|t| (t.name.clone(), t.clone())).collect(),
        outs: 0,
    };
    for handler in handlers {
        let is_spinner = ready_fields.contains_key(&handler.view);
        if is_spinner != spinner {
            continue;
        }
        lowerer.next = 0;
        let next = lowerer.label();
        let target_view = lowerer.alloc(Type::String)?;
        let view_field = *lowerer
            .view_fields
            .get(&handler.view)
            .ok_or(DexError::InvalidInput("missing selection view field"))?;
        lowerer.code.push(Instruction::IGet {
            dst: target_view,
            object: this,
            field: view_field,
        });
        lowerer.code.push(Instruction::IfNe {
            left: parent,
            right: target_view,
            target: next,
        });
        if spinner {
            let ready = lowerer.alloc(Type::Bool)?;
            let one = lowerer.alloc(Type::Bool)?;
            let arm = lowerer.label();
            let body = lowerer.label();
            lowerer.code.push(Instruction::IGet {
                dst: ready,
                object: this,
                field: ready_fields[&handler.view],
            });
            lowerer
                .code
                .push(Instruction::Const4 { dst: one, value: 1 });
            lowerer.code.push(Instruction::IfNe {
                left: ready,
                right: one,
                target: arm,
            });
            lowerer.code.push(Instruction::Goto16 { target: body });
            lowerer.code.push(Instruction::Label(arm));
            lowerer.code.push(Instruction::IPut {
                src: one,
                object: this,
                field: ready_fields[&handler.view],
            });
            lowerer.code.push(Instruction::ReturnVoid);
            lowerer.code.push(Instruction::Label(body));
        }
        let object = lowerer.alloc(Type::String)?;
        let value = lowerer.alloc(Type::String)?;
        lowerer.code.push(Instruction::InvokeVirtual {
            method: ui.adapter_view_get_item_at_position,
            args: vec![parent, position],
        });
        lowerer
            .code
            .push(Instruction::MoveResultObject { dst: object });
        lowerer.code.push(Instruction::InvokeVirtual {
            method: ui.object_to_string,
            args: vec![object],
        });
        lowerer
            .code
            .push(Instruction::MoveResultObject { dst: value });
        lowerer.locals.insert(
            handler.index.clone(),
            Binding {
                register: position,
                ty: Type::I32,
            },
        );
        lowerer.locals.insert(
            handler.value.clone(),
            Binding {
                register: value,
                ty: Type::String,
            },
        );
        lowerer.on_create_statements(&handler.body, this, ui)?;
        lowerer.code.push(Instruction::ReturnVoid);
        lowerer.code.push(Instruction::Label(next));
    }
    lowerer.code.push(Instruction::ReturnVoid);
    Ok(LoweredMethod {
        code: assemble(&lowerer.code)?,
        registers: 16,
        ins: 6,
        outs: lowerer.outs,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn lower_on_create(
    statements: &[Statement],
    variants: &[aic_ir::CreateVariant],
    activity_name: &str,
    target: &dyn Fn(&str) -> Result<FunctionTarget, DexError>,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    strings: Option<StringLowering>,
    ui: UiLowering,
    state_fields: BTreeMap<String, (u16, Type)>,
    collection_fields: BTreeMap<String, u16>,
    selection_views: BTreeMap<String, bool>,
    view_fields: BTreeMap<String, u16>,
    states: &[aic_ir::State],
    string_collections: &[aic_ir::StringCollectionState],
    lifecycle: Option<LifecycleLowering>,
    persistence: Option<PersistenceLowering>,
    preferences: &[Preference],
    tables: &[Table],
) -> Result<LoweredMethod, DexError> {
    let fallback = |name: &str| Ok(target(name)?.method);
    let touch_target_views = statements
        .iter()
        .chain(variants.iter().flat_map(|variant| variant.body.iter()))
        .filter_map(|statement| match &statement.kind {
            StatementKind::Button { id, .. }
            | StatementKind::EditText { id, .. }
            | StatementKind::TextInput { id, .. }
            | StatementKind::CheckBox { id, .. }
            | StatementKind::Switch { id, .. }
            | StatementKind::Toolbar { id, .. }
            | StatementKind::ListView { id, .. }
            | StatementKind::Spinner { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let this = Register {
        index: 14,
        kind: ValueKind::Reference,
    };
    let bundle = Register {
        index: 15,
        kind: ValueKind::Reference,
    };
    let mut lowerer = Lowerer {
        code: vec![Instruction::InvokeSuper {
            method: ui.activity_on_create,
            args: vec![this, bundle],
        }],
        locals: BTreeMap::new(),
        views: BTreeMap::new(),
        state_fields,
        collection_fields,
        selection_views,
        touch_target_views,
        view_fields,
        this: Some(this),
        next: 0,
        limit: 14,
        label: 0,
        resolve: &fallback,
        target: Some(target),
        string_index: Some(string_index),
        strings,
        persistence,
        preferences: preferences
            .iter()
            .map(|p| (p.name.clone(), p.clone()))
            .collect(),
        tables: tables.iter().map(|t| (t.name.clone(), t.clone())).collect(),
        outs: 2,
    };
    if let Some(p) = persistence {
        if let Some(field) = p.preferences_field {
            let name = lowerer.alloc(Type::String)?;
            let mode = lowerer.alloc(Type::I32)?;
            let object = lowerer.alloc(Type::String)?;
            lowerer.code.push(Instruction::ConstString {
                dst: name,
                string: string_index("aic.preferences")?,
            });
            lowerer.code.push(Instruction::Const4 {
                dst: mode,
                value: 0,
            });
            lowerer.code.push(Instruction::InvokeVirtual {
                method: p.get_shared_preferences,
                args: vec![this, name, mode],
            });
            lowerer
                .code
                .push(Instruction::MoveResultObject { dst: object });
            lowerer.code.push(Instruction::IPut {
                src: object,
                object: this,
                field,
            });
            lowerer.next = 0;
            lowerer.outs = lowerer.outs.max(3);
        }
        if let Some(field) = p.database_field {
            let database_name = tables.first().map_or("aic.db", |_| "aic.db");
            let name = lowerer.alloc(Type::String)?;
            let mode = lowerer.alloc(Type::I32)?;
            let factory = lowerer.alloc(Type::String)?;
            let object = lowerer.alloc(Type::String)?;
            lowerer.code.push(Instruction::ConstString {
                dst: name,
                string: string_index(database_name)?,
            });
            lowerer.code.push(Instruction::Const4 {
                dst: mode,
                value: 0,
            });
            lowerer.code.push(Instruction::ConstNull { dst: factory });
            lowerer.code.push(Instruction::InvokeVirtual {
                method: p.open_database,
                args: vec![this, name, mode, factory],
            });
            lowerer
                .code
                .push(Instruction::MoveResultObject { dst: object });
            lowerer.code.push(Instruction::IPut {
                src: object,
                object: this,
                field,
            });
            lowerer.outs = lowerer.outs.max(4);
            for table in tables {
                let sql = create_table_sql(table);
                let query = lowerer.alloc(Type::String)?;
                lowerer.code.push(Instruction::ConstString {
                    dst: query,
                    string: string_index(&sql)?,
                });
                lowerer.code.push(Instruction::InvokeVirtual {
                    method: p.database_exec_sql,
                    args: vec![object, query],
                });
                lowerer.next = object.index + 1;
            }
            lowerer.next = 0;
        }
    }
    for state in states {
        let register = lowerer.alloc(state.ty)?;
        lowerer.expr(&state.initial, register)?;
        let field = lowerer.state_fields[&state.name].0;
        lowerer.code.push(Instruction::IPut {
            src: register,
            object: this,
            field,
        });
        lowerer.next = 0;
    }
    for collection in string_collections {
        let size = lowerer.alloc(Type::I32)?;
        let array = lowerer.alloc(Type::String)?;
        lowerer.code.push(Instruction::Const16 {
            dst: size,
            value: i16::try_from(collection.items.len())
                .map_err(|_| DexError::ArithmeticOverflow)?,
        });
        lowerer.code.push(Instruction::NewArray {
            dst: array,
            size,
            ty: ui.string_array_type,
        });
        for (offset, item) in collection.items.iter().enumerate() {
            let value = lowerer.alloc(Type::String)?;
            let index = lowerer.alloc(Type::I32)?;
            lowerer.expr(item, value)?;
            lowerer.code.push(Instruction::Const16 {
                dst: index,
                value: i16::try_from(offset).map_err(|_| DexError::ArithmeticOverflow)?,
            });
            lowerer.code.push(Instruction::AputObject {
                value,
                array,
                index,
            });
            lowerer.release_temporaries(array.index + 1);
        }
        lowerer.code.push(Instruction::IPut {
            src: array,
            object: this,
            field: lowerer.collection_fields[&collection.name],
        });
        lowerer.next = 0;
    }
    if let Some(lifecycle) = lifecycle {
        let no_bundle = lowerer.label();
        let has_bundle = lowerer.label();
        let null = lowerer.alloc(Type::String)?;
        lowerer.code.push(Instruction::ConstNull { dst: null });
        lowerer.code.push(Instruction::IfNe {
            left: bundle,
            right: null,
            target: has_bundle,
        });
        lowerer.code.push(Instruction::Goto16 { target: no_bundle });
        lowerer.code.push(Instruction::Label(has_bundle));
        lowerer.next = 0;
        for state in states {
            let key = lowerer.alloc(Type::String)?;
            let present = lowerer.alloc(Type::Bool)?;
            let value = lowerer.alloc(state.ty)?;
            let next = lowerer.label();
            lowerer.code.push(Instruction::ConstString {
                dst: key,
                string: string_index(&format!("aic.state.{activity_name}.{}", state.name))?,
            });
            lowerer.code.push(Instruction::InvokeVirtual {
                method: lifecycle.bundle_contains_key,
                args: vec![bundle, key],
            });
            lowerer.code.push(Instruction::MoveResult { dst: present });
            lowerer.code.push(Instruction::IfEqz {
                value: present,
                target: next,
            });
            lowerer.code.push(Instruction::IGet {
                dst: value,
                object: this,
                field: lowerer.state_fields[&state.name].0,
            });
            lowerer.code.push(Instruction::InvokeVirtual {
                method: match state.ty {
                    Type::I32 => lifecycle.bundle_get_i32,
                    Type::Bool => lifecycle.bundle_get_bool,
                    Type::String => lifecycle.bundle_get_string,
                },
                args: vec![bundle, key, value],
            });
            lowerer.code.push(if state.ty == Type::String {
                Instruction::MoveResultObject { dst: value }
            } else {
                Instruction::MoveResult { dst: value }
            });
            lowerer.code.push(Instruction::IPut {
                src: value,
                object: this,
                field: lowerer.state_fields[&state.name].0,
            });
            lowerer.code.push(Instruction::Label(next));
            lowerer.next = 0;
        }
        lowerer.code.push(Instruction::Label(no_bundle));
    }
    lowerer.initialize_minimum_touch_target(this, ui)?;
    if variants.is_empty() {
        lowerer.on_create_statements(statements, this, ui)?;
    } else {
        let resources = lowerer.alloc(Type::String)?;
        let configuration = lowerer.alloc(Type::String)?;
        let orientation = lowerer.alloc(Type::I32)?;
        let width = lowerer.alloc(Type::I32)?;
        let portrait = lowerer.alloc(Type::I32)?;
        let landscape = lowerer.alloc(Type::I32)?;
        let breakpoint = lowerer.alloc(Type::I32)?;
        lowerer.code.push(Instruction::InvokeVirtual {
            method: ui.context_get_resources,
            args: vec![this],
        });
        lowerer
            .code
            .push(Instruction::MoveResultObject { dst: resources });
        lowerer.code.push(Instruction::InvokeVirtual {
            method: ui.resources_get_configuration,
            args: vec![resources],
        });
        lowerer
            .code
            .push(Instruction::MoveResultObject { dst: configuration });
        lowerer.code.push(Instruction::IGet {
            dst: orientation,
            object: configuration,
            field: ui
                .configuration_orientation
                .ok_or(DexError::InvalidInput("missing adaptive orientation field"))?,
        });
        lowerer.code.push(Instruction::IGet {
            dst: width,
            object: configuration,
            field: ui
                .configuration_screen_width_dp
                .ok_or(DexError::InvalidInput("missing adaptive width field"))?,
        });
        lowerer.code.push(Instruction::Const4 {
            dst: portrait,
            value: 1,
        });
        lowerer.code.push(Instruction::Const4 {
            dst: landscape,
            value: 2,
        });
        lowerer.code.push(Instruction::Const16 {
            dst: breakpoint,
            value: 600,
        });
        let end = lowerer.label();
        let arms = variants.iter().map(|_| lowerer.label()).collect::<Vec<_>>();
        for (variant, arm) in variants.iter().zip(&arms) {
            let next = lowerer.label();
            match variant.orientation {
                aic_ir::DeviceOrientation::Portrait => lowerer.code.push(Instruction::IfNe {
                    left: orientation,
                    right: portrait,
                    target: next,
                }),
                aic_ir::DeviceOrientation::Landscape => lowerer.code.push(Instruction::IfNe {
                    left: orientation,
                    right: landscape,
                    target: next,
                }),
            }
            match variant.window {
                aic_ir::WindowClass::Compact => lowerer.code.push(Instruction::IfLt {
                    left: width,
                    right: breakpoint,
                    target: *arm,
                }),
                aic_ir::WindowClass::Expanded => lowerer.code.push(Instruction::IfGe {
                    left: width,
                    right: breakpoint,
                    target: *arm,
                }),
            }
            lowerer.code.push(Instruction::Label(next));
        }
        lowerer.code.push(Instruction::Goto16 { target: arms[0] });
        for (variant, arm) in variants.iter().zip(arms) {
            lowerer.code.push(Instruction::Label(arm));
            lowerer.next = 0;
            lowerer.locals.clear();
            lowerer.views.clear();
            lowerer.on_create_statements(&variant.body, this, ui)?;
            lowerer.code.push(Instruction::Goto16 { target: end });
        }
        lowerer.code.push(Instruction::Label(end));
    }
    lowerer.code.push(Instruction::ReturnVoid);
    Ok(LoweredMethod {
        code: assemble(&lowerer.code)?,
        registers: 16,
        ins: 2,
        outs: lowerer.outs,
    })
}

pub fn lower_on_save_instance_state(
    states: &[aic_ir::State],
    activity_name: &str,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    state_fields: &BTreeMap<String, (u16, Type)>,
    lifecycle: LifecycleLowering,
) -> Result<LoweredMethod, DexError> {
    let this = Register {
        index: 14,
        kind: ValueKind::Reference,
    };
    let bundle = Register {
        index: 15,
        kind: ValueKind::Reference,
    };
    let mut code = vec![Instruction::InvokeSuper {
        method: lifecycle.activity_on_save_instance_state,
        args: vec![this, bundle],
    }];
    for state in states {
        let key = Register {
            index: 0,
            kind: ValueKind::Reference,
        };
        let value = Register {
            index: 1,
            kind: kind(state.ty),
        };
        code.push(Instruction::ConstString {
            dst: key,
            string: string_index(&format!("aic.state.{activity_name}.{}", state.name))?,
        });
        code.push(Instruction::IGet {
            dst: value,
            object: this,
            field: state_fields[&state.name].0,
        });
        code.push(Instruction::InvokeVirtual {
            method: match state.ty {
                Type::I32 => lifecycle.bundle_put_i32,
                Type::Bool => lifecycle.bundle_put_bool,
                Type::String => lifecycle.bundle_put_string,
            },
            args: vec![bundle, key, value],
        });
    }
    code.push(Instruction::ReturnVoid);
    Ok(LoweredMethod {
        code: assemble(&code)?,
        registers: 16,
        ins: 2,
        outs: 3,
    })
}

#[must_use]
pub fn create_table_sql(table: &Table) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        table.name,
        table
            .columns
            .iter()
            .map(|column| {
                let mut value = format!(
                    "{} {}",
                    column.name,
                    match column.ty {
                        Type::I32 | Type::Bool => "INTEGER",
                        Type::String => "TEXT",
                    }
                );
                if column.primary_key {
                    value.push_str(" PRIMARY KEY AUTOINCREMENT");
                } else {
                    value.push_str(" NOT NULL");
                }
                value
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aic_ir::parse_program;
    #[test]
    fn boolean_and_branches_before_rhs_division() {
        let p=parse_program("aic_version 0.1 app \"x\" package \"dev.aic.x\" { fn safe(flag: bool, divisor: i32) -> bool { return flag && 10 / divisor > 1 } activity MainActivity { on_create { let value: bool = safe(false, 0) let root = android.linear_layout(orientation: vertical) let message = android.text_view(text: string(value)) android.add_view(parent: root, child: message) android.set_content_view(root) } } }").unwrap();
        let m = lower_function(&p.functions[0], &|_| Ok(0)).unwrap();
        let b = m.code.iter().position(|w| w & 255 == 0x38).unwrap();
        let d = m.code.iter().position(|w| w & 255 == 0x93).unwrap();
        assert!(b < d)
    }

    #[test]
    fn lowers_runtime_string_conversion_concat_and_reference_return() {
        let source = "aic_version 0.1 app \"x\" package \"dev.aic.x\" { fn render(value: i32) -> string { return \"Value: \" + string(value) } activity MainActivity { on_create { let text: string = render(7) let root = android.linear_layout(orientation: vertical) let message = android.text_view(text: text) android.add_view(parent: root, child: message) android.set_content_view(root) } } }";
        let program = parse_program(source).unwrap();
        let method = lower_function_typed(
            &program.functions[0],
            &|_| Err(DexError::InvalidInput("unexpected user call")),
            &|value| {
                if value == "Value: " {
                    Ok(3)
                } else {
                    Err(DexError::InvalidInput("unknown string"))
                }
            },
            StringLowering {
                value_of_i32: 10,
                value_of_bool: 16,
                builder_type: 11,
                builder_init: 12,
                builder_append: 13,
                builder_to_string: 14,
                equals: 15,
                text_view_get_text: 17,
                object_to_string: 18,
                string_matches: 19,
                integer_parse_int: 20,
                context_get_string: 21,
            },
        )
        .unwrap();
        assert!(method.code.iter().any(|word| word & 0xff == 0x1a));
        assert!(method.code.iter().any(|word| word & 0xff == 0x22));
        assert!(method.code.iter().any(|word| word & 0xff == 0x11));
    }
}

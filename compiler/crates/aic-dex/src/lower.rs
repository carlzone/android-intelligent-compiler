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
    BinaryOp, Expression, ExpressionKind, Function, Statement, StatementKind, Type, UnaryOp, Value,
};
use std::collections::BTreeMap;

pub(crate) const VALID_I32_PATTERN: &str = "(?:[+-]?0*[0-9]{1,9}|[+]?0*(?:1[0-9]{9}|20[0-9]{8}|21[0-3][0-9]{7}|214[0-6][0-9]{6}|2147[0-3][0-9]{5}|21474[0-7][0-9]{4}|214748[0-2][0-9]{3}|2147483[0-5][0-9]{2}|21474836[0-3][0-9]|214748364[0-7])|-0*(?:1[0-9]{9}|20[0-9]{8}|21[0-3][0-9]{7}|214[0-6][0-9]{6}|2147[0-3][0-9]{5}|21474[0-7][0-9]{4}|214748[0-2][0-9]{3}|2147483[0-5][0-9]{2}|21474836[0-3][0-9]|214748364[0-8]))";

pub struct LoweredMethod {
    pub code: Vec<u16>,
    pub registers: u16,
    pub ins: u16,
    pub outs: u16,
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
}
#[derive(Clone, Copy, Debug)]
pub struct UiLowering {
    pub activity_on_create: u16,
    pub linear_layout_type: u16,
    pub linear_layout_init: u16,
    pub linear_layout_orientation: u16,
    pub text_view_type: u16,
    pub text_view_init: u16,
    pub text_view_set_text: u16,
    pub button_type: u16,
    pub button_init: u16,
    pub edit_text_type: u16,
    pub edit_text_init: u16,
    pub edit_text_set_hint: u16,
    pub edit_text_set_input_type: u16,
    pub scroll_view_type: u16,
    pub scroll_view_init: u16,
    pub set_on_click_listener: u16,
    pub layout_params_type: u16,
    pub layout_params_init: u16,
    pub set_layout_params: u16,
    pub color_parse: u16,
    pub set_text_color: u16,
    pub set_background_color: u16,
    pub add_view: u16,
    pub set_content_view: u16,
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
    view_fields: BTreeMap<String, u16>,
    this: Option<Register>,
    next: u8,
    limit: u8,
    label: u16,
    resolve: &'a dyn Fn(&str) -> Result<u16, DexError>,
    target: Option<&'a dyn Fn(&str) -> Result<FunctionTarget, DexError>>,
    string_index: Option<&'a dyn Fn(&str) -> Result<u16, DexError>>,
    strings: Option<StringLowering>,
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
            ExpressionKind::AndroidText { .. } => Ok(Type::String),
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
        }
        Ok(())
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
        view_fields: BTreeMap::new(),
        this: None,
        next: 0,
        limit: first,
        label: 0,
        resolve,
        target: None,
        string_index: None,
        strings: None,
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
        view_fields: BTreeMap::new(),
        this: None,
        next: 0,
        limit: first,
        label: 0,
        resolve: &fallback,
        target: Some(target),
        string_index: Some(string_index),
        strings: Some(strings),
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
                        self.next = register.index;
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
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.next = view.index + 1;
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
                    self.views.insert(id.clone(), view);
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.next = view.index + 1;
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
                    self.next = rendered.index;
                    self.views.insert(id.clone(), view);
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
                    self.next = view.index + 1;
                }
                StatementKind::EditText { id, hint } => {
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
                    self.code.push(Instruction::Const4 {
                        dst: input_type,
                        value: 2,
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.edit_text_set_input_type,
                        args: vec![view, input_type],
                    });
                    self.outs = self.outs.max(2);
                    self.views.insert(id.clone(), view);
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
                    self.next = view.index + 1;
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
                    if let Some(field) = self.view_fields.get(id) {
                        self.code.push(Instruction::IPut {
                            src: view,
                            object: this,
                            field: *field,
                        });
                    }
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
                    self.next = rendered.index;
                }
                StatementKind::SetLayout {
                    view,
                    width,
                    height,
                    weight,
                } => {
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
                        self.code.push(Instruction::Const4 {
                            dst: register,
                            value: if *size == aic_ir::LayoutSize::MatchParent {
                                -1
                            } else {
                                -2
                            },
                        });
                    }
                    self.code.push(Instruction::Const32 {
                        dst: weight_register,
                        value: if *weight == 0 { 0 } else { 0x3f80_0000 },
                    });
                    self.code.push(Instruction::InvokeDirect {
                        method: ui.layout_params_init,
                        args: vec![params, w, h, weight_register],
                    });
                    self.code.push(Instruction::InvokeVirtual {
                        method: ui.set_layout_params,
                        args: vec![view, params],
                    });
                    self.outs = self.outs.max(4);
                    self.next = params.index;
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
                    self.next = text.index;
                }
                StatementKind::Return(_) => {
                    return Err(DexError::InvalidInput("return inside onCreate"))
                }
            }
        }
        Ok(())
    }
}

pub fn lower_on_click(
    handlers: &[aic_ir::ClickHandler],
    target: &dyn Fn(&str) -> Result<FunctionTarget, DexError>,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    strings: Option<StringLowering>,
    ui: UiLowering,
    state_fields: BTreeMap<String, (u16, Type)>,
    view_fields: BTreeMap<String, u16>,
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
        view_fields,
        this: Some(this),
        next: 0,
        limit: 14,
        label: 0,
        resolve: &fallback,
        target: Some(target),
        string_index: Some(string_index),
        strings,
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
pub fn lower_on_create(
    statements: &[Statement],
    target: &dyn Fn(&str) -> Result<FunctionTarget, DexError>,
    string_index: &dyn Fn(&str) -> Result<u16, DexError>,
    strings: Option<StringLowering>,
    ui: UiLowering,
    state_fields: BTreeMap<String, (u16, Type)>,
    view_fields: BTreeMap<String, u16>,
    states: &[aic_ir::State],
) -> Result<LoweredMethod, DexError> {
    let fallback = |name: &str| Ok(target(name)?.method);
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
        view_fields,
        this: Some(this),
        next: 0,
        limit: 14,
        label: 0,
        resolve: &fallback,
        target: Some(target),
        string_index: Some(string_index),
        strings,
        outs: 2,
    };
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
    lowerer.on_create_statements(statements, this, ui)?;
    lowerer.code.push(Instruction::ReturnVoid);
    Ok(LoweredMethod {
        code: assemble(&lowerer.code)?,
        registers: 16,
        ins: 2,
        outs: lowerer.outs,
    })
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
            },
        )
        .unwrap();
        assert!(method.code.iter().any(|word| word & 0xff == 0x1a));
        assert!(method.code.iter().any(|word| word & 0xff == 0x22));
        assert!(method.code.iter().any(|word| word & 0xff == 0x11));
    }
}

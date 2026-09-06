//! Semantics-preserving optimizations for verified AIC IR.
#![allow(clippy::enum_glob_use, clippy::many_single_char_names)]
use aic_ir::{
    BinaryOp, Expression, ExpressionKind, Program, Statement, StatementKind, UnaryOp, Value,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OptimizationLevel {
    None,
    #[default]
    Basic,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompilerOptions {
    pub optimization_level: OptimizationLevel,
}

#[must_use]
pub fn optimize(mut program: Program, options: CompilerOptions) -> Program {
    if options.optimization_level == OptimizationLevel::None {
        return program;
    }
    for function in &mut program.functions {
        function.body = statements(std::mem::take(&mut function.body));
    }
    program.activity.on_create = statements(std::mem::take(&mut program.activity.on_create));
    for handler in &mut program.activity.on_click {
        handler.body = statements(std::mem::take(&mut handler.body));
    }
    program
}
fn statements(body: Vec<Statement>) -> Vec<Statement> {
    body.into_iter()
        .flat_map(|mut s| {
            s.kind = match s.kind {
                StatementKind::Declare {
                    mutable,
                    name,
                    ty,
                    value,
                } => StatementKind::Declare {
                    mutable,
                    name,
                    ty,
                    value: expression(value),
                },
                StatementKind::Assign { name, value } => StatementKind::Assign {
                    name,
                    value: expression(value),
                },
                StatementKind::Return(value) => StatementKind::Return(expression(value)),
                StatementKind::TextView { id, text } => StatementKind::TextView {
                    id,
                    text: expression(text),
                },
                StatementKind::Button { id, text } => StatementKind::Button {
                    id,
                    text: expression(text),
                },
                StatementKind::EditText { id, hint } => StatementKind::EditText {
                    id,
                    hint: expression(hint),
                },
                StatementKind::SetText { view, text } => StatementKind::SetText {
                    view,
                    text: expression(text),
                },
                StatementKind::For {
                    variable,
                    start,
                    end,
                    body,
                } => StatementKind::For {
                    variable,
                    start: expression(start),
                    end: expression(end),
                    body: statements(body),
                },
                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let condition = expression(condition);
                    if let ExpressionKind::Literal(Value::Bool(value)) = condition.kind {
                        return if value {
                            statements(then_body)
                        } else {
                            statements(else_body)
                        };
                    }
                    StatementKind::If {
                        condition,
                        then_body: statements(then_body),
                        else_body: statements(else_body),
                    }
                }
                other => other,
            };
            vec![s]
        })
        .collect()
}
fn expression(mut e: Expression) -> Expression {
    e.kind = match e.kind {
        ExpressionKind::Unary { op, value } => {
            let value = expression(*value);
            if let Some(v) = fold_unary(op, &value.kind) {
                ExpressionKind::Literal(v)
            } else {
                ExpressionKind::Unary {
                    op,
                    value: Box::new(value),
                }
            }
        }
        ExpressionKind::Binary { op, left, right } => {
            let left = expression(*left);
            if op == BinaryOp::And && left.kind == ExpressionKind::Literal(Value::Bool(false)) {
                return Expression {
                    kind: left.kind,
                    span: e.span,
                };
            }
            if op == BinaryOp::Or && left.kind == ExpressionKind::Literal(Value::Bool(true)) {
                return Expression {
                    kind: left.kind,
                    span: e.span,
                };
            }
            let right = expression(*right);
            if let Some(v) = fold_binary(op, &left.kind, &right.kind) {
                ExpressionKind::Literal(v)
            } else {
                ExpressionKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
        }
        ExpressionKind::Call { name, args } => ExpressionKind::Call {
            name,
            args: args.into_iter().map(expression).collect(),
        },
        ExpressionKind::AndroidText { view } => ExpressionKind::AndroidText { view },
        other => other,
    };
    e
}
fn fold_unary(op: UnaryOp, v: &ExpressionKind) -> Option<Value> {
    match (op, v) {
        (UnaryOp::Negate, ExpressionKind::Literal(Value::I32(x))) => {
            Some(Value::I32(x.wrapping_neg()))
        }
        (UnaryOp::Not, ExpressionKind::Literal(Value::Bool(x))) => Some(Value::Bool(!x)),
        _ => None,
    }
}
fn fold_binary(op: BinaryOp, l: &ExpressionKind, r: &ExpressionKind) -> Option<Value> {
    use BinaryOp::*;
    let (ExpressionKind::Literal(a), ExpressionKind::Literal(b)) = (l, r) else {
        return None;
    };
    Some(match (op, a, b) {
        (Add, Value::I32(x), Value::I32(y)) => Value::I32(x.wrapping_add(*y)),
        (Subtract, Value::I32(x), Value::I32(y)) => Value::I32(x.wrapping_sub(*y)),
        (Multiply, Value::I32(x), Value::I32(y)) => Value::I32(x.wrapping_mul(*y)),
        (Divide, Value::I32(x), Value::I32(y)) if *y != 0 => {
            Value::I32(x.checked_div(*y).unwrap_or(i32::MIN))
        }
        (Remainder, Value::I32(x), Value::I32(y)) if *y != 0 => {
            Value::I32(x.checked_rem(*y).unwrap_or(0))
        }
        (Add, Value::String(x), Value::String(y)) => Value::String(x.clone() + y),
        (Equal, x, y) => Value::Bool(x == y),
        (NotEqual, x, y) => Value::Bool(x != y),
        (Less, Value::I32(x), Value::I32(y)) => Value::Bool(x < y),
        (LessEqual, Value::I32(x), Value::I32(y)) => Value::Bool(x <= y),
        (Greater, Value::I32(x), Value::I32(y)) => Value::Bool(x > y),
        (GreaterEqual, Value::I32(x), Value::I32(y)) => Value::Bool(x >= y),
        (And, Value::Bool(x), Value::Bool(y)) => Value::Bool(*x && *y),
        (Or, Value::Bool(x), Value::Bool(y)) => Value::Bool(*x || *y),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aic_ir::{evaluate_text, parse_program};
    const IR:&str="aic_version 0.1 app \"x\" package \"dev.aic.x\" { activity MainActivity { on_create { let root = android.linear_layout(orientation: vertical) let message = android.text_view(text: \"Result: \" + string(1 + 2 * 3)) android.add_view(parent: root, child: message) android.set_content_view(root) } } }";
    #[test]
    fn differential() {
        let p = parse_program(IR).unwrap();
        let q = optimize(p.clone(), CompilerOptions::default());
        assert_eq!(evaluate_text(&p).unwrap(), evaluate_text(&q).unwrap());
    }
}

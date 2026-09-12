//! Whole-program, semantics-preserving optimizations for verified AIC IR.
#![allow(clippy::enum_glob_use, clippy::many_single_char_names)]
use aic_ir::{
    BinaryOp, Capability, CollectionItems, Expression, ExpressionKind, Program, Statement,
    StatementKind, UnaryOp, Value,
};
use std::collections::{BTreeMap, BTreeSet};

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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Reachability {
    pub functions: BTreeSet<String>,
    pub capabilities: BTreeSet<Capability>,
    pub views: BTreeSet<String>,
    pub preferences: BTreeSet<String>,
    pub tables: BTreeSet<String>,
    pub columns: BTreeMap<String, BTreeSet<String>>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OptimizationReport {
    pub before: Reachability,
    pub after: Reachability,
    pub functions_before: usize,
    pub functions_after: usize,
    pub preferences_before: usize,
    pub preferences_after: usize,
    pub tables_before: usize,
    pub tables_after: usize,
}
impl OptimizationReport {
    #[must_use]
    pub fn removed_functions(&self) -> usize {
        self.functions_before.saturating_sub(self.functions_after)
    }
    #[must_use]
    pub fn removed_resources(&self) -> usize {
        self.preferences_before
            .saturating_sub(self.preferences_after)
            + self.tables_before.saturating_sub(self.tables_after)
    }
}

#[must_use]
pub fn optimize(program: Program, options: CompilerOptions) -> Program {
    optimize_with_report(program, options).0
}
#[must_use]
pub fn optimize_with_report(
    mut p: Program,
    options: CompilerOptions,
) -> (Program, OptimizationReport) {
    let mut r = OptimizationReport {
        before: analyze(&p),
        functions_before: p.functions.len(),
        preferences_before: p.preferences.len(),
        tables_before: p.database.as_ref().map_or(0, |d| d.tables.len()),
        ..Default::default()
    };
    if options.optimization_level != OptimizationLevel::None {
        for f in &mut p.functions {
            f.body = statements(std::mem::take(&mut f.body));
        }
        for activity in &mut p.activities {
            for s in &mut activity.state {
                s.initial = expression(s.initial.clone());
            }
            activity.on_create = statements(std::mem::take(&mut activity.on_create));
            for h in &mut activity.on_click {
                h.body = statements(std::mem::take(&mut h.body));
            }
            for h in &mut activity.on_select {
                h.body = statements(std::mem::take(&mut h.body));
            }
        }
        p.activity = p.activities[0].clone();
        let reachable = reachable_functions(&p);
        p.functions.retain(|f| reachable.contains(&f.name));
        let used = analyze(&p);
        p.capabilities.clone_from(&used.capabilities);
        p.preferences.retain(|x| used.preferences.contains(&x.name));
        if let Some(db) = &mut p.database {
            db.tables.retain(|x| used.tables.contains(&x.name));
            if db.tables.is_empty() {
                p.database = None;
            }
        }
    }
    r.after = analyze(&p);
    r.functions_after = p.functions.len();
    r.preferences_after = p.preferences.len();
    r.tables_after = p.database.as_ref().map_or(0, |d| d.tables.len());
    (p, r)
}
#[must_use]
pub fn analyze(p: &Program) -> Reachability {
    let mut r = Reachability::default();
    roots(p, &mut r);
    r.functions = reachable_functions(p);
    for name in r.functions.clone() {
        if let Some(f) = p.functions.iter().find(|f| f.name == name) {
            inspect_statements(&f.body, &mut r);
        }
    }
    r
}
fn roots(p: &Program, r: &mut Reachability) {
    for activity in &p.activities {
        for s in &activity.state {
            inspect_expression(&s.initial, r);
        }
        inspect_statements(&activity.on_create, r);
        for h in &activity.on_click {
            inspect_statements(&h.body, r);
        }
        for h in &activity.on_select {
            inspect_statements(&h.body, r);
        }
    }
}
fn reachable_functions(p: &Program) -> BTreeSet<String> {
    let map: BTreeMap<_, _> = p.functions.iter().map(|f| (f.name.as_str(), f)).collect();
    let mut u = Reachability::default();
    roots(p, &mut u);
    let mut out = BTreeSet::new();
    let mut pending: Vec<_> = u.functions.into_iter().collect();
    while let Some(n) = pending.pop() {
        if !out.insert(n.clone()) {
            continue;
        }
        if let Some(f) = map.get(n.as_str()) {
            let mut x = Reachability::default();
            inspect_statements(&f.body, &mut x);
            pending.extend(x.functions);
        }
    }
    out
}

fn statements(body: Vec<Statement>) -> Vec<Statement> {
    let mut out = Vec::new();
    for mut s in body {
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
            StatementKind::Return(v) => StatementKind::Return(expression(v)),
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
            StatementKind::TextInput {
                id,
                hint,
                input_type,
            } => StatementKind::TextInput {
                id,
                hint: expression(hint),
                input_type,
            },
            StatementKind::SetText { view, text } => StatementKind::SetText {
                view,
                text: expression(text),
            },
            StatementKind::PreferenceSet { key, value } => StatementKind::PreferenceSet {
                key,
                value: expression(value),
            },
            StatementKind::DatabaseUpdate { table, id, values } => StatementKind::DatabaseUpdate {
                table,
                id: expression(id),
                values: values
                    .into_iter()
                    .map(|(n, v)| (n, expression(v)))
                    .collect(),
            },
            StatementKind::DatabaseDelete { table, id } => StatementKind::DatabaseDelete {
                table,
                id: expression(id),
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
                let c = expression(condition);
                if let ExpressionKind::Literal(Value::Bool(v)) = c.kind {
                    out.extend(statements(if v { then_body } else { else_body }));
                    if out.last().is_some_and(terminal) {
                        break;
                    }
                    continue;
                }
                StatementKind::If {
                    condition: c,
                    then_body: statements(then_body),
                    else_body: statements(else_body),
                }
            }
            other => other,
        };
        let end = terminal(&s);
        out.push(s);
        if end {
            break;
        }
    }
    out
}
fn terminal(s: &Statement) -> bool {
    match &s.kind {
        StatementKind::Return(_) => true,
        StatementKind::If {
            then_body,
            else_body,
            ..
        } => {
            !then_body.is_empty()
                && !else_body.is_empty()
                && then_body.last().is_some_and(terminal)
                && else_body.last().is_some_and(terminal)
        }
        _ => false,
    }
}

fn expression(mut e: Expression) -> Expression {
    e.kind = match e.kind {
        ExpressionKind::Unary { op, value } => {
            let v = expression(*value);
            fold_unary(op, &v.kind).map_or_else(
                || ExpressionKind::Unary {
                    op,
                    value: Box::new(v),
                },
                ExpressionKind::Literal,
            )
        }
        ExpressionKind::Binary { op, left, right } => {
            let l = expression(*left);
            if (op == BinaryOp::And && l.kind == ExpressionKind::Literal(Value::Bool(false)))
                || (op == BinaryOp::Or && l.kind == ExpressionKind::Literal(Value::Bool(true)))
            {
                return Expression {
                    kind: l.kind,
                    span: e.span,
                };
            }
            let r = expression(*right);
            fold_binary(op, &l.kind, &r.kind).map_or_else(
                || ExpressionKind::Binary {
                    op,
                    left: Box::new(l),
                    right: Box::new(r),
                },
                ExpressionKind::Literal,
            )
        }
        ExpressionKind::Call { name, args } => ExpressionKind::Call {
            name,
            args: args.into_iter().map(expression).collect(),
        },
        ExpressionKind::DatabaseInsert { table, values } => ExpressionKind::DatabaseInsert {
            table,
            values: values
                .into_iter()
                .map(|(n, v)| (n, expression(v)))
                .collect(),
        },
        ExpressionKind::DatabaseExists { table, id } => ExpressionKind::DatabaseExists {
            table,
            id: Box::new(expression(*id)),
        },
        ExpressionKind::DatabaseGet {
            table,
            id,
            column,
            default,
        } => ExpressionKind::DatabaseGet {
            table,
            id: Box::new(expression(*id)),
            column,
            default: Box::new(expression(*default)),
        },
        other => other,
    };
    e
}

#[allow(clippy::too_many_lines)]
fn inspect_statements(xs: &[Statement], r: &mut Reachability) {
    for s in xs {
        match &s.kind {
            StatementKind::Declare { value, .. }
            | StatementKind::Assign { value, .. }
            | StatementKind::Return(value)
            | StatementKind::SetText { text: value, .. } => inspect_expression(value, r),
            StatementKind::SetEnabled { enabled: value, .. }
            | StatementKind::SetContentDescription { text: value, .. } => {
                inspect_expression(value, r);
            }
            StatementKind::TextView { id, text }
            | StatementKind::Button { id, text }
            | StatementKind::EditText { id, hint: text }
            | StatementKind::TextInput { id, hint: text, .. }
            | StatementKind::CheckBox { id, text }
            | StatementKind::Switch { id, text } => {
                r.views.insert(id.clone());
                inspect_expression(text, r);
            }
            StatementKind::Toolbar { id, title } => {
                r.views.insert(id.clone());
                inspect_expression(title, r);
            }
            StatementKind::ListView { id, items } | StatementKind::Spinner { id, items } => {
                r.views.insert(id.clone());
                if let CollectionItems::Inline(items) = items {
                    for item in items {
                        inspect_expression(item, r);
                    }
                }
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                inspect_expression(condition, r);
                inspect_statements(then_body, r);
                inspect_statements(else_body, r);
            }
            StatementKind::For {
                start, end, body, ..
            } => {
                inspect_expression(start, r);
                inspect_expression(end, r);
                inspect_statements(body, r);
            }
            StatementKind::LinearLayout { id, .. }
            | StatementKind::ScrollView { id }
            | StatementKind::FrameLayout { id }
            | StatementKind::ProgressBar { id }
            | StatementKind::ImageView { id, .. }
            | StatementKind::SetContentView { view: id }
            | StatementKind::SetLayout { view: id, .. }
            | StatementKind::SetTextSize { view: id, .. }
            | StatementKind::SetHeading { view: id }
            | StatementKind::SetDecorative { view: id }
            | StatementKind::SetTextColor { view: id, .. }
            | StatementKind::SetBackgroundColor { view: id, .. } => {
                r.views.insert(id.clone());
            }
            StatementKind::SetPadding { view, .. }
            | StatementKind::SetVisibility { view, .. }
            | StatementKind::SetGravity { view, .. } => {
                r.views.insert(view.clone());
            }
            StatementKind::SetInputLabel { label, input } => {
                r.views.insert(label.clone());
                r.views.insert(input.clone());
            }
            StatementKind::AddView { parent, child } => {
                r.views.insert(parent.clone());
                r.views.insert(child.clone());
            }
            StatementKind::PreferenceSet { key, value } => {
                r.capabilities.insert(Capability::KeyValue);
                r.preferences.insert(key.clone());
                inspect_expression(value, r);
            }
            StatementKind::DatabaseUpdate { table, id, values } => {
                database_values(table, values, r);
                inspect_expression(id, r);
            }
            StatementKind::DatabaseDelete { table, id } => {
                r.capabilities.insert(Capability::Sqlite);
                r.tables.insert(table.clone());
                inspect_expression(id, r);
            }
            StatementKind::StartActivity { extras, .. } => {
                for (_, value) in extras {
                    inspect_expression(value, r);
                }
            }
            StatementKind::FinishActivity => {}
            StatementKind::ShowDialog { title, message } => {
                inspect_expression(title, r);
                inspect_expression(message, r);
            }
            StatementKind::ShowMenu { anchor, item } => {
                r.views.insert(anchor.clone());
                inspect_expression(item, r);
            }
        }
    }
}
fn database_values(table: &str, values: &[(String, Expression)], r: &mut Reachability) {
    r.capabilities.insert(Capability::Sqlite);
    r.tables.insert(table.into());
    for (c, v) in values {
        r.columns.entry(table.into()).or_default().insert(c.clone());
        inspect_expression(v, r);
    }
}
fn inspect_expression(e: &Expression, r: &mut Reachability) {
    match &e.kind {
        ExpressionKind::Unary { value, .. } => inspect_expression(value, r),
        ExpressionKind::Binary { left, right, .. } => {
            inspect_expression(left, r);
            inspect_expression(right, r);
        }
        ExpressionKind::Call { name, args } => {
            r.functions.insert(name.clone());
            for a in args {
                inspect_expression(a, r);
            }
        }
        ExpressionKind::AndroidText { view } => {
            r.views.insert(view.clone());
        }
        ExpressionKind::PreferenceGet { key } => {
            r.capabilities.insert(Capability::KeyValue);
            r.preferences.insert(key.clone());
        }
        ExpressionKind::DatabaseInsert { table, values } => database_values(table, values, r),
        ExpressionKind::DatabaseExists { table, id } => {
            r.capabilities.insert(Capability::Sqlite);
            r.tables.insert(table.clone());
            inspect_expression(id, r);
        }
        ExpressionKind::DatabaseGet {
            table,
            id,
            column,
            default,
        } => {
            r.capabilities.insert(Capability::Sqlite);
            r.tables.insert(table.clone());
            r.columns
                .entry(table.clone())
                .or_default()
                .insert(column.clone());
            inspect_expression(id, r);
            inspect_expression(default, r);
        }
        ExpressionKind::Literal(_) | ExpressionKind::Name(_) => {}
    }
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
    const IR:&str="aic_version 0.1 app \"x\" package \"dev.aic.x\" { fn used(x: i32) -> i32 { return x + 1 } fn transit(x: i32) -> i32 { return used(x) } fn unused() -> string { return \"dead\" + \" string\" } activity MainActivity { on_create { let root = android.linear_layout(orientation: vertical) let message = android.text_view(text: \"Result: \" + string(transit(1 + 2 * 3))) if false { android.set_text(view: message, text: \"unreachable\") } else { android.set_text(view: message, text: \"Result: \" + string(transit(7))) } android.add_view(parent: root, child: message) android.set_content_view(root) } } }";
    #[test]
    fn differential_reachability() {
        let p = parse_program(IR).unwrap();
        let (q, r) = optimize_with_report(p.clone(), CompilerOptions::default());
        assert_eq!(evaluate_text(&p).unwrap(), evaluate_text(&q).unwrap());
        assert_eq!(
            q.functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["used", "transit"]
        );
        assert_eq!(r.removed_functions(), 1);
        assert!(!format!("{q:?}").contains("unreachable"));
    }
    #[test]
    fn o0_identity_and_deterministic_report() {
        let p = parse_program(IR).unwrap();
        let o = CompilerOptions {
            optimization_level: OptimizationLevel::None,
        };
        let (q, a) = optimize_with_report(p.clone(), o);
        let (_, b) = optimize_with_report(p.clone(), o);
        assert_eq!(p, q);
        assert_eq!(a, b);
        assert_eq!(a.removed_functions(), 0);
    }

    #[test]
    fn m9_text_sizes_survive_o0_and_o1() {
        let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
        for options in [
            CompilerOptions {
                optimization_level: OptimizationLevel::None,
            },
            CompilerOptions::default(),
        ] {
            let optimized = optimize(program.clone(), options);
            assert!(optimized
                .activities
                .iter()
                .flat_map(|activity| &activity.on_create)
                .any(|statement| matches!(
                    statement.kind,
                    StatementKind::SetTextSize { size_sp: 24, .. }
                )));
        }
    }

    #[test]
    fn m9_literal_colors_survive_o0_and_o1() {
        let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
        for options in [
            CompilerOptions {
                optimization_level: OptimizationLevel::None,
            },
            CompilerOptions::default(),
        ] {
            let optimized = optimize(program.clone(), options);
            let statements = optimized
                .activities
                .iter()
                .flat_map(|activity| &activity.on_create)
                .collect::<Vec<_>>();
            assert!(statements.iter().any(|statement| matches!(
                &statement.kind,
                StatementKind::SetTextColor { view, color }
                    if view == "title" && color == "#202124"
            )));
            assert!(statements.iter().any(|statement| matches!(
                &statement.kind,
                StatementKind::SetBackgroundColor { view, color }
                    if view == "root" && color == "#E8F0FE"
            )));
        }
    }

    #[test]
    fn m9_touch_target_controls_survive_o0_and_o1() {
        let program = parse_program(include_str!("../../../testdata/m9-navigation.aic")).unwrap();
        for options in [
            CompilerOptions {
                optimization_level: OptimizationLevel::None,
            },
            CompilerOptions::default(),
        ] {
            let optimized = optimize(program.clone(), options);
            let statements = optimized
                .activities
                .iter()
                .flat_map(|activity| &activity.on_create)
                .collect::<Vec<_>>();
            for predicate in [
                |statement: &&aic_ir::Statement| {
                    matches!(statement.kind, StatementKind::Button { .. })
                },
                |statement: &&aic_ir::Statement| {
                    matches!(statement.kind, StatementKind::TextInput { .. })
                },
                |statement: &&aic_ir::Statement| {
                    matches!(statement.kind, StatementKind::CheckBox { .. })
                },
                |statement: &&aic_ir::Statement| {
                    matches!(statement.kind, StatementKind::Switch { .. })
                },
                |statement: &&aic_ir::Statement| {
                    matches!(statement.kind, StatementKind::Toolbar { .. })
                },
                |statement: &&aic_ir::Statement| {
                    matches!(statement.kind, StatementKind::ListView { .. })
                },
                |statement: &&aic_ir::Statement| {
                    matches!(statement.kind, StatementKind::Spinner { .. })
                },
            ] {
                assert!(statements.iter().any(predicate));
            }
            assert!(statements.iter().any(|statement| matches!(
                statement.kind,
                StatementKind::SetLayout {
                    height: aic_ir::LayoutSize::Dp(48),
                    ..
                }
            )));
        }
    }

    #[test]
    fn folds_away_and_prunes_dead_persistence_resources() {
        let source = "aic_version 0.1 app \"x\" package \"dev.aic.x\" { capability persistence.key_value preference legacy: bool = false activity MainActivity { on_create { let root = android.linear_layout(orientation: vertical) if 1 + 1 == 3 { preference.set(legacy, true) } android.set_content_view(root) } } }";
        let p = parse_program(source).unwrap();
        let (q, report) = optimize_with_report(p, CompilerOptions::default());
        assert!(q.capabilities.is_empty());
        assert!(q.preferences.is_empty());
        assert_eq!(report.removed_resources(), 1);
    }

    #[test]
    fn supported_evaluator_corpus_is_differential() {
        for source in [
            include_str!("../../../testdata/hello.aic"),
            include_str!("../../../testdata/compute.aic"),
            include_str!("../../../testdata/dynamic-string.aic"),
            include_str!("../../../testdata/multi-function.aic"),
            include_str!("../../../testdata/short-circuit.aic"),
            include_str!("../../../testdata/string-equality.aic"),
        ] {
            let original = parse_program(source).unwrap();
            let optimized = optimize(original.clone(), CompilerOptions::default());
            assert_eq!(
                evaluate_text(&original).unwrap(),
                evaluate_text(&optimized).unwrap()
            );
        }
    }
}

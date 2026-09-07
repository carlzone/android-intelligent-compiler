//! Source-located parser and verifier for the AIC M2 language.
#![allow(
    clippy::enum_glob_use,
    clippy::many_single_char_names,
    clippy::missing_errors_doc,
    clippy::semicolon_if_nothing_returned,
    clippy::too_many_lines,
    clippy::unnested_or_patterns,
    clippy::while_let_loop
)]
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: SourceLocation,
    pub end: SourceLocation,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub location: Option<SourceSpan>,
    pub message: String,
}
pub type IrError = Diagnostic;
impl Diagnostic {
    fn at(code: &'static str, s: SourceSpan, m: impl Into<String>) -> Self {
        Self {
            code,
            location: Some(s),
            message: m.into(),
        }
    }
    fn global(code: &'static str, m: impl Into<String>) -> Self {
        Self {
            code,
            location: None,
            message: m.into(),
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = self.location {
            write!(
                f,
                "{} at {}:{}: {}",
                self.code, s.start.line, s.start.column, self.message
            )
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}
impl Error for Diagnostic {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimalClass {
    descriptor: String,
}
impl MinimalClass {
    pub fn new(v: impl Into<String>) -> Result<Self, Diagnostic> {
        let v = v.into();
        let b = v
            .strip_prefix('L')
            .and_then(|x| x.strip_suffix(';'))
            .ok_or_else(|| {
                Diagnostic::global("AIC0001", format!("invalid DEX class descriptor: {v:?}"))
            })?;
        if b.is_empty()
            || b.split('/')
                .any(|p| p.is_empty() || p.contains(['.', ';', '[']))
        {
            return Err(Diagnostic::global(
                "AIC0001",
                format!("invalid DEX class descriptor: {v:?}"),
            ));
        }
        Ok(Self { descriptor: v })
    }
    #[must_use]
    pub fn descriptor(&self) -> &str {
        &self.descriptor
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Type {
    I32,
    Bool,
    String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    I32(i32),
    Bool(bool),
    String(String),
}
impl Value {
    #[must_use]
    pub fn ty(&self) -> Type {
        match self {
            Self::I32(_) => Type::I32,
            Self::Bool(_) => Type::Bool,
            Self::String(_) => Type::String,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionKind {
    Literal(Value),
    Name(String),
    Unary {
        op: UnaryOp,
        value: Box<Expression>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Call {
        name: String,
        args: Vec<Expression>,
    },
    AndroidText {
        view: String,
    },
    PreferenceGet {
        key: String,
    },
    DatabaseInsert {
        table: String,
        values: Vec<(String, Expression)>,
    },
    DatabaseExists {
        table: String,
        id: Box<Expression>,
    },
    DatabaseGet {
        table: String,
        id: Box<Expression>,
        column: String,
        default: Box<Expression>,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Negate,
    Not,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatementKind {
    Declare {
        mutable: bool,
        name: String,
        ty: Option<Type>,
        value: Expression,
    },
    Assign {
        name: String,
        value: Expression,
    },
    Return(Expression),
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    For {
        variable: String,
        start: Expression,
        end: Expression,
        body: Vec<Statement>,
    },
    LinearLayout {
        id: String,
        orientation: Orientation,
    },
    TextView {
        id: String,
        text: Expression,
    },
    Button {
        id: String,
        text: Expression,
    },
    EditText {
        id: String,
        hint: Expression,
    },
    TextInput {
        id: String,
        hint: Expression,
    },
    ScrollView {
        id: String,
    },
    AddView {
        parent: String,
        child: String,
    },
    SetContentView {
        view: String,
    },
    SetText {
        view: String,
        text: Expression,
    },
    SetLayout {
        view: String,
        width: LayoutSize,
        height: LayoutSize,
        weight: i32,
    },
    SetTextColor {
        view: String,
        color: String,
    },
    SetBackgroundColor {
        view: String,
        color: String,
    },
    PreferenceSet {
        key: String,
        value: Expression,
    },
    DatabaseUpdate {
        table: String,
        id: Expression,
        values: Vec<(String, Expression)>,
    },
    DatabaseDelete {
        table: String,
        id: Expression,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Capability {
    KeyValue,
    Sqlite,
}
impl Capability {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::KeyValue => "persistence.key_value",
            Self::Sqlite => "persistence.sqlite",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preference {
    pub name: String,
    pub ty: Type,
    pub default: Value,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Column {
    pub name: String,
    pub ty: Type,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Database {
    pub name: String,
    pub version: i32,
    pub tables: Vec<Table>,
    pub span: SourceSpan,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutSize {
    MatchParent,
    WrapContent,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub name: String,
    pub ty: Type,
    pub initial: Expression,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickHandler {
    pub view: String,
    pub body: Vec<Statement>,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxFunction {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxActivity {
    pub name: String,
    pub state: Vec<State>,
    pub on_create: Vec<Statement>,
    pub on_click: Vec<ClickHandler>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxProgram {
    pub version: String,
    pub label: String,
    pub package: String,
    pub capabilities: Vec<(Capability, SourceSpan)>,
    pub preferences: Vec<Preference>,
    pub database: Option<Database>,
    pub functions: Vec<SyntaxFunction>,
    pub activity: SyntaxActivity,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Activity {
    pub name: String,
    pub state: Vec<State>,
    pub on_create: Vec<Statement>,
    pub on_click: Vec<ClickHandler>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub version: String,
    pub label: String,
    pub package: String,
    pub capabilities: BTreeSet<Capability>,
    pub preferences: Vec<Preference>,
    pub database: Option<Database>,
    pub functions: Vec<Function>,
    pub activity: Activity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum K {
    Word(String),
    Int(i32),
    Str(String),
    Sym(&'static str),
    Eof,
}
#[derive(Clone, Debug)]
struct Tok {
    k: K,
    s: SourceSpan,
}
fn lex(src: &str) -> Result<Vec<Tok>, Diagnostic> {
    let c: Vec<char> = src.chars().collect();
    let (mut i, mut line, mut col) = (0, 1, 1);
    let mut o = vec![];
    while i < c.len() {
        let x = c[i];
        if x == '\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if x.is_whitespace() {
            i += 1;
            col += 1;
            continue;
        }
        if x == '#' {
            while i < c.len() && c[i] != '\n' {
                i += 1;
                col += 1
            }
            continue;
        }
        let st = SourceLocation { line, column: col };
        if x.is_ascii_alphabetic() || x == '_' {
            let b = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                i += 1;
                col += 1
            }
            o.push(Tok {
                k: K::Word(c[b..i].iter().collect()),
                s: SourceSpan {
                    start: st,
                    end: SourceLocation { line, column: col },
                },
            });
            continue;
        }
        if x.is_ascii_digit() {
            let b = i;
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
                col += 1
            }
            let raw: String = c[b..i].iter().collect();
            let n = raw.parse().map_err(|_| {
                Diagnostic::at(
                    "AIC1201",
                    SourceSpan {
                        start: st,
                        end: SourceLocation { line, column: col },
                    },
                    "integer literal is outside i32 range",
                )
            })?;
            o.push(Tok {
                k: K::Int(n),
                s: SourceSpan {
                    start: st,
                    end: SourceLocation { line, column: col },
                },
            });
            continue;
        }
        if x == '"' {
            i += 1;
            col += 1;
            let mut v = String::new();
            let mut closed = false;
            while i < c.len() && c[i] != '\n' {
                if c[i] == '"' {
                    i += 1;
                    col += 1;
                    closed = true;
                    break;
                }
                if c[i] == '\\' {
                    i += 1;
                    col += 1;
                    if i >= c.len() {
                        break;
                    }
                    v.push(match c[i] {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        _ => {
                            return Err(Diagnostic::at(
                                "AIC1202",
                                SourceSpan {
                                    start: st,
                                    end: SourceLocation { line, column: col },
                                },
                                "unsupported escape",
                            ))
                        }
                    });
                } else {
                    v.push(c[i])
                }
                i += 1;
                col += 1
            }
            if !closed {
                return Err(Diagnostic::at(
                    "AIC1203",
                    SourceSpan {
                        start: st,
                        end: SourceLocation { line, column: col },
                    },
                    "unterminated string",
                ));
            }
            o.push(Tok {
                k: K::Str(v),
                s: SourceSpan {
                    start: st,
                    end: SourceLocation { line, column: col },
                },
            });
            continue;
        }
        let pair = if i + 1 < c.len() {
            match (c[i], c[i + 1]) {
                ('-', '>') => Some("->"),
                ('.', '.') => Some(".."),
                ('=', '=') => Some("=="),
                ('!', '=') => Some("!="),
                ('<', '=') => Some("<="),
                ('>', '=') => Some(">="),
                ('&', '&') => Some("&&"),
                ('|', '|') => Some("||"),
                _ => None,
            }
        } else {
            None
        };
        if let Some(p) = pair {
            i += 2;
            col += 2;
            o.push(Tok {
                k: K::Sym(p),
                s: SourceSpan {
                    start: st,
                    end: SourceLocation { line, column: col },
                },
            });
            continue;
        }
        let y = match x {
            '{' => "{",
            '}' => "}",
            '(' => "(",
            ')' => ")",
            ':' => ":",
            ',' => ",",
            '=' => "=",
            '+' => "+",
            '-' => "-",
            '*' => "*",
            '/' => "/",
            '%' => "%",
            '<' => "<",
            '>' => ">",
            '!' => "!",
            '.' => ".",
            _ => {
                return Err(Diagnostic::at(
                    "AIC1204",
                    SourceSpan {
                        start: st,
                        end: SourceLocation {
                            line,
                            column: col + 1,
                        },
                    },
                    format!("unexpected character `{x}`"),
                ))
            }
        };
        i += 1;
        col += 1;
        o.push(Tok {
            k: K::Sym(y),
            s: SourceSpan {
                start: st,
                end: SourceLocation { line, column: col },
            },
        })
    }
    let p = SourceLocation { line, column: col };
    o.push(Tok {
        k: K::Eof,
        s: SourceSpan { start: p, end: p },
    });
    Ok(o)
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
}
impl Parser {
    fn cur(&self) -> &Tok {
        &self.t[self.i]
    }
    fn pop(&mut self) -> Tok {
        let x = self.t[self.i].clone();
        self.i += 1;
        x
    }
    fn word(&mut self, w: &str) -> Result<Tok, Diagnostic> {
        let x = self.pop();
        if matches!(&x.k,K::Word(v)if v==w) {
            Ok(x)
        } else {
            Err(Diagnostic::at("AIC1001", x.s, format!("expected `{w}`")))
        }
    }
    fn sym(&mut self, w: &'static str) -> Result<Tok, Diagnostic> {
        let x = self.pop();
        if x.k == K::Sym(w) {
            Ok(x)
        } else {
            Err(Diagnostic::at("AIC1002", x.s, format!("expected `{w}`")))
        }
    }
    fn id(&mut self) -> Result<(String, SourceSpan), Diagnostic> {
        let x = self.pop();
        if let K::Word(v) = x.k {
            if !keyword(&v) {
                return Ok((v, x.s));
            }
        }
        Err(Diagnostic::at("AIC1003", x.s, "expected identifier"))
    }
    fn string(&mut self) -> Result<String, Diagnostic> {
        let x = self.pop();
        if let K::Str(v) = x.k {
            Ok(v)
        } else {
            Err(Diagnostic::at("AIC1004", x.s, "expected string"))
        }
    }
    fn program(mut self) -> Result<SyntaxProgram, Diagnostic> {
        self.word("aic_version")?;
        let n = self.pop();
        if n.k != K::Int(0) {
            return Err(Diagnostic::at(
                "AIC1005",
                n.s,
                "only version 0.1 is supported",
            ));
        }
        self.sym(".")?;
        let n = self.pop();
        if n.k != K::Int(1) {
            return Err(Diagnostic::at(
                "AIC1005",
                n.s,
                "only version 0.1 is supported",
            ));
        }
        self.word("app")?;
        let label = self.string()?;
        self.word("package")?;
        let package = self.string()?;
        self.sym("{")?;
        let mut capabilities = vec![];
        while matches!(&self.cur().k,K::Word(v)if v=="capability") {
            let start = self.pop().s.start;
            self.word("persistence")?;
            self.sym(".")?;
            let token = self.pop();
            let capability = match &token.k {
                K::Word(v) if v == "key_value" => Capability::KeyValue,
                K::Word(v) if v == "sqlite" => Capability::Sqlite,
                K::Word(v) => {
                    return Err(Diagnostic::at(
                        "AIC1301",
                        token.s,
                        format!("unsupported capability `persistence.{v}`"),
                    ))
                }
                _ => {
                    return Err(Diagnostic::at(
                        "AIC1301",
                        token.s,
                        "expected persistence capability",
                    ))
                }
            };
            capabilities.push((
                capability,
                SourceSpan {
                    start,
                    end: token.s.end,
                },
            ));
        }
        let mut preferences = vec![];
        while matches!(&self.cur().k,K::Word(v)if v=="preference") {
            let start = self.pop().s.start;
            let (name, _) = self.id()?;
            self.sym(":")?;
            let ty = self.ty()?;
            self.sym("=")?;
            let token = self.pop();
            let default = match token.k {
                K::Int(v) => Value::I32(v),
                K::Str(v) => Value::String(v),
                K::Word(v) if v == "true" || v == "false" => Value::Bool(v == "true"),
                _ => {
                    return Err(Diagnostic::at(
                        "AIC1302",
                        token.s,
                        "preference default must be a literal",
                    ))
                }
            };
            preferences.push(Preference {
                name,
                ty,
                default,
                span: SourceSpan {
                    start,
                    end: token.s.end,
                },
            });
        }
        let database = if matches!(&self.cur().k,K::Word(v)if v=="database") {
            Some(self.database()?)
        } else {
            None
        };
        let mut functions = vec![];
        while matches!(&self.cur().k,K::Word(v)if v=="fn") {
            functions.push(self.function()?)
        }
        self.word("activity")?;
        let (name, _) = self.id()?;
        self.sym("{")?;
        let mut state = vec![];
        while matches!(&self.cur().k,K::Word(v)if v=="state") {
            let start = self.pop().s.start;
            let (name, _) = self.id()?;
            self.sym(":")?;
            let ty = self.ty()?;
            self.sym("=")?;
            let initial = self.expr(0)?;
            state.push(State {
                name,
                ty,
                initial,
                span: SourceSpan {
                    start,
                    end: self.t[self.i - 1].s.end,
                },
            });
        }
        self.word("on_create")?;
        let on_create = self.block()?;
        let mut on_click = vec![];
        while matches!(&self.cur().k,K::Word(v)if v=="on_click") {
            let start = self.pop().s.start;
            self.sym("(")?;
            let (view, _) = self.id()?;
            self.sym(")")?;
            let body = self.block()?;
            on_click.push(ClickHandler {
                view,
                body,
                span: SourceSpan {
                    start,
                    end: self.t[self.i - 1].s.end,
                },
            });
        }
        self.sym("}")?;
        self.sym("}")?;
        if self.cur().k != K::Eof {
            return Err(Diagnostic::at("AIC1006", self.cur().s, "content after app"));
        }
        Ok(SyntaxProgram {
            version: "0.1".into(),
            label,
            package,
            capabilities,
            preferences,
            database,
            functions,
            activity: SyntaxActivity {
                name,
                state,
                on_create,
                on_click,
            },
        })
    }
    fn database(&mut self) -> Result<Database, Diagnostic> {
        let start = self.word("database")?.s.start;
        let (name, _) = self.id()?;
        self.word("version")?;
        let version_token = self.pop();
        let K::Int(version) = version_token.k else {
            return Err(Diagnostic::at(
                "AIC1303",
                version_token.s,
                "database version must be 1",
            ));
        };
        self.sym("{")?;
        let mut tables = vec![];
        while matches!(&self.cur().k,K::Word(v)if v=="table") {
            let table_start = self.pop().s.start;
            let (table_name, _) = self.id()?;
            self.sym("{")?;
            let mut columns = vec![];
            while self.cur().k != K::Sym("}") {
                let (column_name, span) = self.id()?;
                self.sym(":")?;
                let ty = self.ty()?;
                let primary_key = matches!(&self.cur().k,K::Word(v)if v=="primary_key");
                if primary_key {
                    self.pop();
                }
                let auto_increment = matches!(&self.cur().k,K::Word(v)if v=="auto_increment");
                if auto_increment {
                    self.pop();
                }
                columns.push(Column {
                    name: column_name,
                    ty,
                    primary_key,
                    auto_increment,
                    span,
                });
            }
            let end = self.sym("}")?.s.end;
            tables.push(Table {
                name: table_name,
                columns,
                span: SourceSpan {
                    start: table_start,
                    end,
                },
            });
        }
        let end = self.sym("}")?.s.end;
        Ok(Database {
            name,
            version,
            tables,
            span: SourceSpan { start, end },
        })
    }
    fn function(&mut self) -> Result<SyntaxFunction, Diagnostic> {
        let start = self.word("fn")?.s.start;
        let (name, _) = self.id()?;
        self.sym("(")?;
        let mut params = vec![];
        if self.cur().k != K::Sym(")") {
            loop {
                let (n, s) = self.id()?;
                self.sym(":")?;
                let ty = self.ty()?;
                params.push(Parameter {
                    name: n,
                    ty,
                    span: s,
                });
                if self.cur().k != K::Sym(",") {
                    break;
                }
                self.pop();
            }
        }
        self.sym(")")?;
        self.sym("->")?;
        let return_type = self.ty()?;
        let body = self.block()?;
        let end = self.t[self.i - 1].s.end;
        Ok(SyntaxFunction {
            name,
            params,
            return_type,
            body,
            span: SourceSpan { start, end },
        })
    }
    fn ty(&mut self) -> Result<Type, Diagnostic> {
        let x = self.pop();
        match &x.k {
            K::Word(v) if v == "i32" => Ok(Type::I32),
            K::Word(v) if v == "bool" => Ok(Type::Bool),
            K::Word(v) if v == "string" => Ok(Type::String),
            _ => Err(Diagnostic::at(
                "AIC1007",
                x.s,
                "expected i32, bool, or string",
            )),
        }
    }
    fn block(&mut self) -> Result<Vec<Statement>, Diagnostic> {
        self.sym("{")?;
        let mut v = vec![];
        while self.cur().k != K::Sym("}") {
            if self.cur().k == K::Eof {
                return Err(Diagnostic::at(
                    "AIC1008",
                    self.cur().s,
                    "unterminated block",
                ));
            }
            v.push(self.statement()?)
        }
        self.pop();
        Ok(v)
    }
    fn statement(&mut self) -> Result<Statement, Diagnostic> {
        let start = self.cur().s.start;
        let kind = match &self.cur().k {
            K::Word(v) if v == "let" || v == "var" => {
                let mutable = v == "var";
                self.pop();
                let (name, _) = self.id()?;
                if self.cur().k == K::Sym("=") {
                    self.pop();
                    if matches!(&self.cur().k,K::Word(v)if v=="android") {
                        self.word("android")?;
                        self.sym(".")?;
                        let (ctor, _) = self.id()?;
                        self.sym("(")?;
                        let k = match ctor.as_str() {
                            "linear_layout" => {
                                self.word("orientation")?;
                                self.sym(":")?;
                                let (value, span) = self.id()?;
                                let orientation = match value.as_str() {
                                    "vertical" => Orientation::Vertical,
                                    "horizontal" => Orientation::Horizontal,
                                    _ => {
                                        return Err(Diagnostic::at(
                                            "AIC1012",
                                            span,
                                            "orientation must be vertical or horizontal",
                                        ))
                                    }
                                };
                                StatementKind::LinearLayout {
                                    id: name,
                                    orientation,
                                }
                            }
                            "text_view" => {
                                self.word("text")?;
                                self.sym(":")?;
                                StatementKind::TextView {
                                    id: name,
                                    text: self.expr(0)?,
                                }
                            }
                            "button" => {
                                self.word("text")?;
                                self.sym(":")?;
                                StatementKind::Button {
                                    id: name,
                                    text: self.expr(0)?,
                                }
                            }
                            "edit_text" => {
                                self.word("hint")?;
                                self.sym(":")?;
                                StatementKind::EditText {
                                    id: name,
                                    hint: self.expr(0)?,
                                }
                            }
                            "text_input" => {
                                self.word("hint")?;
                                self.sym(":")?;
                                StatementKind::TextInput {
                                    id: name,
                                    hint: self.expr(0)?,
                                }
                            }
                            "scroll_view" => StatementKind::ScrollView { id: name },
                            _ => {
                                return Err(Diagnostic::at(
                                    "AIC1010",
                                    self.cur().s,
                                    "unsupported view constructor",
                                ))
                            }
                        };
                        self.sym(")")?;
                        k
                    } else {
                        StatementKind::Declare {
                            mutable,
                            name,
                            ty: None,
                            value: self.expr(0)?,
                        }
                    }
                } else {
                    self.sym(":")?;
                    let ty = self.ty()?;
                    self.sym("=")?;
                    StatementKind::Declare {
                        mutable,
                        name,
                        ty: Some(ty),
                        value: self.expr(0)?,
                    }
                }
            }
            K::Word(v) if v == "return" => {
                self.pop();
                StatementKind::Return(self.expr(0)?)
            }
            K::Word(v) if v == "if" => {
                self.pop();
                let condition = self.expr(0)?;
                let then_body = self.block()?;
                let else_body = if matches!(&self.cur().k,K::Word(v)if v=="else") {
                    self.pop();
                    self.block()?
                } else {
                    vec![]
                };
                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                }
            }
            K::Word(v) if v == "for" => {
                self.pop();
                let (variable, _) = self.id()?;
                self.word("in")?;
                let a = self.expr(0)?;
                self.sym("..")?;
                let b = self.expr(0)?;
                let body = self.block()?;
                StatementKind::For {
                    variable,
                    start: a,
                    end: b,
                    body,
                }
            }
            K::Word(v) if v == "android" => self.android()?,
            K::Word(v) if v == "preference" => self.preference_statement()?,
            K::Word(v) if v == "database" => self.database_statement()?,
            K::Word(_) => {
                let (name, _) = self.id()?;
                self.sym("=")?;
                StatementKind::Assign {
                    name,
                    value: self.expr(0)?,
                }
            }
            _ => {
                return Err(Diagnostic::at(
                    "AIC1009",
                    self.cur().s,
                    "expected statement",
                ))
            }
        };
        Ok(Statement {
            kind,
            span: SourceSpan {
                start,
                end: self.t[self.i - 1].s.end,
            },
        })
    }
    fn preference_statement(&mut self) -> Result<StatementKind, Diagnostic> {
        self.word("preference")?;
        self.sym(".")?;
        self.word("set")?;
        self.sym("(")?;
        let (key, _) = self.id()?;
        self.sym(",")?;
        let value = self.expr(0)?;
        self.sym(")")?;
        Ok(StatementKind::PreferenceSet { key, value })
    }
    fn database_statement(&mut self) -> Result<StatementKind, Diagnostic> {
        self.word("database")?;
        self.sym(".")?;
        let (operation, span) = self.id()?;
        self.sym("(")?;
        let (table, _) = self.id()?;
        self.sym(",")?;
        let id = self.expr(0)?;
        let result = match operation.as_str() {
            "delete" => StatementKind::DatabaseDelete { table, id },
            "update" => {
                self.sym(",")?;
                let values = self.named_values()?;
                StatementKind::DatabaseUpdate { table, id, values }
            }
            _ => {
                return Err(Diagnostic::at(
                    "AIC1304",
                    span,
                    "unsupported database statement",
                ))
            }
        };
        self.sym(")")?;
        Ok(result)
    }
    fn named_values(&mut self) -> Result<Vec<(String, Expression)>, Diagnostic> {
        let mut values = vec![];
        loop {
            let (name, _) = self.id()?;
            self.sym(":")?;
            values.push((name, self.expr(0)?));
            if self.cur().k != K::Sym(",") {
                break;
            }
            self.pop();
        }
        Ok(values)
    }
    fn android(&mut self) -> Result<StatementKind, Diagnostic> {
        self.word("android")?;
        self.sym(".")?;
        let (n, _) = self.id()?;
        self.sym("(")?;
        let k = match n.as_str() {
            "add_view" => {
                self.word("parent")?;
                self.sym(":")?;
                let (p, _) = self.id()?;
                self.sym(",")?;
                self.word("child")?;
                self.sym(":")?;
                let (c, _) = self.id()?;
                StatementKind::AddView {
                    parent: p,
                    child: c,
                }
            }
            "set_content_view" => {
                let (v, _) = self.id()?;
                StatementKind::SetContentView { view: v }
            }
            "set_text" => {
                self.word("view")?;
                self.sym(":")?;
                let (view, _) = self.id()?;
                self.sym(",")?;
                self.word("text")?;
                self.sym(":")?;
                StatementKind::SetText {
                    view,
                    text: self.expr(0)?,
                }
            }
            "set_layout" => {
                self.word("view")?;
                self.sym(":")?;
                let (view, _) = self.id()?;
                self.sym(",")?;
                self.word("width")?;
                self.sym(":")?;
                let width = self.layout_size()?;
                self.sym(",")?;
                self.word("height")?;
                self.sym(":")?;
                let height = self.layout_size()?;
                self.sym(",")?;
                self.word("weight")?;
                self.sym(":")?;
                let token = self.pop();
                let K::Int(weight) = token.k else {
                    return Err(Diagnostic::at(
                        "AIC1013",
                        token.s,
                        "layout weight must be 0 or 1",
                    ));
                };
                if !(0..=1).contains(&weight) {
                    return Err(Diagnostic::at(
                        "AIC1013",
                        token.s,
                        "layout weight must be 0 or 1",
                    ));
                }
                StatementKind::SetLayout {
                    view,
                    width,
                    height,
                    weight,
                }
            }
            "set_text_color" | "set_background_color" => {
                self.word("view")?;
                self.sym(":")?;
                let (view, _) = self.id()?;
                self.sym(",")?;
                self.word("color")?;
                self.sym(":")?;
                let color = self.string()?;
                if !valid_color(&color) {
                    return Err(Diagnostic::at(
                        "AIC1014",
                        self.t[self.i - 1].s,
                        "color must be #RRGGBB or #AARRGGBB",
                    ));
                }
                if n == "set_text_color" {
                    StatementKind::SetTextColor { view, color }
                } else {
                    StatementKind::SetBackgroundColor { view, color }
                }
            }
            _ => {
                return Err(Diagnostic::at(
                    "AIC1010",
                    self.cur().s,
                    "unsupported Android operation",
                ))
            }
        };
        self.sym(")")?;
        Ok(k)
    }
    fn layout_size(&mut self) -> Result<LayoutSize, Diagnostic> {
        let (value, span) = self.id()?;
        match value.as_str() {
            "match_parent" => Ok(LayoutSize::MatchParent),
            "wrap_content" => Ok(LayoutSize::WrapContent),
            _ => Err(Diagnostic::at(
                "AIC1015",
                span,
                "layout size must be match_parent or wrap_content",
            )),
        }
    }
    fn expr(&mut self, min: u8) -> Result<Expression, Diagnostic> {
        let mut l = self.prefix()?;
        loop {
            let Some((op, p)) = bin(&self.cur().k) else {
                break;
            };
            if p < min {
                break;
            }
            self.pop();
            let r = self.expr(p + 1)?;
            let s = SourceSpan {
                start: l.span.start,
                end: r.span.end,
            };
            l = Expression {
                kind: ExpressionKind::Binary {
                    op,
                    left: Box::new(l),
                    right: Box::new(r),
                },
                span: s,
            }
        }
        Ok(l)
    }
    fn prefix(&mut self) -> Result<Expression, Diagnostic> {
        let x = self.pop();
        let st = x.s.start;
        match x.k {
            K::Int(v) => Ok(Expression {
                kind: ExpressionKind::Literal(Value::I32(v)),
                span: x.s,
            }),
            K::Str(v) => Ok(Expression {
                kind: ExpressionKind::Literal(Value::String(v)),
                span: x.s,
            }),
            K::Word(v) if v == "true" || v == "false" => Ok(Expression {
                kind: ExpressionKind::Literal(Value::Bool(v == "true")),
                span: x.s,
            }),
            K::Word(n) if n == "android" && self.cur().k == K::Sym(".") => {
                self.pop();
                let (operation, _) = self.id()?;
                if operation != "get_text" {
                    return Err(Diagnostic::at(
                        "AIC1010",
                        x.s,
                        "unsupported Android expression",
                    ));
                }
                self.sym("(")?;
                let (view, _) = self.id()?;
                self.sym(")")?;
                Ok(Expression {
                    kind: ExpressionKind::AndroidText { view },
                    span: SourceSpan {
                        start: st,
                        end: self.t[self.i - 1].s.end,
                    },
                })
            }
            K::Word(n) if (n == "preference" || n == "database") && self.cur().k == K::Sym(".") => {
                self.pop();
                let (operation, span) = self.id()?;
                self.sym("(")?;
                let kind = if n == "preference" && operation == "get" {
                    let (key, _) = self.id()?;
                    ExpressionKind::PreferenceGet { key }
                } else if n == "database" {
                    let (table, _) = self.id()?;
                    match operation.as_str() {
                        "insert" => {
                            self.sym(",")?;
                            ExpressionKind::DatabaseInsert {
                                table,
                                values: self.named_values()?,
                            }
                        }
                        "exists" => {
                            self.sym(",")?;
                            ExpressionKind::DatabaseExists {
                                table,
                                id: Box::new(self.expr(0)?),
                            }
                        }
                        "get" => {
                            self.sym(",")?;
                            let id = Box::new(self.expr(0)?);
                            self.sym(",")?;
                            let (column, _) = self.id()?;
                            self.sym(",")?;
                            let default = Box::new(self.expr(0)?);
                            ExpressionKind::DatabaseGet {
                                table,
                                id,
                                column,
                                default,
                            }
                        }
                        _ => {
                            return Err(Diagnostic::at(
                                "AIC1305",
                                span,
                                "unsupported database expression",
                            ))
                        }
                    }
                } else {
                    return Err(Diagnostic::at(
                        "AIC1306",
                        span,
                        "unsupported preference expression",
                    ));
                };
                let end = self.sym(")")?.s.end;
                Ok(Expression {
                    kind,
                    span: SourceSpan { start: st, end },
                })
            }
            K::Word(n) => {
                if self.cur().k == K::Sym("(") {
                    self.pop();
                    let mut a = vec![];
                    if self.cur().k != K::Sym(")") {
                        loop {
                            a.push(self.expr(0)?);
                            if self.cur().k != K::Sym(",") {
                                break;
                            }
                            self.pop();
                        }
                    }
                    let end = self.sym(")")?.s.end;
                    Ok(Expression {
                        kind: ExpressionKind::Call { name: n, args: a },
                        span: SourceSpan { start: st, end },
                    })
                } else {
                    Ok(Expression {
                        kind: ExpressionKind::Name(n),
                        span: x.s,
                    })
                }
            }
            K::Sym("-") | K::Sym("!") => {
                let op = if x.k == K::Sym("-") {
                    UnaryOp::Negate
                } else {
                    UnaryOp::Not
                };
                let v = self.expr(8)?;
                let end = v.span.end;
                Ok(Expression {
                    kind: ExpressionKind::Unary {
                        op,
                        value: Box::new(v),
                    },
                    span: SourceSpan { start: st, end },
                })
            }
            K::Sym("(") => {
                let e = self.expr(0)?;
                self.sym(")")?;
                Ok(e)
            }
            _ => Err(Diagnostic::at("AIC1011", x.s, "expected expression")),
        }
    }
}
fn valid_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}
fn keyword(v: &str) -> bool {
    matches!(
        v,
        "aic_version"
            | "app"
            | "package"
            | "fn"
            | "activity"
            | "on_create"
            | "on_click"
            | "state"
            | "capability"
            | "preference"
            | "database"
            | "version"
            | "table"
            | "let"
            | "var"
            | "return"
            | "if"
            | "else"
            | "for"
            | "in"
            | "true"
            | "false"
            | "android"
            | "i32"
            | "bool"
            | "string"
    )
}
fn bin(k: &K) -> Option<(BinaryOp, u8)> {
    Some(match k {
        K::Sym("||") => (BinaryOp::Or, 1),
        K::Sym("&&") => (BinaryOp::And, 2),
        K::Sym("==") => (BinaryOp::Equal, 3),
        K::Sym("!=") => (BinaryOp::NotEqual, 3),
        K::Sym("<") => (BinaryOp::Less, 4),
        K::Sym("<=") => (BinaryOp::LessEqual, 4),
        K::Sym(">") => (BinaryOp::Greater, 4),
        K::Sym(">=") => (BinaryOp::GreaterEqual, 4),
        K::Sym("+") => (BinaryOp::Add, 5),
        K::Sym("-") => (BinaryOp::Subtract, 5),
        K::Sym("*") => (BinaryOp::Multiply, 6),
        K::Sym("/") => (BinaryOp::Divide, 6),
        K::Sym("%") => (BinaryOp::Remainder, 6),
        _ => return None,
    })
}
pub fn parse(source: &str) -> Result<SyntaxProgram, Diagnostic> {
    Parser {
        t: lex(source)?,
        i: 0,
    }
    .program()
}
pub fn parse_program(source: &str) -> Result<Program, Diagnostic> {
    verify(parse(source)?)
}

#[derive(Clone)]
struct Binding {
    ty: Type,
    mutable: bool,
    is_view: bool,
}
struct Check {
    sig: BTreeMap<String, (Vec<Type>, Type)>,
    ret: Option<Type>,
    caller: String,
    calls: BTreeMap<String, BTreeSet<String>>,
    preferences: BTreeMap<String, Type>,
    tables: BTreeMap<String, BTreeMap<String, Column>>,
    used_capabilities: BTreeSet<Capability>,
}
pub fn verify(s: SyntaxProgram) -> Result<Program, Diagnostic> {
    if s.package.split('.').count() < 2
        || s.package.split('.').any(|x| {
            x.is_empty()
                || !x.starts_with(|c: char| c == '_' || c.is_ascii_lowercase())
                || !x.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
        })
    {
        return Err(Diagnostic::global(
            "AIC1101",
            format!("invalid Android package `{}`", s.package),
        ));
    }
    let content_views = s
        .activity
        .on_create
        .iter()
        .filter(|statement| matches!(statement.kind, StatementKind::SetContentView { .. }))
        .count();
    if content_views == 0 {
        return Err(Diagnostic::global(
            "AIC1106",
            "on_create must call android.set_content_view",
        ));
    }
    if content_views > 1 {
        return Err(Diagnostic::global(
            "AIC1107",
            "on_create may call android.set_content_view only once",
        ));
    }
    let mut containers = BTreeMap::new();
    for statement in &s.activity.on_create {
        match &statement.kind {
            StatementKind::LinearLayout { id, .. } => {
                containers.insert(id.clone(), None);
            }
            StatementKind::ScrollView { id } => {
                containers.insert(id.clone(), Some(0_usize));
            }
            _ => {}
        }
    }
    for statement in &s.activity.on_create {
        if let StatementKind::AddView { parent, .. } = &statement.kind {
            let entry = containers.get_mut(parent).ok_or_else(|| {
                Diagnostic::at(
                    "AIC1130",
                    statement.span,
                    "android.add_view parent must be a LinearLayout or ScrollView",
                )
            })?;
            if let Some(count) = entry {
                *count += 1;
                if *count > 1 {
                    return Err(Diagnostic::at(
                        "AIC1131",
                        statement.span,
                        "ScrollView accepts exactly one child",
                    ));
                }
            }
        }
    }
    if containers.values().any(|value| matches!(value, Some(0))) {
        return Err(Diagnostic::global(
            "AIC1132",
            "ScrollView must contain exactly one child",
        ));
    }
    let mut declared_capabilities = BTreeSet::new();
    for (capability, span) in &s.capabilities {
        if !declared_capabilities.insert(*capability) {
            return Err(Diagnostic::at(
                "AIC1307",
                *span,
                format!("duplicate capability `{}`", capability.name()),
            ));
        }
    }
    let mut preferences = BTreeMap::new();
    for preference in &s.preferences {
        req(preference.ty, preference.default.ty(), preference.span)?;
        if preferences
            .insert(preference.name.clone(), preference.ty)
            .is_some()
        {
            return Err(Diagnostic::at(
                "AIC1308",
                preference.span,
                "duplicate preference key",
            ));
        }
    }
    let mut tables = BTreeMap::new();
    if let Some(database) = &s.database {
        if database.version != 1 {
            return Err(Diagnostic::at(
                "AIC1309",
                database.span,
                "only database version 1 is supported",
            ));
        }
        for table in &database.tables {
            let mut columns = BTreeMap::new();
            let mut primary_keys = 0;
            for column in &table.columns {
                if column.primary_key {
                    primary_keys += 1;
                    if column.ty != Type::I32 || !column.auto_increment {
                        return Err(Diagnostic::at(
                            "AIC1310",
                            column.span,
                            "primary key must be i32 primary_key auto_increment",
                        ));
                    }
                }
                if column.auto_increment && !column.primary_key {
                    return Err(Diagnostic::at(
                        "AIC1311",
                        column.span,
                        "auto_increment requires primary_key",
                    ));
                }
                if columns
                    .insert(column.name.clone(), column.clone())
                    .is_some()
                {
                    return Err(Diagnostic::at(
                        "AIC1312",
                        column.span,
                        "duplicate database column",
                    ));
                }
            }
            if primary_keys != 1 {
                return Err(Diagnostic::at(
                    "AIC1313",
                    table.span,
                    "table requires exactly one primary key",
                ));
            }
            if tables.insert(table.name.clone(), columns).is_some() {
                return Err(Diagnostic::at(
                    "AIC1314",
                    table.span,
                    "duplicate database table",
                ));
            }
        }
    }
    let mut sig = BTreeMap::new();
    for f in &s.functions {
        if sig
            .insert(
                f.name.clone(),
                (f.params.iter().map(|x| x.ty).collect(), f.return_type),
            )
            .is_some()
        {
            return Err(Diagnostic::at(
                "AIC1102",
                f.span,
                format!("duplicate function `{}`", f.name),
            ));
        }
    }
    let mut graph = BTreeMap::new();
    for f in &s.functions {
        let mut c = Check {
            sig: sig.clone(),
            ret: Some(f.return_type),
            caller: f.name.clone(),
            calls: BTreeMap::new(),
            preferences: preferences.clone(),
            tables: tables.clone(),
            used_capabilities: BTreeSet::new(),
        };
        let mut env = BTreeMap::new();
        for p in &f.params {
            if env
                .insert(
                    p.name.clone(),
                    Binding {
                        ty: p.ty,
                        mutable: false,
                        is_view: false,
                    },
                )
                .is_some()
            {
                return Err(Diagnostic::at("AIC1103", p.span, "duplicate parameter"));
            }
        }
        if !c.stmts(&f.body, &mut env)? {
            return Err(Diagnostic::at(
                "AIC1104",
                f.span,
                format!("function `{}` does not return on every path", f.name),
            ));
        }
        for (k, v) in c.calls {
            graph.entry(k).or_insert_with(BTreeSet::new).extend(v)
        }
    }
    let mut c = Check {
        sig,
        ret: None,
        caller: "<on_create>".into(),
        calls: BTreeMap::new(),
        preferences: preferences.clone(),
        tables: tables.clone(),
        used_capabilities: BTreeSet::new(),
    };
    let mut activity_env = BTreeMap::new();
    for state in &s.activity.state {
        let ty = c.expr(&state.initial, &activity_env)?;
        req(state.ty, ty, state.initial.span)?;
        if activity_env
            .insert(
                state.name.clone(),
                Binding {
                    ty: state.ty,
                    mutable: true,
                    is_view: false,
                },
            )
            .is_some()
        {
            return Err(Diagnostic::at(
                "AIC1124",
                state.span,
                "duplicate activity state",
            ));
        }
    }
    c.stmts(&s.activity.on_create, &mut activity_env)?;
    let buttons: BTreeSet<_> = s
        .activity
        .on_create
        .iter()
        .filter_map(|statement| match &statement.kind {
            StatementKind::Button { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    let mut handled = BTreeSet::new();
    for handler in &s.activity.on_click {
        if !buttons.contains(&handler.view) {
            return Err(Diagnostic::at(
                "AIC1125",
                handler.span,
                "on_click target must be a declared button",
            ));
        }
        if !handled.insert(handler.view.clone()) {
            return Err(Diagnostic::at(
                "AIC1126",
                handler.span,
                "duplicate on_click handler",
            ));
        }
        c.stmts(&handler.body, &mut activity_env.clone())?;
    }
    let used = c.used_capabilities.clone();
    for capability in &used {
        if !declared_capabilities.contains(capability) {
            return Err(Diagnostic::global(
                "AIC1315",
                format!(
                    "capability `{}` is used but not declared",
                    capability.name()
                ),
            ));
        }
    }
    for capability in &declared_capabilities {
        if !used.contains(capability) {
            return Err(Diagnostic::global(
                "AIC1316",
                format!("declared capability `{}` is unused", capability.name()),
            ));
        }
    }
    cycles(&graph)?;
    Ok(Program {
        version: s.version,
        label: s.label,
        package: s.package,
        capabilities: declared_capabilities,
        preferences: s.preferences,
        database: s.database,
        functions: s
            .functions
            .into_iter()
            .map(|f| Function {
                name: f.name,
                params: f.params,
                return_type: f.return_type,
                body: f.body,
                span: f.span,
            })
            .collect(),
        activity: Activity {
            name: s.activity.name,
            state: s.activity.state,
            on_create: s.activity.on_create,
            on_click: s.activity.on_click,
        },
    })
}
impl Check {
    fn stmts(
        &mut self,
        b: &[Statement],
        e: &mut BTreeMap<String, Binding>,
    ) -> Result<bool, Diagnostic> {
        let mut r = false;
        for s in b {
            if r {
                return Err(Diagnostic::at("AIC1105", s.span, "unreachable statement"));
            }
            r = self.stmt(s, e)?
        }
        Ok(r)
    }
    fn stmt(
        &mut self,
        s: &Statement,
        e: &mut BTreeMap<String, Binding>,
    ) -> Result<bool, Diagnostic> {
        match &s.kind {
            StatementKind::Declare {
                mutable,
                name,
                ty,
                value,
            } => {
                let a = self.expr(value, e)?;
                if let Some(t) = ty {
                    req(*t, a, value.span)?
                }
                if e.insert(
                    name.clone(),
                    Binding {
                        ty: ty.unwrap_or(a),
                        mutable: *mutable,
                        is_view: false,
                    },
                )
                .is_some()
                {
                    return Err(Diagnostic::at(
                        "AIC1106",
                        s.span,
                        format!("duplicate local `{name}`"),
                    ));
                }
                Ok(false)
            }
            StatementKind::Assign { name, value } => {
                let b = e.get(name).cloned().ok_or_else(|| {
                    Diagnostic::at("AIC1107", s.span, format!("undefined symbol `{name}`"))
                })?;
                if !b.mutable {
                    return Err(Diagnostic::at(
                        "AIC1108",
                        s.span,
                        format!("cannot assign to immutable `{name}`"),
                    ));
                }
                let a = self.expr(value, e)?;
                req(b.ty, a, value.span)?;
                Ok(false)
            }
            StatementKind::Return(x) => {
                let t = self.ret.ok_or_else(|| {
                    Diagnostic::at("AIC1109", s.span, "return is not allowed here")
                })?;
                let a = self.expr(x, e)?;
                req(t, a, x.span)?;
                Ok(true)
            }
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let a = self.expr(condition, e)?;
                req(Type::Bool, a, condition.span)?;
                let x = self.stmts(then_body, &mut e.clone())?;
                let y = self.stmts(else_body, &mut e.clone())?;
                Ok(x && !else_body.is_empty() && y)
            }
            StatementKind::For {
                variable,
                start,
                end,
                body,
            } => {
                let a = self.expr(start, e)?;
                req(Type::I32, a, start.span)?;
                let a = self.expr(end, e)?;
                req(Type::I32, a, end.span)?;
                let mut n = e.clone();
                if n.insert(
                    variable.clone(),
                    Binding {
                        ty: Type::I32,
                        mutable: false,
                        is_view: false,
                    },
                )
                .is_some()
                {
                    return Err(Diagnostic::at(
                        "AIC1110",
                        s.span,
                        "loop variable shadows a symbol",
                    ));
                }
                self.stmts(body, &mut n)?;
                Ok(false)
            }
            StatementKind::LinearLayout { id, .. }
            | StatementKind::TextView { id, .. }
            | StatementKind::Button { id, .. }
            | StatementKind::EditText { id, .. }
            | StatementKind::TextInput { id, .. }
            | StatementKind::ScrollView { id } => {
                if e.insert(
                    id.clone(),
                    Binding {
                        ty: Type::String,
                        mutable: false,
                        is_view: true,
                    },
                )
                .is_some()
                {
                    return Err(Diagnostic::at("AIC1104", s.span, "duplicate view id"));
                }
                if let StatementKind::TextView { text, .. }
                | StatementKind::Button { text, .. }
                | StatementKind::EditText { hint: text, .. }
                | StatementKind::TextInput { hint: text, .. } = &s.kind
                {
                    let a = self.expr(text, e)?;
                    req(Type::String, a, text.span)?
                }
                Ok(false)
            }
            StatementKind::AddView { parent, child } => {
                view(e, parent, s.span)?;
                view(e, child, s.span)?;
                Ok(false)
            }
            StatementKind::SetContentView { view: v }
            | StatementKind::SetLayout { view: v, .. }
            | StatementKind::SetTextColor { view: v, .. }
            | StatementKind::SetBackgroundColor { view: v, .. } => {
                view(e, v, s.span)?;
                Ok(false)
            }
            StatementKind::SetText { view: v, text } => {
                view(e, v, s.span)?;
                let actual = self.expr(text, e)?;
                req(Type::String, actual, text.span)?;
                Ok(false)
            }
            StatementKind::PreferenceSet { key, value } => {
                if self.ret.is_some() {
                    return Err(Diagnostic::at(
                        "AIC1317",
                        s.span,
                        "persistence is activity-only",
                    ));
                }
                let expected = *self.preferences.get(key).ok_or_else(|| {
                    Diagnostic::at("AIC1318", s.span, format!("unknown preference key `{key}`"))
                })?;
                let actual = self.expr(value, e)?;
                req(expected, actual, value.span)?;
                self.used_capabilities.insert(Capability::KeyValue);
                Ok(false)
            }
            StatementKind::DatabaseUpdate { table, id, values } => {
                self.verify_database_write(table, Some(id), values, e, s.span)?;
                Ok(false)
            }
            StatementKind::DatabaseDelete { table, id } => {
                self.verify_database_write(table, Some(id), &[], e, s.span)?;
                Ok(false)
            }
        }
    }
    fn expr(&mut self, x: &Expression, e: &BTreeMap<String, Binding>) -> Result<Type, Diagnostic> {
        match &x.kind {
            ExpressionKind::Literal(v) => Ok(v.ty()),
            ExpressionKind::Name(n) => e.get(n).map(|x| x.ty).ok_or_else(|| {
                Diagnostic::at("AIC1107", x.span, format!("undefined symbol `{n}`"))
            }),
            ExpressionKind::Unary { op, value } => {
                let t = if *op == UnaryOp::Not {
                    Type::Bool
                } else {
                    Type::I32
                };
                let a = self.expr(value, e)?;
                req(t, a, value.span)?;
                Ok(t)
            }
            ExpressionKind::Binary { op, left, right } => {
                let l = self.expr(left, e)?;
                let r = self.expr(right, e)?;
                match op {
                    BinaryOp::Add if l == Type::String && r == Type::String => Ok(Type::String),
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => {
                        req(Type::I32, l, left.span)?;
                        req(Type::I32, r, right.span)?;
                        if matches!(op, BinaryOp::Divide | BinaryOp::Remainder)
                            && matches!(right.kind, ExpressionKind::Literal(Value::I32(0)))
                        {
                            return Err(Diagnostic::at(
                                "AIC1111",
                                right.span,
                                "division or remainder by zero",
                            ));
                        }
                        Ok(Type::I32)
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        req(Type::I32, l, left.span)?;
                        req(Type::I32, r, right.span)?;
                        Ok(Type::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        req(Type::Bool, l, left.span)?;
                        req(Type::Bool, r, right.span)?;
                        Ok(Type::Bool)
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        req(l, r, right.span)?;
                        Ok(Type::Bool)
                    }
                }
            }
            ExpressionKind::Call { name, args } => {
                if name == "valid_i32" {
                    if args.len() != 1 || self.expr(&args[0], e)? != Type::String {
                        return Err(Diagnostic::at(
                            "AIC1127",
                            x.span,
                            "valid_i32() expects one string",
                        ));
                    }
                    return Ok(Type::Bool);
                }
                if name == "i32" {
                    if args.len() != 1 || self.expr(&args[0], e)? != Type::String {
                        return Err(Diagnostic::at(
                            "AIC1128",
                            x.span,
                            "i32() expects one validated string",
                        ));
                    }
                    return Ok(Type::I32);
                }
                if name == "string" {
                    if args.len() != 1 {
                        return Err(Diagnostic::at(
                            "AIC1112",
                            x.span,
                            "string() expects one argument",
                        ));
                    }
                    let t = self.expr(&args[0], e)?;
                    if t == Type::String {
                        return Err(Diagnostic::at(
                            "AIC1113",
                            x.span,
                            "string() accepts i32 or bool",
                        ));
                    }
                    return Ok(Type::String);
                }
                let (ps, r) = self.sig.get(name).cloned().ok_or_else(|| {
                    Diagnostic::at("AIC1114", x.span, format!("undefined function `{name}`"))
                })?;
                if ps.len() != args.len() {
                    return Err(Diagnostic::at("AIC1115", x.span, "wrong argument count"));
                }
                for (a, t) in args.iter().zip(ps) {
                    let q = self.expr(a, e)?;
                    req(t, q, a.span)?
                }
                self.calls
                    .entry(self.caller.clone())
                    .or_default()
                    .insert(name.clone());
                Ok(r)
            }
            ExpressionKind::AndroidText { view: name } => {
                view(e, name, x.span)?;
                Ok(Type::String)
            }
            ExpressionKind::PreferenceGet { key } => {
                if self.ret.is_some() {
                    return Err(Diagnostic::at(
                        "AIC1317",
                        x.span,
                        "persistence is activity-only",
                    ));
                }
                let ty = *self.preferences.get(key).ok_or_else(|| {
                    Diagnostic::at("AIC1318", x.span, format!("unknown preference key `{key}`"))
                })?;
                self.used_capabilities.insert(Capability::KeyValue);
                Ok(ty)
            }
            ExpressionKind::DatabaseInsert { table, values } => {
                self.verify_database_write(table, None, values, e, x.span)?;
                Ok(Type::I32)
            }
            ExpressionKind::DatabaseExists { table, id } => {
                self.verify_database_read(table, id, e, x.span)?;
                Ok(Type::Bool)
            }
            ExpressionKind::DatabaseGet {
                table,
                id,
                column,
                default,
            } => {
                let columns = self.verify_database_read(table, id, e, x.span)?;
                let ty = columns
                    .get(column)
                    .ok_or_else(|| {
                        Diagnostic::at("AIC1319", x.span, format!("unknown column `{column}`"))
                    })?
                    .ty;
                let actual = self.expr(default, e)?;
                req(ty, actual, default.span)?;
                Ok(ty)
            }
        }
    }
    fn verify_database_read(
        &mut self,
        table: &str,
        id: &Expression,
        e: &BTreeMap<String, Binding>,
        span: SourceSpan,
    ) -> Result<BTreeMap<String, Column>, Diagnostic> {
        if self.ret.is_some() {
            return Err(Diagnostic::at(
                "AIC1317",
                span,
                "persistence is activity-only",
            ));
        }
        let columns =
            self.tables.get(table).cloned().ok_or_else(|| {
                Diagnostic::at("AIC1320", span, format!("unknown table `{table}`"))
            })?;
        let actual = self.expr(id, e)?;
        req(Type::I32, actual, id.span)?;
        self.used_capabilities.insert(Capability::Sqlite);
        Ok(columns)
    }
    fn verify_database_write(
        &mut self,
        table: &str,
        id: Option<&Expression>,
        values: &[(String, Expression)],
        e: &BTreeMap<String, Binding>,
        span: SourceSpan,
    ) -> Result<(), Diagnostic> {
        let columns = if let Some(id) = id {
            self.verify_database_read(table, id, e, span)?
        } else {
            if self.ret.is_some() {
                return Err(Diagnostic::at(
                    "AIC1317",
                    span,
                    "persistence is activity-only",
                ));
            }
            self.used_capabilities.insert(Capability::Sqlite);
            self.tables.get(table).cloned().ok_or_else(|| {
                Diagnostic::at("AIC1320", span, format!("unknown table `{table}`"))
            })?
        };
        let mut seen = BTreeSet::new();
        for (name, value) in values {
            let column = columns.get(name).ok_or_else(|| {
                Diagnostic::at("AIC1319", value.span, format!("unknown column `{name}`"))
            })?;
            if column.primary_key {
                return Err(Diagnostic::at(
                    "AIC1321",
                    value.span,
                    "primary key cannot be written",
                ));
            }
            if !seen.insert(name) {
                return Err(Diagnostic::at(
                    "AIC1322",
                    value.span,
                    "duplicate query column",
                ));
            }
            let actual = self.expr(value, e)?;
            req(column.ty, actual, value.span)?;
        }
        if id.is_none()
            && columns
                .values()
                .filter(|column| !column.primary_key)
                .any(|column| !seen.contains(&column.name))
        {
            return Err(Diagnostic::at(
                "AIC1323",
                span,
                "insert must provide every non-primary column",
            ));
        }
        Ok(())
    }
}
fn req(a: Type, b: Type, s: SourceSpan) -> Result<(), Diagnostic> {
    if a == b {
        Ok(())
    } else {
        Err(Diagnostic::at(
            "AIC1116",
            s,
            format!("type mismatch: expected {a:?}, found {b:?}"),
        ))
    }
}
fn view(e: &BTreeMap<String, Binding>, n: &str, s: SourceSpan) -> Result<(), Diagnostic> {
    if e.get(n).is_some_and(|x| x.is_view) {
        Ok(())
    } else {
        Err(Diagnostic::at(
            "AIC1105",
            s,
            format!("unknown view id `{n}`"),
        ))
    }
}
fn cycles(g: &BTreeMap<String, BTreeSet<String>>) -> Result<(), Diagnostic> {
    fn go(
        n: &str,
        g: &BTreeMap<String, BTreeSet<String>>,
        a: &mut BTreeSet<String>,
        d: &mut BTreeSet<String>,
    ) -> bool {
        if a.contains(n) {
            return true;
        }
        if d.contains(n) {
            return false;
        }
        a.insert(n.into());
        if g.get(n).is_some_and(|v| v.iter().any(|x| go(x, g, a, d))) {
            return true;
        }
        a.remove(n);
        d.insert(n.into());
        false
    }
    let mut d = BTreeSet::new();
    for n in g.keys() {
        if go(n, g, &mut BTreeSet::new(), &mut d) {
            return Err(Diagnostic::global(
                "AIC1119",
                "recursive or cyclic calls are unsupported",
            ));
        }
    }
    Ok(())
}

pub fn evaluate_text(p: &Program) -> Result<String, Diagnostic> {
    let mut e = BTreeMap::new();
    for s in &p.activity.on_create {
        if let StatementKind::Declare { name, value, .. } = &s.kind {
            e.insert(name.clone(), eval(value, &e, p)?);
        }
        if let StatementKind::TextView { text, .. } = &s.kind {
            if let Value::String(v) = eval(text, &e, p)? {
                return Ok(v);
            }
        }
    }
    Err(Diagnostic::global(
        "AIC1120",
        "on_create must create a TextView",
    ))
}
fn eval(x: &Expression, e: &BTreeMap<String, Value>, p: &Program) -> Result<Value, Diagnostic> {
    match &x.kind {
        ExpressionKind::Literal(v) => Ok(v.clone()),
        ExpressionKind::Name(n) => e
            .get(n)
            .cloned()
            .ok_or_else(|| Diagnostic::at("AIC1107", x.span, "undefined value")),
        ExpressionKind::Unary { op, value } => match (op, eval(value, e, p)?) {
            (UnaryOp::Negate, Value::I32(v)) => Ok(Value::I32(v.wrapping_neg())),
            (UnaryOp::Not, Value::Bool(v)) => Ok(Value::Bool(!v)),
            _ => Err(Diagnostic::at("AIC1121", x.span, "invalid unary value")),
        },
        ExpressionKind::Binary { op, left, right } => {
            let l = eval(left, e, p)?;
            if *op == BinaryOp::And && l == Value::Bool(false) {
                return Ok(l);
            }
            if *op == BinaryOp::Or && l == Value::Bool(true) {
                return Ok(l);
            }
            let r = eval(right, e, p)?;
            value(*op, l, r, x.span)
        }
        ExpressionKind::Call { name, args } => {
            let a = args
                .iter()
                .map(|v| eval(v, e, p))
                .collect::<Result<Vec<_>, _>>()?;
            if name == "string" {
                return Ok(Value::String(match &a[0] {
                    Value::I32(v) => v.to_string(),
                    Value::Bool(v) => v.to_string(),
                    Value::String(v) => v.clone(),
                }));
            }
            let f = p
                .functions
                .iter()
                .find(|f| f.name == *name)
                .ok_or_else(|| Diagnostic::at("AIC1114", x.span, "unknown function"))?;
            let mut q = f
                .params
                .iter()
                .zip(a)
                .map(|(x, v)| (x.name.clone(), v))
                .collect();
            run(&f.body, &mut q, p)?
                .ok_or_else(|| Diagnostic::at("AIC1104", f.span, "missing return"))
        }
        ExpressionKind::AndroidText { .. }
        | ExpressionKind::PreferenceGet { .. }
        | ExpressionKind::DatabaseInsert { .. }
        | ExpressionKind::DatabaseExists { .. }
        | ExpressionKind::DatabaseGet { .. } => Err(Diagnostic::at(
            "AIC1129",
            x.span,
            "UI text is available only at runtime",
        )),
    }
}
fn run(
    b: &[Statement],
    e: &mut BTreeMap<String, Value>,
    p: &Program,
) -> Result<Option<Value>, Diagnostic> {
    for s in b {
        match &s.kind {
            StatementKind::Declare { name, value, .. } | StatementKind::Assign { name, value } => {
                let v = eval(value, e, p)?;
                e.insert(name.clone(), v);
            }
            StatementKind::Return(x) => return Ok(Some(eval(x, e, p)?)),
            StatementKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let q = if eval(condition, e, p)? == Value::Bool(true) {
                    then_body
                } else {
                    else_body
                };
                if let Some(v) = run(q, e, p)? {
                    return Ok(Some(v));
                }
            }
            StatementKind::For {
                variable,
                start,
                end,
                body,
            } => {
                let (Value::I32(a), Value::I32(z)) = (eval(start, e, p)?, eval(end, e, p)?) else {
                    return Err(Diagnostic::at("AIC1122", s.span, "invalid loop bounds"));
                };
                for i in a..z {
                    e.insert(variable.clone(), Value::I32(i));
                    if let Some(v) = run(body, e, p)? {
                        return Ok(Some(v));
                    }
                }
                e.remove(variable);
            }
            _ => {}
        }
    }
    Ok(None)
}
fn value(op: BinaryOp, l: Value, r: Value, s: SourceSpan) -> Result<Value, Diagnostic> {
    use BinaryOp::*;
    Ok(match (op, l, r) {
        (Add, Value::I32(a), Value::I32(b)) => Value::I32(a.wrapping_add(b)),
        (Subtract, Value::I32(a), Value::I32(b)) => Value::I32(a.wrapping_sub(b)),
        (Multiply, Value::I32(a), Value::I32(b)) => Value::I32(a.wrapping_mul(b)),
        (Divide, Value::I32(a), Value::I32(b)) if b != 0 => {
            Value::I32(a.checked_div(b).unwrap_or(i32::MIN))
        }
        (Remainder, Value::I32(a), Value::I32(b)) if b != 0 => {
            Value::I32(a.checked_rem(b).unwrap_or(0))
        }
        (Add, Value::String(a), Value::String(b)) => Value::String(a + &b),
        (Equal, a, b) => Value::Bool(a == b),
        (NotEqual, a, b) => Value::Bool(a != b),
        (Less, Value::I32(a), Value::I32(b)) => Value::Bool(a < b),
        (LessEqual, Value::I32(a), Value::I32(b)) => Value::Bool(a <= b),
        (Greater, Value::I32(a), Value::I32(b)) => Value::Bool(a > b),
        (GreaterEqual, Value::I32(a), Value::I32(b)) => Value::Bool(a >= b),
        (And, Value::Bool(a), Value::Bool(b)) => Value::Bool(a && b),
        (Or, Value::Bool(a), Value::Bool(b)) => Value::Bool(a || b),
        _ => return Err(Diagnostic::at("AIC1123", s, "invalid binary value")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const M1:&str="aic_version 0.1 app \"Hello\" package \"dev.aic.hello\" { activity MainActivity { on_create { let root = android.linear_layout(orientation: vertical) let message = android.text_view(text: \"Hello\") android.add_view(parent: root, child: message) android.set_content_view(root) } } }";
    const M2:&str="aic_version 0.1 app \"Compute\" package \"dev.aic.compute\" { fn sum(n: i32) -> i32 { var total: i32 = 0 for i in 0..n { if i % 2 == 0 { total = total + i } else { total = total + 1 } } return total } activity MainActivity { on_create { let result: i32 = sum(10) let root = android.linear_layout(orientation: vertical) let message = android.text_view(text: \"Result: \" + string(result)) android.add_view(parent: root, child: message) android.set_content_view(root) } } }";
    #[test]
    fn m1() {
        assert!(parse_program(M1).is_ok())
    }
    #[test]
    fn m2() {
        assert_eq!(
            evaluate_text(&parse_program(M2).unwrap()).unwrap(),
            "Result: 25"
        )
    }
    #[test]
    fn precedence() {
        let p = parse_program(&M2.replace("sum(10)", "1 + 2 * 3")).unwrap();
        assert_eq!(evaluate_text(&p).unwrap(), "Result: 7")
    }
    #[test]
    fn immutable() {
        assert_eq!(
            parse_program(&M2.replace("var total", "let total"))
                .unwrap_err()
                .code,
            "AIC1108"
        )
    }
    #[test]
    fn descriptor() {
        assert!(MinimalClass::new("Ldev/aic/X;").is_ok())
    }
    #[test]
    fn rejects_type_and_symbol_errors_with_locations() {
        let type_error =
            parse_program(&M2.replace("var total: i32 = 0", "var total: bool = 0")).unwrap_err();
        assert_eq!(type_error.code, "AIC1116");
        assert!(type_error.location.is_some());
        let symbol_error =
            parse_program(&M2.replace("total = total + i", "total = missing + i")).unwrap_err();
        assert_eq!(symbol_error.code, "AIC1107");
    }
    #[test]
    fn rejects_missing_return_and_static_zero_divisor() {
        assert_eq!(
            parse_program(&M2.replace(" return total", ""))
                .unwrap_err()
                .code,
            "AIC1104"
        );
        assert_eq!(
            parse_program(&M2.replace("i % 2", "i % 0"))
                .unwrap_err()
                .code,
            "AIC1111"
        );
    }
    #[test]
    fn preserves_m1_view_diagnostics() {
        assert_eq!(
            parse_program(&M1.replace("child: message", "child: missing"))
                .unwrap_err()
                .code,
            "AIC1105"
        );
        assert_eq!(
            parse_program(&M1.replace("android.set_content_view(root)", ""))
                .unwrap_err()
                .code,
            "AIC1106"
        );
    }
    #[test]
    fn m3_state_widgets_and_handlers_verify() {
        let program = parse_program(include_str!("../../../testdata/counter.aic")).unwrap();
        assert_eq!(program.activity.state.len(), 1);
        assert_eq!(program.activity.on_click.len(), 3);
    }
    #[test]
    fn m3_rejects_non_button_handler_and_bad_orientation() {
        assert_eq!(
            parse_program(
                &include_str!("../../../testdata/counter.aic")
                    .replace("on_click(increment)", "on_click(output)")
            )
            .unwrap_err()
            .code,
            "AIC1125"
        );
        assert_eq!(
            parse_program(&M1.replace("vertical", "diagonal"))
                .unwrap_err()
                .code,
            "AIC1012"
        );
        assert_eq!(
            parse_program(
                &include_str!("../../../testdata/counter.aic").replace("#202124", "blue")
            )
            .unwrap_err()
            .code,
            "AIC1014"
        );
        assert_eq!(
            parse_program(&include_str!("../../../testdata/calculator.aic").replace("android.add_view(parent: scroll, child: root)", "android.add_view(parent: scroll, child: root) android.add_view(parent: scroll, child: result)"))
                .unwrap_err().code,
            "AIC1131"
        );
    }
    #[test]
    fn m4_typed_persistence_verifies() {
        let program = parse_program(include_str!("../../../testdata/notes.aic")).unwrap();
        assert_eq!(program.capabilities.len(), 2);
        assert_eq!(program.preferences[0].name, "last_note_id");
        assert_eq!(program.database.as_ref().unwrap().tables[0].name, "notes");
    }
    #[test]
    fn m4_rejects_capability_and_schema_errors() {
        let source = include_str!("../../../testdata/notes.aic");
        let undeclared = source.replace("  capability persistence.sqlite\n", "");
        assert_eq!(parse_program(&undeclared).unwrap_err().code, "AIC1315");
        let unused = source.replace(
            "  capability persistence.sqlite\n",
            "  capability persistence.sqlite\n  capability persistence.sqlite\n",
        );
        assert_eq!(parse_program(&unused).unwrap_err().code, "AIC1307");
        let unused = M1.replace("{ activity", "{ capability persistence.key_value activity");
        assert_eq!(parse_program(&unused).unwrap_err().code, "AIC1316");
        let unsupported = M1.replace("{ activity", "{ capability persistence.camera activity");
        assert_eq!(parse_program(&unsupported).unwrap_err().code, "AIC1301");
        let bad_version = source.replace("version 1", "version 2");
        assert_eq!(parse_program(&bad_version).unwrap_err().code, "AIC1309");
        let unknown_column = source.replace(
            "title: title_text, body: body_text)",
            "missing: title_text, body: body_text)",
        );
        assert_eq!(parse_program(&unknown_column).unwrap_err().code, "AIC1319");
        let mismatch = source.replace(
            "preference.set(last_note_id, note_id)",
            "preference.set(last_note_id, title_text)",
        );
        assert_eq!(parse_program(&mismatch).unwrap_err().code, "AIC1116");
    }
}

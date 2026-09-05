//! Validated input models for the deliberately small AIC compiler surface.
use std::{collections::BTreeSet, error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimalClass {
    descriptor: String,
}
impl MinimalClass {
    /// Creates a class after validating its DEX descriptor.
    ///
    /// # Errors
    /// Returns `AIC0001` when the descriptor is outside the supported object form.
    pub fn new(descriptor: impl Into<String>) -> Result<Self, IrError> {
        let descriptor = descriptor.into();
        validate_class_descriptor(&descriptor)?;
        Ok(Self { descriptor })
    }
    #[must_use]
    pub fn descriptor(&self) -> &str {
        &self.descriptor
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub version: String,
    pub label: String,
    pub package: String,
    pub activity: Activity,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Activity {
    pub name: String,
    pub on_create: Vec<Operation>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    LinearLayout {
        id: String,
        location: SourceLocation,
    },
    TextView {
        id: String,
        text: String,
        location: SourceLocation,
    },
    AddView {
        parent: String,
        child: String,
        location: SourceLocation,
    },
    SetContentView {
        view: String,
        location: SourceLocation,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrError {
    pub code: &'static str,
    pub location: Option<SourceLocation>,
    pub message: String,
}
impl IrError {
    fn at(code: &'static str, line: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            location: Some(SourceLocation { line, column: 1 }),
            message: message.into(),
        }
    }
    fn global(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            location: None,
            message: message.into(),
        }
    }
}
impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = self.location {
            write!(
                f,
                "{} at {}:{}: {}",
                self.code, p.line, p.column, self.message
            )
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}
impl Error for IrError {}

/// Parses and verifies the complete M1 text DSL.
///
/// # Errors
/// Returns a stable, source-located diagnostic for invalid or unsupported input.
#[allow(clippy::too_many_lines)]
pub fn parse_program(source: &str) -> Result<Program, IrError> {
    let lines: Vec<_> = source
        .lines()
        .enumerate()
        .filter_map(|(i, raw)| {
            let line = raw.trim();
            (!line.is_empty() && !line.starts_with('#')).then_some((i + 1, line))
        })
        .collect();
    let mut cursor = 0;
    let (line, declaration) = take(
        &lines,
        &mut cursor,
        "AIC1001",
        "missing aic_version declaration",
    )?;
    let version = declaration
        .strip_prefix("aic_version ")
        .ok_or_else(|| IrError::at("AIC1002", line, "expected `aic_version 0.1`"))?;
    if version != "0.1" {
        return Err(IrError::at(
            "AIC1003",
            line,
            "only AIC version 0.1 is supported",
        ));
    }
    let (line, declaration) = take(&lines, &mut cursor, "AIC1004", "missing app declaration")?;
    let app = declaration
        .strip_prefix("app ")
        .and_then(|s| s.strip_suffix(" {"))
        .ok_or_else(|| {
            IrError::at(
                "AIC1005",
                line,
                "expected `app \"label\" package \"name\" {`",
            )
        })?;
    let (label, rest) =
        quoted(app).ok_or_else(|| IrError::at("AIC1005", line, "invalid quoted app label"))?;
    let rest = rest
        .trim()
        .strip_prefix("package ")
        .ok_or_else(|| IrError::at("AIC1005", line, "missing package declaration"))?;
    let (package, trailing) =
        quoted(rest).ok_or_else(|| IrError::at("AIC1005", line, "invalid quoted package"))?;
    if !trailing.trim().is_empty() {
        return Err(IrError::at(
            "AIC1005",
            line,
            "unsupported app declaration content",
        ));
    }
    validate_package(package).map_err(|m| IrError::at("AIC1101", line, m))?;
    let (line, declaration) = take(
        &lines,
        &mut cursor,
        "AIC1006",
        "missing activity declaration",
    )?;
    let name = declaration
        .strip_prefix("activity ")
        .and_then(|s| s.strip_suffix(" {"))
        .ok_or_else(|| IrError::at("AIC1007", line, "expected `activity Name {`"))?;
    validate_identifier(name).map_err(|m| IrError::at("AIC1102", line, m))?;
    let (line, declaration) = take(
        &lines,
        &mut cursor,
        "AIC1008",
        "missing on_create declaration",
    )?;
    if declaration != "on_create {" {
        return Err(IrError::at(
            "AIC1009",
            line,
            "expected exactly one `on_create {`",
        ));
    }
    let mut operations = Vec::new();
    let mut ids = BTreeSet::new();
    let mut content_views = 0;
    loop {
        let (line, text) = take(
            &lines,
            &mut cursor,
            "AIC1010",
            "unterminated on_create block",
        )?;
        if text == "}" {
            break;
        }
        let location = SourceLocation { line, column: 1 };
        if let Some(rest) = text.strip_prefix("let ") {
            let (id, expression) = rest
                .split_once(" = ")
                .ok_or_else(|| IrError::at("AIC1011", line, "invalid let operation"))?;
            validate_identifier(id).map_err(|m| IrError::at("AIC1103", line, m))?;
            if !ids.insert(id.to_owned()) {
                return Err(IrError::at(
                    "AIC1104",
                    line,
                    format!("duplicate view id `{id}`"),
                ));
            }
            if expression == "android.linear_layout(orientation: vertical)" {
                operations.push(Operation::LinearLayout {
                    id: id.to_owned(),
                    location,
                });
            } else if let Some(arg) = expression
                .strip_prefix("android.text_view(text: ")
                .and_then(|s| s.strip_suffix(')'))
            {
                let (value, trailing) = quoted(arg)
                    .ok_or_else(|| IrError::at("AIC1012", line, "TextView text must be quoted"))?;
                if !trailing.is_empty() {
                    return Err(IrError::at(
                        "AIC1012",
                        line,
                        "unsupported TextView arguments",
                    ));
                }
                operations.push(Operation::TextView {
                    id: id.to_owned(),
                    text: value.to_owned(),
                    location,
                });
            } else {
                return Err(IrError::at("AIC1013", line, "unsupported view constructor"));
            }
        } else if let Some(args) = text
            .strip_prefix("android.add_view(parent: ")
            .and_then(|s| s.strip_suffix(')'))
        {
            let (parent, child) = args
                .split_once(", child: ")
                .ok_or_else(|| IrError::at("AIC1014", line, "invalid add_view arguments"))?;
            require_id(&ids, parent, line)?;
            require_id(&ids, child, line)?;
            operations.push(Operation::AddView {
                parent: parent.to_owned(),
                child: child.to_owned(),
                location,
            });
        } else if let Some(view) = text
            .strip_prefix("android.set_content_view(")
            .and_then(|s| s.strip_suffix(')'))
        {
            require_id(&ids, view, line)?;
            content_views += 1;
            operations.push(Operation::SetContentView {
                view: view.to_owned(),
                location,
            });
        } else {
            return Err(IrError::at(
                "AIC1015",
                line,
                "unsupported on_create operation",
            ));
        }
    }
    if content_views == 0 {
        return Err(IrError::global(
            "AIC1106",
            "on_create must call android.set_content_view",
        ));
    }
    if content_views > 1 {
        return Err(IrError::global(
            "AIC1107",
            "on_create may call android.set_content_view only once",
        ));
    }
    for _ in 0..2 {
        let (line, text) = take(
            &lines,
            &mut cursor,
            "AIC1016",
            "unterminated activity or app block",
        )?;
        if text != "}" {
            return Err(IrError::at("AIC1017", line, "expected closing `}`"));
        }
    }
    if let Some((line, _)) = lines.get(cursor) {
        return Err(IrError::at(
            "AIC1018",
            *line,
            "unexpected content after app",
        ));
    }
    Ok(Program {
        version: version.to_owned(),
        label: label.to_owned(),
        package: package.to_owned(),
        activity: Activity {
            name: name.to_owned(),
            on_create: operations,
        },
    })
}

fn take<'a>(
    lines: &'a [(usize, &'a str)],
    cursor: &mut usize,
    code: &'static str,
    message: &'static str,
) -> Result<(usize, &'a str), IrError> {
    let value = lines
        .get(*cursor)
        .copied()
        .ok_or_else(|| IrError::global(code, message))?;
    *cursor += 1;
    Ok(value)
}
fn quoted(value: &str) -> Option<(&str, &str)> {
    let body = value.strip_prefix('"')?;
    let end = body.find('"')?;
    Some((&body[..end], &body[end + 1..]))
}
fn require_id(ids: &BTreeSet<String>, id: &str, line: usize) -> Result<(), IrError> {
    if ids.contains(id) {
        Ok(())
    } else {
        Err(IrError::at(
            "AIC1105",
            line,
            format!("unknown view id `{id}`"),
        ))
    }
}
fn validate_identifier(value: &str) -> Result<(), String> {
    let mut chars = value.chars();
    if !chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        || !chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    {
        return Err(format!("invalid identifier `{value}`"));
    }
    Ok(())
}
fn validate_package(value: &str) -> Result<(), String> {
    if value.split('.').count() < 2
        || value.split('.').any(|p| {
            validate_identifier(p).is_err()
                || p.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        })
    {
        return Err(format!("invalid Android package `{value}`"));
    }
    Ok(())
}
fn validate_class_descriptor(value: &str) -> Result<(), IrError> {
    let body = value
        .strip_prefix('L')
        .and_then(|s| s.strip_suffix(';'))
        .ok_or_else(|| {
            IrError::global(
                "AIC0001",
                format!("invalid DEX class descriptor: {value:?}"),
            )
        })?;
    if body.is_empty()
        || body.starts_with('/')
        || body.ends_with('/')
        || body.split('/').any(|p| {
            p.is_empty()
                || p.chars()
                    .any(|c| c.is_control() || matches!(c, '.' | ';' | '['))
        })
    {
        return Err(IrError::global(
            "AIC0001",
            format!("invalid DEX class descriptor: {value:?}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const VALID: &str = "aic_version 0.1\napp \"AIC Hello\" package \"dev.aic.generated.hello\" {\nactivity MainActivity {\non_create {\nlet root = android.linear_layout(orientation: vertical)\nlet message = android.text_view(text: \"Hello from AndroidIntelligentCompiler\")\nandroid.add_view(parent: root, child: message)\nandroid.set_content_view(root)\n}\n}\n}\n";
    #[test]
    fn parses_valid_program() {
        assert_eq!(parse_program(VALID).unwrap().activity.on_create.len(), 4);
    }
    #[test]
    fn rejects_bad_package() {
        assert_eq!(
            parse_program(&VALID.replace("dev.aic.generated.hello", "Bad"))
                .unwrap_err()
                .code,
            "AIC1101"
        );
    }
    #[test]
    fn rejects_unknown_reference() {
        assert_eq!(
            parse_program(&VALID.replace("child: message", "child: missing"))
                .unwrap_err()
                .code,
            "AIC1105"
        );
    }
    #[test]
    fn requires_content_view() {
        assert_eq!(
            parse_program(&VALID.replace("android.set_content_view(root)\n", ""))
                .unwrap_err()
                .code,
            "AIC1106"
        );
    }
    #[test]
    fn rejects_duplicate_lifecycle() {
        assert_eq!(
            parse_program(&VALID.replace("}\n}\n}\n", "}\non_create {\n}\n}\n}\n"))
                .unwrap_err()
                .code,
            "AIC1017"
        );
    }
    #[test]
    fn validates_descriptor() {
        assert!(MinimalClass::new("Ldev/aic/Minimal;").is_ok());
        assert!(MinimalClass::new("bad").is_err());
    }
}

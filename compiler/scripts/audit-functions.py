#!/usr/bin/env python3
"""Generate a maintainability audit of named functions in the AIC repository."""

from __future__ import annotations

import argparse
import os
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path


EXTENSIONS = {".rs": "Rust", ".kt": "Kotlin", ".kts": "Kotlin", ".py": "Python", ".ps1": "PowerShell"}
SKIP_PARTS = {".git", ".gradle", "build", "target", "generated", "__pycache__"}


@dataclass(frozen=True)
class Function:
    language: str
    path: str
    line: int
    name: str
    visibility: str
    parameters: str
    returns: str
    category: str
    documented: bool
    priority: str


def balanced_signature(lines: list[str], start: int, limit: int = 30) -> str:
    """Join a multiline declaration until its parameter list is balanced."""
    pieces: list[str] = []
    depth = 0
    seen_open = False
    for line in lines[start : start + limit]:
        clean = line.strip()
        pieces.append(clean)
        depth += clean.count("(") - clean.count(")")
        seen_open = seen_open or "(" in clean
        if seen_open and depth <= 0:
            break
    return " ".join(pieces)


def preceding_documentation(lines: list[str], index: int, language: str) -> bool:
    """Return whether a declaration has an adjacent language-native doc comment."""
    cursor = index - 1
    while cursor >= 0 and (not lines[cursor].strip() or lines[cursor].lstrip().startswith(("#[", "@"))):
        cursor -= 1
    if cursor < 0:
        return False
    text = lines[cursor].strip()
    if language == "Rust":
        return text.startswith("///")
    if language == "Kotlin":
        return text.endswith("*/") and any("/**" in line for line in lines[max(0, cursor - 20) : cursor + 1])
    if language == "Python":
        # Python function docstrings occur after the declaration and are handled by the caller.
        return False
    if language == "PowerShell":
        return text.startswith("#")
    return False


def split_signature(signature: str, name: str) -> tuple[str, str]:
    """Extract compact parameter and return text from a declaration signature."""
    match = re.search(rf"\b{re.escape(name)}\s*\((.*)\)", signature)
    parameters = re.sub(r"\s+", " ", match.group(1)).strip() if match else ""
    tail = signature[match.end() :] if match else ""
    rust_return = re.search(r"->\s*([^\{;=]+)", tail)
    kotlin_return = re.search(r":\s*([^\{=]+)", tail)
    returns = (rust_return or kotlin_return)
    return parameters or "-", re.sub(r"\s+", " ", returns.group(1)).strip() if returns else "implicit/unit"


def category_for(path: Path) -> str:
    """Classify a declaration by its repository role."""
    normalized = path.as_posix()
    if "/test/" in normalized or "/tests/" in normalized or "/androidTest/" in normalized:
        return "test"
    if "/scripts/" in normalized:
        return "tooling"
    if "/host/app/src/main/" in normalized:
        return "host"
    return "compiler"


def priority_for(category: str, visibility: str, documented: bool) -> str:
    """Assign documentation priority based on exposure and operational importance."""
    if documented:
        return "documented"
    if visibility in {"public", "external"}:
        return "P0"
    if category in {"compiler", "host"}:
        return "P1"
    return "P2"


def scan_file(root: Path, path: Path) -> list[Function]:
    """Extract named declarations from one supported source file."""
    language = EXTENSIONS[path.suffix]
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    relative = path.relative_to(root).as_posix()
    category = category_for(Path("/") / relative)
    found: list[Function] = []
    patterns = {
        "Rust": re.compile(r'^\s*(?:(?:pub(?:\([^)]*\))?|const|async|unsafe|extern(?:\s+"[^"]+")?)\s+)*fn\s+([A-Za-z_]\w*)\s*\('),
        "Kotlin": re.compile(r"^\s*(?:(?:public|private|internal|protected|override|suspend|inline|operator|tailrec|external|open|final|abstract|infix)\s+)*fun\s+(?:<[^>]+>\s*)?([A-Za-z_]\w*)\s*\("),
        "Python": re.compile(r"^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)\s*\("),
        "PowerShell": re.compile(r"^\s*function\s+([A-Za-z_][\w-]*)", re.IGNORECASE),
    }
    for index, line in enumerate(lines):
        match = patterns[language].search(line)
        if not match:
            continue
        name = match.group(1)
        signature = balanced_signature(lines, index) if language != "PowerShell" else line.strip()
        if language == "Rust":
            visibility = "public" if re.search(r"\bpub\s*(?:\([^)]*\))?", line) else "private"
            if 'extern "system"' in signature:
                visibility = "external"
        elif language == "Kotlin":
            visibility = "private" if re.search(r"\bprivate\b", line) else ("external" if "external fun" in line else "public")
        elif language == "Python":
            visibility = "private" if name.startswith("_") else "public"
        else:
            visibility = "public"
        documented = preceding_documentation(lines, index, language)
        if language == "Python":
            cursor = index + 1
            while cursor < len(lines) and not lines[cursor].strip():
                cursor += 1
            documented = cursor < len(lines) and lines[cursor].lstrip().startswith(('"""', "'''"))
        parameters, returns = split_signature(signature, name)
        if language == "PowerShell":
            parameters = "see param block" if any("param(" in candidate.lower() for candidate in lines[index : index + 10]) else "-"
            returns = "implicit"
        found.append(Function(language, relative, index + 1, name, visibility, parameters, returns, category, documented, priority_for(category, visibility, documented)))
    return found


def markdown(functions: list[Function]) -> str:
    """Render the complete audit and prioritized documentation checklist."""
    by_language = Counter(item.language for item in functions)
    by_category = Counter(item.category for item in functions)
    by_priority = Counter(item.priority for item in functions)
    documented = sum(item.documented for item in functions)
    lines = [
        "# Function Documentation Audit",
        "",
        "Generated by `python compiler/scripts/audit-functions.py` from named function declarations.",
        "The inventory is intentionally syntactic: generated/build directories are excluded, while unit tests embedded in source files remain in their containing category.",
        "",
        "## Summary",
        "",
        f"- Total functions: **{len(functions)}**",
        f"- Documented functions: **{documented}**",
        f"- Undocumented functions: **{len(functions) - documented}**",
        f"- Documentation coverage: **{documented / len(functions):.1%}**" if functions else "- Documentation coverage: **0.0%**",
        "",
        "| Language | Functions |",
        "|---|---:|",
    ]
    lines.extend(f"| {name} | {count} |" for name, count in sorted(by_language.items()))
    lines.extend(["", "| Category | Functions |", "|---|---:|"])
    lines.extend(f"| {name} | {count} |" for name, count in sorted(by_category.items()))
    lines.extend([
        "",
        "| Documentation priority | Functions | Meaning |",
        "|---|---:|---|",
        f"| P0 | {by_priority['P0']} | Undocumented public/external boundary; document first |",
        f"| P1 | {by_priority['P1']} | Undocumented compiler or Android-host implementation |",
        f"| P2 | {by_priority['P2']} | Undocumented tests and operational tooling |",
        f"| Documented | {by_priority['documented']} | Native doc comment detected |",
        "",
        "## How to use this audit",
        "",
        "Add native documentation (`///`, KDoc, Python docstring, or PowerShell comment help), rerun the generator, and review the P0 list before P1 and P2. Parameters and returns below are mechanically extracted and should be verified when writing descriptions.",
        "",
    ])
    grouped: dict[str, list[Function]] = defaultdict(list)
    for item in functions:
        grouped[item.priority].append(item)
    for priority in ("P0", "P1", "P2", "documented"):
        title = "Already documented" if priority == "documented" else f"{priority} checklist"
        lines.extend([f"## {title}", "", "| Done | Function | Language / role | Parameters | Returns | Location |", "|---|---|---|---|---|---|"])
        for item in sorted(grouped[priority], key=lambda value: (value.path, value.line, value.name)):
            checkbox = "[x]" if item.documented else "[ ]"
            parameters = item.parameters.replace("|", "\\|").replace("`", "'")
            returns = item.returns.replace("|", "\\|").replace("`", "'")
            lines.append(f"| {checkbox} | `{item.name}` | {item.language} / {item.category} | `{parameters}` | `{returns}` | [`{item.path}:{item.line}`](../../{item.path}:{item.line}) |")
        lines.append("")
    return "\n".join(lines)


def main() -> None:
    """Scan the repository and write the deterministic Markdown report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    output = args.output or root / "compiler" / "docs" / "function-audit.md"
    functions: list[Function] = []
    source_files: list[Path] = []
    for source_root in (root / "compiler", root / "host"):
        for directory, names, files in os.walk(source_root):
            names[:] = sorted(name for name in names if name not in SKIP_PARTS)
            source_files.extend(Path(directory) / name for name in files if Path(name).suffix in EXTENSIONS)
    for path in sorted(source_files):
        functions.extend(scan_file(root, path))
    output.write_text(markdown(functions), encoding="utf-8")
    print(f"wrote {output} with {len(functions)} functions")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
Simple DSL based audit log query tool.

Supports expressions of the form:
    metadata.bridge.platform == "windows-msvc" and severity == "Error"
    category == "ffi.bridge" and pass_rate >= 1.0
Operators: ==, !=, ~= (substring), >=, <=, >, <, in
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple


def _bootstrap_imports() -> None:
    if __package__:
        return
    current = Path(__file__).resolve().parent
    parent = current.parent
    for path in (current, parent):
        if str(path) not in sys.path:
            sys.path.insert(0, str(path))


_bootstrap_imports()

from audit_shared import NormalizedAuditEntry, load_entries  # type: ignore  # pylint: disable=import-error


@dataclass
class Condition:
    field: str
    operator: str
    value: Any


def _parse_value(token: str) -> Any:
    if token.startswith("[") and token.endswith("]"):
        inner = token[1:-1].strip()
        if not inner:
            return []
        return [item.strip().strip('"').strip("'") for item in inner.split(",")]
    if token.startswith('"') and token.endswith('"'):
        return token[1:-1]
    if token.startswith("'") and token.endswith("'"):
        return token[1:-1]
    try:
        return int(token)
    except ValueError:
        pass
    try:
        return float(token)
    except ValueError:
        pass
    if token.lower() in {"true", "false"}:
        return token.lower() == "true"
    return token


def _parse_and_expression(expression: str) -> List[Condition]:
    parts = expression.split(" and ")
    conditions: List[Condition] = []
    for part in parts:
        part = part.strip()
        if not part:
            continue
        for op in ("==", "!=", ">=", "<=", ">", "<", "~="):
            if op in part:
                field, value = part.split(op, 1)
                conditions.append(
                    Condition(field.strip(), op, _parse_value(value.strip()))
                )
                break
        else:
            if " in " in part:
                field, value = part.split(" in ", 1)
                conditions.append(
                    Condition(field.strip(), "in", _parse_value(value.strip()))
                )
            else:
                raise ValueError(f"Unsupported expression: {part}")
    return conditions


def parse_conditions(expression: str) -> Tuple[List[List[Condition]], str]:
    expression = expression.strip()
    if not expression:
        return [], "and"
    if " or " in expression:
        groups = []
        for part in expression.split(" or "):
            groups.append(_parse_and_expression(part.strip()))
        return groups, "or"
    return [_parse_and_expression(expression)], "and"


def _resolve_field(entry: NormalizedAuditEntry, field: str) -> Any:
    if field == "category":
        return entry.category
    if field == "code":
        return entry.code
    if field == "severity":
        return entry.severity
    if field == "pass_rate":
        return entry.pass_rate
    if field == "audit_id":
        return entry.audit_id
    if field == "cli.audit_id":
        return entry.cli_audit_id
    if field == "change_set":
        return entry.change_set
    if field.startswith("metadata."):
        return _resolve_path(entry.metadata, field.split(".")[1:])
    if field.startswith("extensions."):
        return _resolve_path(entry.extensions, field.split(".")[1:])
    return _resolve_path(entry.raw, field.split("."))


def _resolve_path(container: Any, segments: Sequence[str]) -> Any:
    current = container
    for segment in segments:
        if isinstance(current, dict) and segment in current:
            current = current[segment]
        else:
            return None
    return current


def _match_condition(entry: NormalizedAuditEntry, condition: Condition) -> bool:
    value = _resolve_field(entry, condition.field)
    op = condition.operator
    target = condition.value
    if op == "==":
        return value == target
    if op == "!=":
        return value != target
    if op == ">":
        return isinstance(value, (int, float)) and value > target
    if op == "<":
        return isinstance(value, (int, float)) and value < target
    if op == ">=":
        return isinstance(value, (int, float)) and value >= target
    if op == "<=":
        return isinstance(value, (int, float)) and value <= target
    if op == "~=":
        if value is None:
            return False
        return str(target) in str(value)
    if op == "in":
        if not isinstance(target, (list, tuple, set)):
            return False
        return value in target
    return False


def filter_entries(
    entries: Sequence[NormalizedAuditEntry],
    query: Optional[str],
) -> List[NormalizedAuditEntry]:
    if not query:
        return list(entries)
    groups, mode = parse_conditions(query)
    if not groups:
        return list(entries)
    filtered: List[NormalizedAuditEntry] = []
    for entry in entries:
        if mode == "or":
            if any(all(_match_condition(entry, cond) for cond in group) for group in groups):
                filtered.append(entry)
        else:  # and
            group = groups[0]
            if all(_match_condition(entry, cond) for cond in group):
                filtered.append(entry)
    return filtered


def _render_table(entries: Sequence[NormalizedAuditEntry]) -> str:
    lines = []
    header = f"{'CATEGORY':30} {'CODE':25} {'SEVERITY':10} {'PASS_RATE':8}"
    lines.append(header)
    lines.append("-" * len(header))
    for entry in entries:
        lines.append(
            f"{entry.category:30.30} "
            f"{(entry.code or ''):25.25} "
            f"{(entry.severity or ''):10.10} "
            f"{'' if entry.pass_rate is None else f'{entry.pass_rate:.2f}':>8}"
        )
    return "\n".join(lines)


def collect_summary(
    entries: Sequence[NormalizedAuditEntry], *, total: int, preset: str
) -> Dict[str, Any]:
    return {
        "preset": preset,
        "matched": len(entries),
        "total": total,
        "coverage": (len(entries) / total) if total else None,
    }


def parse_cli(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Query audit JSON logs.")
    parser.add_argument(
        "--from",
        dest="sources",
        action="append",
        required=True,
        help="Input JSON/JSONL path (repeatable).",
    )
    parser.add_argument(
        "--query",
        type=str,
        help="Filter expression.",
    )
    parser.add_argument(
        "--query-file",
        type=Path,
        help="Read filter expression from file (overrides --query if both are set).",
    )
    parser.add_argument(
        "--format",
        choices=["json", "table", "ndjson"],
        default="json",
        help="Output format (default: json).",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Write result to file instead of stdout.",
    )
    parser.add_argument(
        "--preset-name",
        type=str,
        default="ad-hoc",
        help="Preset name recorded in JSON output.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        help="Maximum number of entries to emit.",
    )
    return parser.parse_args(argv)


def run_cli(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_cli(argv)
    entries: List[NormalizedAuditEntry] = []
    for src in args.sources:
        src_path = Path(src)
        entries.extend(load_entries(src_path))
    total_entries = len(entries)
    query_expr = args.query
    if args.query_file:
        query_expr = args.query_file.read_text(encoding="utf-8").strip()
    filtered = filter_entries(entries, query_expr)
    if args.limit is not None:
        filtered = filtered[: args.limit]

    if args.format == "table":
        text = _render_table(filtered)
        if args.output:
            args.output.write_text(text + "\n", encoding="utf-8")
        else:
            print(text)
        return 0

    if args.format == "ndjson":
        out_lines = [json.dumps(entry.raw, ensure_ascii=False) for entry in filtered]
        text = "\n".join(out_lines)
        if args.output:
            args.output.write_text(text + ("\n" if text else ""), encoding="utf-8")
        else:
            print(text)
        return 0

    # JSON summary
    summary = collect_summary(filtered, total=total_entries, preset=args.preset_name)
    summary["entries"] = [entry.raw for entry in filtered]
    text = json.dumps(summary, ensure_ascii=False, indent=2)
    if args.output:
        args.output.write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    return 0


if __name__ == "__main__":
    sys.exit(run_cli())

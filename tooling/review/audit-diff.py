#!/usr/bin/env python3
"""
Audit log diff generator.

Compares two audit JSON/JSONL files and emits summary reports (JSON/Markdown/HTML).
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple


def _bootstrap_imports() -> None:
    if __package__:
        return
    current = Path(__file__).resolve().parent
    parent = current.parent
    for path in (current, parent):
        if str(path) not in sys.path:
            sys.path.insert(0, str(path))


_bootstrap_imports()

from audit_shared import (  # type: ignore  # pylint: disable=import-error
    NormalizedAuditEntry,
    flatten_metadata,
    load_entries,
)

try:
    from audit_query import filter_entries  # type: ignore  # pylint: disable=import-error
except Exception:  # pragma: no cover - fallback when audit_query not available
    def filter_entries(
        entries: Sequence[NormalizedAuditEntry],
        query: Optional[str] = None,
    ) -> List[NormalizedAuditEntry]:
        return list(entries)


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Compare two audit logs.")
    parser.add_argument("--base", required=True, type=Path, help="Baseline JSON/JSONL path")
    parser.add_argument("--target", required=True, type=Path, help="Target JSON/JSONL path")
    parser.add_argument(
        "--format",
        default="json,md",
        help="Comma separated list of output formats: json,md,html (default: json,md)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Output directory (default: reports/audit/review/<target stem>/)",
    )
    parser.add_argument(
        "--query",
        type=str,
        help="Filter query (DSL parsed by audit-query).",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.0,
        help="Regressions threshold for warnings (currently informational).",
    )
    parser.add_argument(
        "--preset-name",
        type=str,
        help="Optional preset/label recorded in outputs.",
    )
    return parser.parse_args(argv)


def _output_directory(target: Path, specified: Optional[Path]) -> Path:
    if specified:
        return specified
    commit_dir = target.stem
    return Path("reports/audit/review") / commit_dir


def _entry_signature(entry: NormalizedAuditEntry) -> Tuple[str, Optional[str]]:
    return entry.identity()


def _flatten_snapshot(entry: NormalizedAuditEntry) -> Dict[str, str]:
    return flatten_metadata(entry)


def _compare_entries(
    base_entries: Sequence[NormalizedAuditEntry],
    target_entries: Sequence[NormalizedAuditEntry],
) -> Dict[str, object]:
    base_index: Dict[Tuple[str, Optional[str]], NormalizedAuditEntry] = {
        _entry_signature(entry): entry for entry in base_entries
    }
    target_index: Dict[Tuple[str, Optional[str]], NormalizedAuditEntry] = {
        _entry_signature(entry): entry for entry in target_entries
    }

    base_keys = set(base_index.keys())
    target_keys = set(target_index.keys())

    removed_keys = sorted(base_keys - target_keys)
    new_keys = sorted(target_keys - base_keys)
    shared_keys = sorted(base_keys & target_keys)

    metadata_changed_details: List[Dict[str, object]] = []
    pass_rate_previous: List[float] = []
    pass_rate_current: List[float] = []

    for key in shared_keys:
        base_entry = base_index[key]
        target_entry = target_index[key]
        base_flat = _flatten_snapshot(base_entry)
        target_flat = _flatten_snapshot(target_entry)
        base_items = sorted(base_flat.items())
        target_items = sorted(target_flat.items())
        if base_items != target_items:
            metadata_changed_details.append(
                {
                    "category": key[0],
                    "code": key[1],
                    "base": base_flat,
                    "target": target_flat,
                }
            )
        if base_entry.pass_rate is not None:
            pass_rate_previous.append(base_entry.pass_rate)
        if target_entry.pass_rate is not None:
            pass_rate_current.append(target_entry.pass_rate)

    def _summarize_pass_rate(values: Iterable[float]) -> Optional[float]:
        items = list(values)
        if not items:
            return None
        return sum(items) / len(items)

    previous_mean = _summarize_pass_rate(pass_rate_previous)
    current_mean = _summarize_pass_rate(pass_rate_current)
    delta: Optional[float] = None
    if previous_mean is not None and current_mean is not None:
        delta = current_mean - previous_mean

    def _detail_for(keys: List[Tuple[str, Optional[str]]], label: str) -> List[Dict[str, object]]:
        details: List[Dict[str, object]] = []
        for category, code in keys:
            details.append({"category": category, "code": code, "kind": label})
        return details

    diagnostic_details = (
        _detail_for(removed_keys, "removed") + _detail_for(new_keys, "added")
    )

    summary = {
        "diagnostic": {
            "regressions": len(removed_keys),
            "new": len(new_keys),
            "improved": 0,
            "details": diagnostic_details,
        },
        "metadata": {
            "changed": len(metadata_changed_details),
            "details": metadata_changed_details,
        },
        "pass_rate": {
            "previous": previous_mean,
            "current": current_mean,
            "delta": delta,
        },
    }
    return summary


def _write_json(path: Path, data: Dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(data, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def _write_markdown(path: Path, summary: Dict[str, object]) -> None:
    diagnostic = summary["diagnostic"]
    metadata = summary["metadata"]
    pass_rate = summary["pass_rate"]
    lines = [
        "# Audit Diff Summary",
        "",
        f"- 既存診断の退行数: {diagnostic['regressions']}",
        f"- 新規診断数: {diagnostic['new']}",
        f"- メタデータ変更数: {metadata['changed']}",
        f"- pass_rate 変化: {pass_rate.get('delta')}",
        "",
        "## 詳細",
    ]
    details: List[Dict[str, object]] = diagnostic.get("details", [])
    if details:
        lines.append("### 診断差分")
        for item in details:
            lines.append(
                f"- `{item.get('category')}` / `{item.get('code')}` ({item.get('kind')})"
            )
        lines.append("")
    metadata_details: List[Dict[str, object]] = metadata.get("details", [])
    if metadata_details:
        lines.append("### メタデータ変更")
        for item in metadata_details:
            lines.append(
                f"- `{item.get('category')}` / `{item.get('code')}`: キー差分 {len(item.get('base', {}))} → {len(item.get('target', {}))}"
            )
        lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def _load_html_template() -> Optional[str]:
    template_path = Path(__file__).resolve().parent / "templates" / "audit-diff.html"
    if template_path.is_file():
        return template_path.read_text(encoding="utf-8")
    return None


def _format_table(headers: Sequence[str], rows: Sequence[Sequence[str]]) -> str:
    if not rows:
        return '<div class="empty-state">差分は検出されませんでした。</div>'
    thead = "".join(f"<th>{header}</th>" for header in headers)
    tbody = "".join(
        "<tr>" + "".join(f"<td>{cell}</td>" for cell in row) + "</tr>"
        for row in rows
    )
    return f"<table><thead><tr>{thead}</tr></thead><tbody>{tbody}</tbody></table>"


def _write_html(path: Path, summary: Dict[str, object]) -> None:
    template = _load_html_template()
    diagnostic = summary["diagnostic"]
    metadata = summary["metadata"]
    pass_rate = summary["pass_rate"]
    base = summary.get("base", {})
    target = summary.get("target", {})

    diagnostic_rows: List[Sequence[str]] = []
    for item in diagnostic.get("details", []):
        if isinstance(item, dict):
            diagnostic_rows.append(
                (
                    item.get("category", ""),
                    item.get("code", ""),
                    item.get("kind", ""),
                )
            )

    metadata_rows: List[Sequence[str]] = []
    for item in metadata.get("details", []):
        if not isinstance(item, dict):
            continue
        base_payload = item.get("base") or {}
        target_payload = item.get("target") or {}
        changed_keys = sorted(set(base_payload.keys()) ^ set(target_payload.keys()))
        description = (
            ", ".join(changed_keys) if changed_keys else "(値のみ変更)"
        )
        metadata_rows.append(
            (
                item.get("category", ""),
                item.get("code", ""),
                description,
            )
        )

    diagnostic_table = _format_table(["category", "code", "kind"], diagnostic_rows)
    metadata_table = _format_table(["category", "code", "changed"], metadata_rows)

    if template is None:
        # fallback rendering
        rows = [
            ("既存診断の退行数", diagnostic["regressions"]),
            ("新規診断数", diagnostic["new"]),
            ("メタデータ変更数", metadata["changed"]),
            ("pass_rate 変化", pass_rate.get("delta")),
        ]
        html_lines = [
            "<!DOCTYPE html>",
            "<html>",
            "<head>",
            '  <meta charset="utf-8" />',
            "  <title>Audit Diff Summary</title>",
            '  <style>table{border-collapse:collapse;}td,th{border:1px solid #ccc;padding:4px 8px;}</style>',
            "</head>",
            "<body>",
            "<h1>Audit Diff Summary</h1>",
            "<table>",
        ]
        for label, value in rows:
            html_lines.append(f"<tr><th>{label}</th><td>{value}</td></tr>")
        html_lines.append("</table>")
        html_lines.append(diagnostic_table)
        html_lines.append(metadata_table)
        html_lines.append("</body></html>")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(html_lines), encoding="utf-8")
        return

    html = template
    html = html.replace("{{generated_at}}", datetime.now().astimezone().isoformat())
    html = html.replace("{{diagnostic_regressions}}", str(diagnostic.get("regressions")))
    html = html.replace("{{diagnostic_new}}", str(diagnostic.get("new")))
    html = html.replace("{{metadata_changed}}", str(metadata.get("changed")))
    html = html.replace("{{pass_rate_delta}}", str(pass_rate.get("delta")))
    html = html.replace("{{base_path}}", str(base.get("path")))
    html = html.replace("{{base_entries}}", str(base.get("entry_count")))
    html = html.replace("{{target_path}}", str(target.get("path")))
    html = html.replace("{{target_entries}}", str(target.get("entry_count")))
    html = html.replace("{{diagnostic_table}}", diagnostic_table)
    html = html.replace("{{metadata_table}}", metadata_table)

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(html, encoding="utf-8")


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(argv)
    base_entries = load_entries(args.base)
    target_entries = load_entries(args.target)

    if args.query:
        base_entries = filter_entries(base_entries, args.query)
        target_entries = filter_entries(target_entries, args.query)

    summary = _compare_entries(base_entries, target_entries)
    schema_version = "audit-diff.v1"
    output_data = {
        "schema_version": schema_version,
        "base": {"path": str(args.base), "entry_count": len(base_entries)},
        "target": {"path": str(args.target), "entry_count": len(target_entries)},
        "diagnostic": summary["diagnostic"],
        "metadata": summary["metadata"],
        "pass_rate": summary["pass_rate"],
        "threshold": args.threshold,
    }
    if args.preset_name:
        output_data["preset"] = args.preset_name

    output_dir = _output_directory(args.target, args.output)
    formats = {item.strip().lower() for item in args.format.split(",") if item.strip()}

    if "json" in formats:
        _write_json(output_dir / "diff.json", output_data)
    if "md" in formats:
        _write_markdown(output_dir / "diff.md", output_data)
    if "html" in formats:
        _write_html(output_dir / "diff.html", output_data)

    print(json.dumps(output_data, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""
Audit dashboard generator.

Aggregates metrics JSON written by collect-iterator-audit-metrics.py and produces
snapshot/timeseries summaries plus optional HTML/Markdown reports.
"""

from __future__ import annotations

import argparse
import csv
import json
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate audit dashboard assets.")
    parser.add_argument(
        "--metrics",
        action="append",
        required=True,
        help="Path to metrics JSON (collect-iterator-audit-metrics output). Repeatable.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("reports/audit/dashboard"),
        help="Output directory (default: reports/audit/dashboard).",
    )
    parser.add_argument(
        "--export",
        type=Path,
        help="Write combined metrics JSON to the specified path "
        "(default: <output>/metrics.snapshot.json).",
    )
    parser.add_argument(
        "--render",
        action="store_true",
        help="Render HTML/Markdown dashboard summaries.",
    )
    return parser.parse_args(argv)


def _load_metrics(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def _file_timestamp(path: Path) -> str:
    try:
        ts = datetime.fromtimestamp(path.stat().st_mtime).astimezone()
    except OSError:
        ts = datetime.now().astimezone()
    return ts.isoformat()


def _flatten_metrics(data: Dict[str, Any], source: Path) -> List[Dict[str, Any]]:
    metrics = data.get("metrics")
    timestamp = data.get("generated_at") or _file_timestamp(source)
    if not isinstance(metrics, list):
        return []
    flattened: List[Dict[str, Any]] = []
    for item in metrics:
        if not isinstance(item, dict):
            continue
        entry = {
            "metric": item.get("metric"),
            "pass_rate": item.get("pass_rate"),
            "total": item.get("total"),
            "passed": item.get("passed"),
            "failed": item.get("failed"),
            "source": str(source),
            "timestamp": timestamp,
        }
        if item.get("metric") == "audit_review.summary":
            audit_diff = item.get("audit_diff") or {}
            entry.update(
                {
                    "review_regressions": audit_diff.get("total_regressions"),
                    "review_metadata_changed": audit_diff.get("metadata_changed"),
                    "review_pass_rate_delta": (audit_diff.get("pass_rate") or {}).get(
                        "delta"
                    ),
                    "review_sources": audit_diff.get("sources"),
                }
            )
            audit_query = item.get("audit_query") or {}
            entry.update(
                {
                    "review_coverage": audit_query.get("coverage"),
                    "review_coverage_matched": audit_query.get("matched"),
                    "review_coverage_total": audit_query.get("total"),
                }
            )
        flattened.append(entry)
    return flattened


def _aggregate_timeseries(
    flattened: Iterable[Dict[str, Any]]
) -> List[Dict[str, Any]]:
    rows: List[Dict[str, Any]] = []
    for index, entry in enumerate(flattened, start=1):
        row = {"index": index}
        row.update(entry)
        rows.append(row)
    return rows


def _write_snapshot(path: Path, entries: List[Dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump({"generated_at": datetime.now().astimezone().isoformat(), "entries": entries}, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def _write_timeseries_csv(path: Path, entries: List[Dict[str, Any]]) -> None:
    if not entries:
        return
    headers = sorted({key for entry in entries for key in entry.keys()})
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=headers)
        writer.writeheader()
        for entry in entries:
            writer.writerow(entry)


def _render_markdown(path: Path, entries: List[Dict[str, Any]]) -> None:
    lines = ["# Audit Dashboard", ""]
    if not entries:
        lines.append("データがありません。")
    else:
        latest = entries[-1]
        lines.append("## 最新スナップショット")
        lines.append("")
        lines.append("| 指標 | 値 |")
        lines.append("|------|----|")
        for key, label in (
            ("metric", "metric"),
            ("pass_rate", "pass_rate"),
            ("total", "total"),
            ("passed", "passed"),
            ("failed", "failed"),
            ("review_regressions", "review.regressions"),
            ("review_coverage", "review.coverage"),
            ("timestamp", "timestamp"),
            ("average_expected_tokens", "parser.avg_tokens"),
            ("min_expected_tokens", "parser.min_tokens"),
            ("max_expected_tokens", "parser.max_tokens"),
        ):
            if key in latest and latest[key] is not None:
                lines.append(f"| {label} | {latest[key]} |")
        lines.append("")
        lines.append("## サマリ一覧")
        lines.append("")
        lines.append("| # | metric | pass_rate | total | passed | failed | timestamp |")
        lines.append("|---|--------|-----------|-------|--------|--------|-----------|")
        for entry in entries:
            lines.append(
                f"| {entry.get('index','')} | {entry.get('metric','')} | "
                f"{entry.get('pass_rate','')} | {entry.get('total','')} | "
                f"{entry.get('passed','')} | {entry.get('failed','')} | "
                f"{entry.get('timestamp','')} |"
            )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _render_html(path: Path, entries: List[Dict[str, Any]]) -> None:
    timestamp = datetime.now().astimezone().isoformat()
    rows_html = "\n".join(
        "<tr>"
        + "".join(f"<td>{entry.get(key,'')}</td>" for key in ("index", "metric", "pass_rate", "total", "passed", "failed", "timestamp"))
        + "</tr>"
        for entry in entries
    )
    extra_html = ""
    if entries:
        latest = entries[-1]
        extra_items = []
        for key, label in (
            ("average_expected_tokens", "parser.avg_tokens"),
            ("min_expected_tokens", "parser.min_tokens"),
            ("max_expected_tokens", "parser.max_tokens"),
        ):
            value = latest.get(key)
            if value is not None:
                extra_items.append(f"<li>{label}: {value}</li>")
        if extra_items:
            extra_html = "<h2>Parser Metrics</h2><ul>{items}</ul>".format(
                items="".join(extra_items)
            )
    html = f"""<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <title>Audit Dashboard</title>
  <style>
    body {{ font-family: sans-serif; margin: 1.5rem; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{ border: 1px solid #ccc; padding: 0.4rem 0.6rem; text-align: left; }}
    th {{ background: #f5f5f5; }}
  </style>
</head>
<body>
  <h1>Audit Dashboard</h1>
  <p>Generated at: {timestamp}</p>
  <table>
    <thead>
      <tr><th>#</th><th>metric</th><th>pass_rate</th><th>total</th><th>passed</th><th>failed</th><th>timestamp</th></tr>
    </thead>
    <tbody>
      {rows_html}
    </tbody>
  </table>
  {extra_html}
</body>
</html>
"""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(html, encoding="utf-8")


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(argv)
    output_dir = args.output
    metrics_entries: List[Dict[str, Any]] = []

    for metric_path_str in args.metrics:
        metric_path = Path(metric_path_str)
        if not metric_path.is_file():
            raise FileNotFoundError(f"Metrics file not found: {metric_path}")
        data = _load_metrics(metric_path)
        metrics_entries.extend(_flatten_metrics(data, metric_path))

    timeseries = _aggregate_timeseries(metrics_entries)
    export_path = args.export or (output_dir / "metrics.snapshot.json")
    _write_snapshot(export_path, timeseries)
    _write_timeseries_csv(output_dir / "metrics.timeseries.csv", timeseries)

    if args.render:
        _render_markdown(output_dir / "index.md", timeseries)
        _render_html(output_dir / "index.html", timeseries)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env bash
# Audit regression detector prototype.

set -euo pipefail

python3 - "$@" <<'PY'
import argparse
import json
import math
import statistics
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional


def load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def extract_metric(data: Dict[str, Any], dotted_key: str) -> Optional[float]:
    cursor: Any = data
    for part in dotted_key.split("."):
        if isinstance(cursor, dict) and part in cursor:
            cursor = cursor[part]
        else:
            return None
    if isinstance(cursor, (int, float)):
        return float(cursor)
    return None


def compute_ratio(current: Optional[float], baseline: Optional[float]) -> Optional[float]:
    if current is None:
        return None
    if baseline is None:
        return None
    if baseline == 0:
        return math.inf if current != 0 else 0.0
    return (current - baseline) / baseline


def format_float(value: Optional[float]) -> str:
    if value is None:
        return "-"
    if math.isinf(value):
        return "∞"
    return f"{value:.3f}"


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Detect regressions in audit metrics JSON."
    )
    parser.add_argument(
        "--current",
        required=True,
        type=Path,
        help="Current run metrics JSON.",
    )
    parser.add_argument(
        "--history",
        action="append",
        default=[],
        type=Path,
        help="Baseline metrics JSON (repeatable).",
    )
    parser.add_argument(
        "--threshold-error",
        type=float,
        default=0.10,
        help="Relative threshold for error/warning/count metrics (default: 0.10).",
    )
    parser.add_argument(
        "--threshold-duration",
        type=float,
        default=0.15,
        help="Relative threshold for duration metrics (default: 0.15).",
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="Write Markdown report to this path.",
    )
    parser.add_argument(
        "--fail-on-regression",
        action="store_true",
        help="Return exit code 1 when a regression is detected.",
    )
    args = parser.parse_args(argv)

    if not args.current.is_file():
        parser.error(f"Current metrics file not found: {args.current}")

    history_values: Dict[str, List[float]] = {}
    metric_plan = [
        ("diagnostics.error.count", args.threshold_error, "error_count"),
        ("diagnostics.warning.count", args.threshold_error, "warning_count"),
        ("ffi_bridge.status_summary.failure", args.threshold_error, "bridge_status_failure"),
        ("ci.duration.total_seconds", args.threshold_duration, "ci_duration_seconds"),
    ]

    for history_path in args.history:
        if not history_path.is_file():
            continue
        try:
            history_data = load_json(history_path)
        except Exception:
            continue
        for key, _, _ in metric_plan:
            value = extract_metric(history_data, key)
            if value is not None:
                history_values.setdefault(key, []).append(value)

    current_data = load_json(args.current)

    results = []
    for key, threshold, label in metric_plan:
        current_value = extract_metric(current_data, key)
        history_list = history_values.get(key, [])
        baseline_value: Optional[float]
        if history_list:
            baseline_value = statistics.mean(history_list)
        else:
            baseline_value = None

        ratio = compute_ratio(current_value, baseline_value)
        exceeded = False
        if ratio is not None:
            if math.isinf(ratio):
                exceeded = True
            elif ratio > threshold:
                exceeded = True
        result = {
            "key": key,
            "label": label,
            "current": current_value,
            "baseline": baseline_value,
            "ratio": ratio,
            "threshold": threshold,
            "history_samples": len(history_list),
            "exceeded": bool(exceeded),
        }
        results.append(result)

    has_regression = any(item["exceeded"] for item in results)

    for item in results:
        status = "REGRESSION" if item["exceeded"] else "OK"
        print(
            f"{status}: {item['key']} "
            f"current={format_float(item['current'])} "
            f"baseline={format_float(item['baseline'])} "
            f"ratio={format_float(item['ratio'])} "
            f"threshold={item['threshold']:.2f} "
            f"samples={item['history_samples']}"
        )

    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        lines = [
            "# レグレッション検出レポート",
            "",
            f"- 現在のメトリクス: `{args.current}`",
            f"- 既存比較サンプル数: {sum(len(history_values.get(key, [])) for key, _, _ in metric_plan)}",
            "",
            "| メトリクス | 現在値 | 基準値 | 比率 | 閾値 | 判定 |",
            "| --- | ---: | ---: | ---: | ---: | --- |",
        ]
        for item in results:
            status = "⚠️" if item["exceeded"] else "✅"
            lines.append(
                f"| `{item['key']}` | {format_float(item['current'])} | "
                f"{format_float(item['baseline'])} | {format_float(item['ratio'])} | "
                f"{item['threshold']:.2f} | {status} |"
            )
        lines.append("")
        if args.history:
            lines.append("## 参照履歴ファイル")
            lines.append("")
            for path in args.history:
                lines.append(f"- `{path}`")
            lines.append("")
        args.report.write_text("\n".join(lines), encoding="utf-8")

    if has_regression and args.fail_on_regression:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
PY

#!/usr/bin/env python3

"""
Phase 4 spec/practical suites runner.

This script reads docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv
and executes reml_frontend for scenarios that belong to the requested suite.
Execution results (diagnostic codes) are compared against the expected
diagnostic_keys column, and a Markdown report is written under
reports/spec-audit/ch4/.
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable, List, Sequence


SUITE_PREFIXES = {
    "spec_core": ("examples/spec_core/",),
    "practical": ("examples/practical/",),
    "language_impl_samples": ("examples/language-impl-samples/",),
}

REQUIRED_SUBDIRS = {
    "spec_core": (
        "examples/spec_core/chapter1/control_flow/",
        "examples/spec_core/chapter1/literals/",
        "examples/spec_core/chapter1/lambda/",
    ),
    "practical": (),
    "language_impl_samples": (),
}

SUITE_REPORT = {
    "spec_core": "spec-core-dashboard.md",
    "practical": "practical-suite-index.md",
    "language_impl_samples": "language-impl-samples-dashboard.md",
}


@dataclass
class Scenario:
    scenario_id: str
    input_path: Path
    expected_codes: List[str]
    scenario_notes: str
    resolution: str


@dataclass
class ScenarioResult:
    scenario: Scenario
    actual_codes: List[str]
    exit_code: int
    succeeded: bool
    error_message: str | None = None
    stdout: str = ""
    stderr: str = ""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", required=True, choices=sorted(SUITE_PREFIXES))
    parser.add_argument(
        "--root",
        default=Path(__file__).resolve().parents[2],
        type=Path,
        help="Repository root (auto-detected by default)",
    )
    parser.add_argument(
        "--allow-failures",
        action="store_true",
        help="Exit 0 even if scenarios fail (useful for generating reports)",
    )
    return parser.parse_args()


def load_scenarios(root: Path, suite: str) -> List[Scenario]:
    matrix_path = (
        root / "docs" / "plans" / "bootstrap-roadmap" / "assets" / "phase4-scenario-matrix.csv"
    )
    prefixes = SUITE_PREFIXES[suite]
    scenarios: List[Scenario] = []
    with matrix_path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        for row in reader:
            input_path = row.get("input_path", "").strip()
            if not input_path:
                continue
            if not any(input_path.startswith(prefix) for prefix in prefixes):
                continue
            diagnostic_keys = row.get("diagnostic_keys", "").strip() or "[]"
            try:
                expected_codes = json.loads(diagnostic_keys)
                if not isinstance(expected_codes, list):
                    raise ValueError
            except ValueError as err:
                raise SystemExit(
                    f"diagnostic_keys の JSON 解析に失敗しました (scenario_id={row.get('scenario_id')}): {err}"
                )
            scenario = Scenario(
                scenario_id=row.get("scenario_id", "").strip(),
                input_path=(root / input_path).resolve(),
                expected_codes=[code for code in expected_codes if code],
                scenario_notes=row.get("scenario_notes", "").strip(),
                resolution=row.get("resolution", "").strip(),
            )
            scenarios.append(scenario)
    if not scenarios:
        raise SystemExit(f"{suite}: 対象シナリオが見つかりません (prefixes={prefixes})")
    required_dirs = REQUIRED_SUBDIRS.get(suite, ())
    if required_dirs:
        coverage = {prefix: False for prefix in required_dirs}
        for scenario in scenarios:
            try:
                rel_path = scenario.input_path.relative_to(root).as_posix()
            except ValueError:
                rel_path = scenario.input_path.as_posix()
            for prefix in required_dirs:
                normalized = prefix if prefix.endswith("/") else f"{prefix}/"
                if rel_path.startswith(normalized):
                    coverage[prefix] = True
        missing = [prefix for prefix, hit in coverage.items() if not hit]
        if missing:
            missing_lines = "\n  - ".join(missing)
            raise SystemExit(
                f"{suite}: Phase4 Missing Examples ディレクトリに紐付くシナリオが不足しています。\n  - {missing_lines}"
            )
    return scenarios


def run_reml_frontend(root: Path, scenario: Scenario) -> ScenarioResult:
    manifest_path = root / "compiler" / "rust" / "frontend" / "Cargo.toml"
    base_cmd: Sequence[str] = (
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(manifest_path),
        "--bin",
        "reml_frontend",
        "--",
    )
    if scenario.scenario_id == "CP-WS2-001":
        cmd: Sequence[str] = (
            *base_cmd,
            "--parse-driver",
            "--parse-driver-label",
            "expression",
            "--output",
            "json",
            str(scenario.input_path),
        )
    elif scenario.scenario_id == "CP-WS3-001":
        cmd = (
            *base_cmd,
            "--parse-driver",
            "--output",
            "json",
            str(scenario.input_path),
        )
    elif scenario.scenario_id == "CP-WS6-001":
        cmd = (
            *base_cmd,
            "--parse-driver",
            "--parse-driver-left-recursion-parser",
            "--parse-driver-packrat",
            "on",
            "--parse-driver-left-recursion",
            "off",
            "--output",
            "json",
            str(scenario.input_path),
        )
    elif scenario.scenario_id == "CP-WS6-002":
        profile_output = (
            root
            / "expected"
            / "spec_core"
            / "chapter2"
            / "parser_core"
            / "core-parse-left-recursion-slow.profile.json"
        )
        cmd = (
            *base_cmd,
            "--parse-driver",
            "--parse-driver-left-recursion-parser",
            "--parse-driver-packrat",
            "on",
            "--parse-driver-left-recursion",
            "on",
            "--parse-driver-profile-output",
            str(profile_output),
            "--output",
            "json",
            str(scenario.input_path),
        )
    else:
        cmd = (
            *base_cmd,
            "--output",
            "json",
            str(scenario.input_path),
        )
    completed = subprocess.run(
        cmd,
        cwd=root,
        capture_output=True,
        text=True,
    )
    stdout = completed.stdout.strip()
    stderr = completed.stderr.strip()
    actual_codes: List[str] = []
    seen_codes: set[str] = set()
    error_message: str | None = None
    if stdout:
        try:
            envelope = json.loads(stdout)
            diagnostics = envelope.get("diagnostics", [])
            if isinstance(diagnostics, list):
                for diag in diagnostics:
                    if not isinstance(diag, dict):
                        continue
                    code = diag.get("code")
                    if not code and isinstance(diag.get("codes"), list):
                        if diag["codes"]:
                            code = diag["codes"][0]
                    if code:
                        if code not in seen_codes:
                            actual_codes.append(code)
                            seen_codes.add(code)
        except json.JSONDecodeError as err:
            error_message = f"CLI 出力の JSON 化に失敗しました: {err}"
    else:
        error_message = "CLI から JSON 出力が得られませんでした"

    expect_success = len(scenario.expected_codes) == 0
    succeeded = (
        error_message is None
        and sorted(actual_codes) == sorted(scenario.expected_codes)
        and (not expect_success or completed.returncode == 0)
    )
    return ScenarioResult(
        scenario=scenario,
        actual_codes=actual_codes,
        exit_code=completed.returncode,
        succeeded=succeeded,
        error_message=error_message,
        stdout=stdout,
        stderr=stderr,
    )


def format_codes(codes: Iterable[str]) -> str:
    values = [f"`{code}`" for code in codes]
    return "<br>".join(values) if values else "—"


def write_report(root: Path, suite: str, results: Sequence[ScenarioResult]) -> Path:
    report_dir = root / "reports" / "spec-audit" / "ch4"
    report_dir.mkdir(parents=True, exist_ok=True)
    report_path = report_dir / SUITE_REPORT[suite]

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%SZ")
    total = len(results)
    passed = sum(1 for r in results if r.succeeded)
    failed = total - passed

    lines = [
        f"# {suite} スイート実行レポート",
        "",
        f"- 実行時刻: {ts}",
        f"- 対象シナリオ: {total} 件 / 成功 {passed} 件 / 失敗 {failed} 件",
        "- 入力ソース: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`",
        "",
        "| Scenario | File | 期待 Diagnostics | 実際 Diagnostics | Exit | 判定 | 備考 |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]

    for result in results:
        scenario = result.scenario
        expected = format_codes(scenario.expected_codes)
        actual = format_codes(result.actual_codes)
        status = "✅ pass" if result.succeeded else "❌ fail"
        notes = scenario.scenario_notes or scenario.resolution or ""
        if result.error_message:
            notes = f"{notes} (error: {result.error_message})".strip()
        try:
            rel_path = scenario.input_path.relative_to(root)
        except ValueError:
            rel_path = scenario.input_path
        lines.append(
            f"| `{scenario.scenario_id}` | `{rel_path}` | {expected} | {actual} | {result.exit_code} | {status} | {notes or '—'} |"
        )

    report_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return report_path


def write_failure_log(
    root: Path, suite: str, results: Sequence[ScenarioResult], report_path: Path
) -> Path:
    log_dir = root / "reports" / "spec-audit" / "ch4" / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    ts = datetime.now(timezone.utc)
    timestamp_str = ts.strftime("%Y%m%dT%H%M%SZ")
    log_path = log_dir / f"{suite}-{timestamp_str}.md"

    total = len(results)
    passed = sum(1 for r in results if r.succeeded)
    failed = total - passed

    lines = [
        f"# Phase 4 {suite} 失敗ログ",
        "",
        f"- 生成時刻: {ts.strftime('%Y-%m-%d %H:%M:%SZ')}",
        f"- レポート: {report_path}",
        f"- 件数: {total} 件 / 成功 {passed} 件 / 失敗 {failed} 件",
        "",
        "## 失敗シナリオ詳細",
        "",
    ]
    for res in results:
        if res.succeeded:
            continue
        scenario = res.scenario
        try:
            rel_path = scenario.input_path.relative_to(root)
        except ValueError:
            rel_path = scenario.input_path
        lines.extend(
            [
                f"### {scenario.scenario_id}",
                "",
                f"- ファイル: `{rel_path}`",
                f"- 期待 Diagnostics: {format_codes(scenario.expected_codes)}",
                f"- 実際 Diagnostics: {format_codes(res.actual_codes)}",
                f"- Exit code: {res.exit_code}",
                f"- 備考: {scenario.scenario_notes or scenario.resolution or '—'}",
            ]
        )
        if res.error_message:
            lines.append(f"- エラー: {res.error_message}")
        lines.extend(
            [
                "",
                "#### stdout",
                "```",
                res.stdout.strip() or "(empty)",
                "```",
                "",
                "#### stderr",
                "```",
                res.stderr.strip() or "(empty)",
                "```",
                "",
            ]
        )
    log_path.write_text("\n".join(lines), encoding="utf-8")
    return log_path


def main() -> None:
    args = parse_args()
    root: Path = args.root.resolve()
    scenarios = load_scenarios(root, args.suite)
    results = [run_reml_frontend(root, scenario) for scenario in scenarios]
    report_path = write_report(root, args.suite, results)

    failures = [res for res in results if not res.succeeded]
    if failures:
        failure_log_path = write_failure_log(root, args.suite, results, report_path)
        summary_lines = [
            f"{args.suite}: {len(failures)} 件のシナリオが失敗しました。",
            f"レポート: {report_path}",
            f"ログ: {failure_log_path}",
        ]
        for res in failures[:10]:
            summary_lines.append(
                f"- {res.scenario.scenario_id}: expected {res.scenario.expected_codes} / actual {res.actual_codes} (exit {res.exit_code})"
            )
            if res.error_message:
                summary_lines.append(f"    error: {res.error_message}")
        message = "\n".join(summary_lines)
        if args.allow_failures:
            print(message, file=sys.stderr)
        else:
            raise SystemExit(message)
    else:
        print(f"{args.suite}: all {len(results)} scenarios passed. Report -> {report_path}")


if __name__ == "__main__":
    main()

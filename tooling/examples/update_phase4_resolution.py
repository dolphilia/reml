#!/usr/bin/env python3

"""
Phase 4 シナリオマトリクス自動同期ツール。

`tooling/examples/run_phase4_suite.py` が収集する ScenarioResult をもとに
`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` 内の
resolution/resolution_notes を自動更新する。
"""

from __future__ import annotations

import argparse
import csv
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Mapping, Sequence

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import run_phase4_suite  # noqa: E402


SuiteName = str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "phase4-scenario-matrix.csv を spec_core/practical スイートの最新結果で更新する補助スクリプト。"
        )
    )
    parser.add_argument(
        "--suite",
        default="all",
        choices=["spec_core", "practical", "all"],
        help="対象スイート。all を指定すると両方を実行する。",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="リポジトリルート（自動検出が既定）。",
    )
    parser.add_argument(
        "--failure-resolution",
        choices=["impl_fix", "spec_fix", "pending"],
        default="impl_fix",
        help="失敗シナリオに設定する resolution/spec_vs_impl_decision の値。",
    )
    parser.add_argument(
        "--notes-prefix",
        default="auto-sync",
        help="resolution_notes に追加するログのプレフィックス。",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="CSV を書き換えずに更新内容のみを表示する。",
    )
    return parser.parse_args()


def collect_results(root: Path, suites: Sequence[SuiteName]) -> Mapping[SuiteName, List[run_phase4_suite.ScenarioResult]]:
    results: Dict[SuiteName, List[run_phase4_suite.ScenarioResult]] = {}
    for suite in suites:
        scenarios = run_phase4_suite.load_scenarios(root, suite)
        suite_results = [run_phase4_suite.run_reml_frontend(root, scenario) for scenario in scenarios]
        results[suite] = suite_results
    return results


def append_note(existing: str, new_note: str) -> str:
    existing = (existing or "").strip()
    if not existing:
        return new_note
    return f"{existing} | {new_note}"


def build_note(prefix: str, suite: SuiteName, result: run_phase4_suite.ScenarioResult) -> str:
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%SZ")
    status = "pass" if result.succeeded else "fail"
    expected = ",".join(result.scenario.expected_codes) if result.scenario.expected_codes else "[]"
    actual = ",".join(result.actual_codes) if result.actual_codes else "[]"
    note = (
        f"[{prefix} {ts} {suite}] status={status} expected={expected} "
        f"actual={actual} exit={result.exit_code}"
    )
    if result.error_message:
        note = f"{note} error={result.error_message}"
    return note


def update_matrix(
    matrix_path: Path,
    results_by_suite: Mapping[SuiteName, List[run_phase4_suite.ScenarioResult]],
    failure_resolution: str,
    notes_prefix: str,
    dry_run: bool,
) -> Dict[str, List[str]]:
    with matrix_path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        fieldnames = reader.fieldnames or []
        rows = [row for row in reader]

    index: Dict[str, Dict[str, str]] = {}
    for row in rows:
        scenario_id = row.get("scenario_id", "").strip()
        if scenario_id:
            index[scenario_id] = row

    updated: List[str] = []
    missing: List[str] = []

    for suite, results in results_by_suite.items():
        for result in results:
            scenario_id = result.scenario.scenario_id
            row = index.get(scenario_id)
            if not row:
                missing.append(scenario_id)
                continue
            desired_resolution = "ok" if result.succeeded else failure_resolution
            desired_decision = desired_resolution
            note = build_note(notes_prefix, suite, result)

            changed = False
            if row.get("resolution", "").strip() != desired_resolution:
                row["resolution"] = desired_resolution
                changed = True
            if row.get("spec_vs_impl_decision", "").strip() != desired_decision:
                row["spec_vs_impl_decision"] = desired_decision
                changed = True
            new_notes = append_note(row.get("resolution_notes", ""), note)
            if new_notes != row.get("resolution_notes", ""):
                row["resolution_notes"] = new_notes
                changed = True
            if changed:
                updated.append(scenario_id)

    if not dry_run and updated:
        with matrix_path.open("w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(rows)

    return {"updated": updated, "missing": missing}


def format_summary(results_by_suite: Mapping[SuiteName, List[run_phase4_suite.ScenarioResult]]) -> str:
    lines: List[str] = []
    for suite, results in results_by_suite.items():
        total = len(results)
        passed = sum(1 for r in results if r.succeeded)
        failed = total - passed
        lines.append(f"- {suite}: {passed}/{total} pass (fail {failed})")
    return "\n".join(lines)


def main() -> None:
    args = parse_args()
    root = args.root.resolve()
    suites: Sequence[SuiteName]
    if args.suite == "all":
        suites = ("spec_core", "practical")
    else:
        suites = (args.suite,)

    results_by_suite = collect_results(root, suites)
    matrix_path = (
        root / "docs" / "plans" / "bootstrap-roadmap" / "assets" / "phase4-scenario-matrix.csv"
    )
    summary = update_matrix(
        matrix_path,
        results_by_suite,
        failure_resolution=args.failure_resolution,
        notes_prefix=args.notes_prefix,
        dry_run=args.dry_run,
    )

    print("=== Phase4 Scenario Matrix Sync ===")
    print(format_summary(results_by_suite))
    print(f"Updated rows: {len(summary['updated'])} -> {summary['updated']}")
    if summary["missing"]:
        print(f"Missing scenario_id: {summary['missing']}", file=sys.stderr)
        if not args.dry_run:
            sys.exit(1)


if __name__ == "__main__":
    main()

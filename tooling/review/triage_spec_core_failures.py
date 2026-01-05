#!/usr/bin/env python3
"""
Phase 4 spec_core/practical 失敗シナリオの自動切り分けツール。

reports/spec-audit/ch4/logs/ 内の Markdown ログを解析し、
diagnostic_keys / expected と突き合わせて example_fix / impl_fix / spec_fix 判定を行う。
"""

from __future__ import annotations

import argparse
import csv
import json
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple


FieldDict = Dict[str, str]


@dataclass
class LogScenario:
    scenario_id: str
    stdout: str
    stderr: str
    exit_code: Optional[int]
    error_message: Optional[str]
    actual_codes: List[str]
    cli_command: Optional[str]
    stdout_parse_error: Optional[str]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Phase4 spec_core/practical の失敗ログを解析し、phase4-scenario-matrix.csv を自動更新します。"
    )
    parser.add_argument(
        "--log",
        required=True,
        type=Path,
        help="reports/spec-audit/ch4/logs/ 内の Markdown ログ",
    )
    parser.add_argument(
        "--matrix",
        type=Path,
        default=Path(__file__).resolve().parents[1]
        / "docs"
        / "plans"
        / "bootstrap-roadmap"
        / "assets"
        / "phase4-scenario-matrix.csv",
        help="シナリオマトリクス CSV（省略時は repo 既定）",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="リポジトリルート（ログ・expected の相対パス計算用）",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="実際に phase4-scenario-matrix.csv を更新する（省略時は dry-run）",
    )
    parser.add_argument(
        "--include-status",
        type=str,
        default="pending",
        help="更新対象とする resolution 値（カンマ区切り、既定: pending のみ）",
    )
    parser.add_argument(
        "--suite",
        choices=("spec_core", "practical"),
        default="spec_core",
        help="ログの対象スイート（出力メッセージ用）",
    )
    return parser.parse_args()


def load_lines(path: Path) -> List[str]:
    return path.read_text(encoding="utf-8").splitlines()


def parse_log(log_path: Path) -> Dict[str, LogScenario]:
    lines = load_lines(log_path)
    entries: Dict[str, LogScenario] = {}
    current: Dict[str, object] | None = None
    idx = 0

    def flush_current() -> None:
        nonlocal current
        if not current:
            return
        scenario_id = str(current.get("scenario_id", ""))
        stdout = str(current.get("stdout", ""))
        stderr = str(current.get("stderr", ""))
        exit_code = current.get("exit_code")
        exit_val = int(exit_code) if isinstance(exit_code, int) else None
        stdout_parse_error = None
        cli_command = None
        actual_codes: List[str] = []
        if stdout:
            try:
                payload = json.loads(stdout)
                cli_command = (
                    payload.get("summary", {})
                    .get("stats", {})
                    .get("cli_command")
                )
                diagnostics = payload.get("diagnostics", [])
                if isinstance(diagnostics, list):
                    for diag in diagnostics:
                        if not isinstance(diag, dict):
                            continue
                        code = diag.get("code")
                        if not code and isinstance(diag.get("codes"), list):
                            codes = diag.get("codes")
                            if codes:
                                code = codes[0]
                        if code:
                            actual_codes.append(str(code))
            except json.JSONDecodeError as err:
                stdout_parse_error = f"stdout JSON 解析に失敗: {err}"
        entries[scenario_id] = LogScenario(
            scenario_id=scenario_id,
            stdout=stdout,
            stderr=stderr,
            exit_code=exit_val,
            error_message=str(current.get("error_message") or "") or None,
            actual_codes=actual_codes,
            cli_command=cli_command,
            stdout_parse_error=stdout_parse_error,
        )
        current = None

    def parse_block(start: int) -> Tuple[str, int]:
        idx = start
        body: List[str] = []
        if idx >= len(lines):
            return "", idx
        if lines[idx].strip() != "```":
            return "", idx
        idx += 1
        while idx < len(lines) and lines[idx].strip() != "```":
            body.append(lines[idx])
            idx += 1
        # skip closing fence
        if idx < len(lines) and lines[idx].strip() == "```":
            idx += 1
        return "\n".join(body).strip(), idx

    while idx < len(lines):
        line = lines[idx]
        if line.startswith("### "):
            flush_current()
            scenario_id = line[4:].strip()
            current = {"scenario_id": scenario_id, "stdout": "", "stderr": ""}
            idx += 1
            continue
        if not current:
            idx += 1
            continue
        if line.startswith("- Exit code:"):
            try:
                current["exit_code"] = int(line.split(":", 1)[1].strip())
            except ValueError:
                current["exit_code"] = None
            idx += 1
            continue
        if line.startswith("- エラー:"):
            current["error_message"] = line.split(":", 1)[1].strip()
            idx += 1
            continue
        stripped = line.strip()
        if stripped == "#### stdout":
            stdout, next_idx = parse_block(idx + 1)
            current["stdout"] = stdout
            idx = next_idx
            continue
        if stripped == "#### stderr":
            stderr, next_idx = parse_block(idx + 1)
            current["stderr"] = stderr
            idx = next_idx
            continue
        idx += 1
    flush_current()
    # 空 ID を除外
    return {k: v for k, v in entries.items() if k}


def load_matrix(matrix_path: Path) -> Tuple[List[str], List[FieldDict]]:
    with matrix_path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        rows = [dict(row) for row in reader]
        if not reader.fieldnames:
            raise SystemExit("matrix: ヘッダを読み取れませんでした")
        return list(reader.fieldnames), rows


def parse_expected_codes(raw: str) -> List[str]:
    raw = (raw or "").strip()
    if not raw:
        return []
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as err:
        raise SystemExit(f"diagnostic_keys の JSON 解析に失敗: {err} (value={raw})")
    if not isinstance(parsed, list):
        raise SystemExit(f"diagnostic_keys は配列である必要があります (value={raw})")
    return [str(item) for item in parsed if item]


def classify_resolution(
    expected_codes: Sequence[str],
    entry: LogScenario,
) -> Tuple[Optional[str], Optional[str]]:
    if entry.stdout_parse_error:
        return "impl_fix", entry.stdout_parse_error
    if entry.error_message:
        return "impl_fix", entry.error_message
    actual_codes = entry.actual_codes
    exp_sorted = sorted(expected_codes)
    act_sorted = sorted(actual_codes)
    if not expected_codes:
        if act_sorted:
            return (
                "impl_fix",
                "期待診断なしのシナリオで Diagnostics が発生しました",
            )
        if entry.exit_code not in (None, 0):
            return (
                "impl_fix",
                f"期待診断なしのシナリオが exit code {entry.exit_code} で終了しました",
            )
        return None, None
    if not actual_codes:
        return "example_fix", "期待された Diagnostics が出力されていません"
    if exp_sorted != act_sorted:
        return "spec_fix", "Diagnostics コードの集合が期待と一致しません"
    if entry.exit_code not in (None, 0):
        return (
            "impl_fix",
            f"Diagnostics は一致しましたが exit code {entry.exit_code} が返されました",
        )
    return None, None


def append_note(existing: str, new_note: str) -> str:
    existing = (existing or "").strip()
    if not existing:
        return new_note
    return f"{existing} | {new_note}"


def main() -> None:
    args = parse_args()
    log_path = args.log.resolve()
    matrix_path = args.matrix.resolve()
    root = args.root.resolve()

    log_entries = parse_log(log_path)
    fieldnames, rows = load_matrix(matrix_path)
    index = {row.get("scenario_id", "").strip(): row for row in rows}

    accepted_statuses = {
        status.strip() for status in args.include_status.split(",") if status.strip()
    }
    if not accepted_statuses:
        accepted_statuses = {"pending"}

    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    updated: List[str] = []
    skipped_missing: List[str] = []

    for scenario_id, entry in log_entries.items():
        row = index.get(scenario_id)
        if not row:
            skipped_missing.append(scenario_id)
            continue
        current_resolution = (row.get("resolution") or "").strip().lower()
        if current_resolution and current_resolution not in accepted_statuses:
            continue
        expected_codes = parse_expected_codes(row.get("diagnostic_keys", ""))
        new_resolution, reason = classify_resolution(expected_codes, entry)
        if not new_resolution or not reason:
            continue
        try:
            rel_log = log_path.relative_to(root)
        except ValueError:
            rel_log = log_path
        cli_command = entry.cli_command or "cargo run --bin reml_frontend -- --output json ..."
        expected_repr = ", ".join(expected_codes) if expected_codes else "(none)"
        actual_repr = ", ".join(entry.actual_codes) if entry.actual_codes else "(none)"
        note = (
            f"{timestamp} triage_spec_core_failures.py ({args.suite}) で {new_resolution} 判定: "
            f"{reason} / log={rel_log} / CLI=\"{cli_command}\" / 期待={expected_repr} / 実際={actual_repr}"
        )
        row["resolution"] = new_resolution
        row["resolution_notes"] = append_note(row.get("resolution_notes", ""), note)
        updated.append(scenario_id)

    if skipped_missing:
        missing_list = ", ".join(skipped_missing)
        print(f"警告: シナリオ ID が CSV に存在しません -> {missing_list}")

    if not updated:
        print("更新対象はありませんでした。")
        return

    summary = ", ".join(updated)
    if args.apply:
        with matrix_path.open("w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(rows)
        print(f"更新完了: {len(updated)} 件 -> {summary}")
    else:
        print(f"[dry-run] {len(updated)} 件を更新予定 -> {summary}")
        print("反映するには --apply を付与してください。")


if __name__ == "__main__":
    main()
